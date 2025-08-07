//! This module provides functionality for converting the WOMIR BlockLess DAG representation of a WASM module to MIR.

use crate::loader::{BlocklessDagModule, WasmLoadError};
use cairo_m_compiler_mir::{
    instruction::CalleeSignature, BasicBlock, BasicBlockId, BinaryOp, FunctionId, Instruction,
    MirFunction, MirModule, MirType, Terminator, Value, ValueId,
};
use std::collections::HashMap;
use thiserror::Error;
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::{BlocklessDag, BreakTarget, Operation, TargetType};
use womir::loader::dag::ValueOrigin;

#[derive(Error, Debug)]
pub enum DagToMirError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
    #[error("Unsupported WASM operation: {0:?}")]
    UnsupportedOperation(String),
    #[error("Invalid control flow: {0}")]
    InvalidControlFlow(String),
    #[error("Value mapping error: {0}")]
    ValueMappingError(String),
    #[error("Unsupported WASM type: {0:?}")]
    UnsupportedWasmType(wasmparser::ValType),
}

pub struct DagToMir {
    module: BlocklessDagModule,
}

/// Context for converting a single DAG to MIR
struct DagToMirContext {
    /// Map from ValueOrigin (WASM DAG values) to MIR ValueId
    value_map: HashMap<ValueOrigin, ValueId>,
    /// Map from WASM label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
    /// Map from WASM local variable indices to MIR ValueId
    local_map: HashMap<u32, ValueId>,
    /// Track values flowing into labels: label_id -> Vec<(source_block, values)>
    label_inputs: HashMap<u32, Vec<(BasicBlockId, Vec<ValueId>)>>,
    /// Track which block we're coming from when jumping
    current_source_block: Option<BasicBlockId>,
    /// The MIR function being built
    mir_function: MirFunction,
    /// Current basic block being constructed
    current_block_id: Option<BasicBlockId>,
    /// Next ValueId to assign
    next_value_id: usize,
}

impl DagToMirContext {
    fn new(func_name: String) -> Self {
        let mut mir_function = MirFunction::new(func_name);
        // Create the entry block immediately
        mir_function.entry_block = 0.into();

        Self {
            value_map: HashMap::new(),
            label_map: HashMap::new(),
            local_map: HashMap::new(),
            label_inputs: HashMap::new(),
            current_source_block: None,
            mir_function,
            current_block_id: Some(0.into()),
            next_value_id: 0,
        }
    }

    fn allocate_value_id(&mut self) -> ValueId {
        let id = ValueId::from_usize(self.next_value_id);
        self.next_value_id += 1;
        id
    }

    fn allocate_basic_block(&mut self) -> BasicBlockId {
        let id = BasicBlockId::from_usize(self.mir_function.basic_blocks.len());
        self.mir_function.basic_blocks.push(BasicBlock::new());
        id
    }

    fn get_current_block(&mut self) -> &mut BasicBlock {
        let block_id = self.current_block_id.expect("No current block set");
        &mut self.mir_function.basic_blocks[block_id]
    }

    const fn set_current_block(&mut self, block_id: BasicBlockId) {
        self.current_source_block = self.current_block_id;
        self.current_block_id = Some(block_id);
    }

    /// Record values flowing into a label from a branch
    fn record_label_input(&mut self, label_id: u32, values: Vec<ValueId>) {
        if let Some(source_block) = self.current_source_block {
            self.label_inputs
                .entry(label_id)
                .or_default()
                .push((source_block, values));
        }
    }

    /// Resolve the actual value for a label output by implementing simple phi logic
    fn resolve_label_value(&self, label_id: u32, output_idx: usize) -> Option<ValueId> {
        if let Some(inputs) = self.label_inputs.get(&label_id) {
            // For now, take the last recorded value (most recent branch)
            // In a full SSA implementation, this would be a proper phi node
            for (_source_block, values) in inputs.iter().rev() {
                if output_idx < values.len() {
                    return Some(values[output_idx]);
                }
            }
        }
        None
    }
}

impl DagToMir {
    pub const fn new(module: BlocklessDagModule) -> Self {
        Self { module }
    }

    /// Convert WASM type to MIR type
    /// For now, we only support i32
    const fn wasm_type_to_mir_type(
        wasm_type: &wasmparser::ValType,
    ) -> Result<MirType, DagToMirError> {
        match wasm_type {
            wasmparser::ValType::I32 => Ok(MirType::U32),
            _ => Err(DagToMirError::UnsupportedWasmType(*wasm_type)),
        }
    }

    /// Convert a single WASM function to MIR using two-pass algorithm
    fn function_to_mir(&self, func_idx: &u32) -> Result<MirFunction, DagToMirError> {
        let func_name = self.module.with_program(|program| {
            program
                .c
                .exported_functions
                .get(func_idx)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("func_{}", func_idx))
        });

        let mut context = DagToMirContext::new(func_name);

        // Get the DAG for this function
        let result = self.module.with_program(|program| {
            let func = program.functions.get(func_idx).ok_or_else(|| {
                DagToMirError::ValueMappingError(format!("Function {} not found", func_idx))
            })?;

            // Two-pass algorithm inspired by blockless DAG to LLVM guide

            // Pass 1: Create all basic blocks for labels
            self.create_basic_blocks_for_labels(func, &mut context)?;

            // Pass 2: Generate instructions and control flow
            self.generate_instructions_from_dag(func, &mut context)?;

            Ok::<(), DagToMirError>(())
        });

        result?;

        // Set entry block if we created any blocks
        if !context.mir_function.basic_blocks.is_empty() {
            context.mir_function.entry_block = 0.into();
        }

        Ok(context.mir_function)
    }

    /// Pass 1: Create basic blocks for all labels in the DAG
    fn create_basic_blocks_for_labels(
        &self,
        dag: &BlocklessDag,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        // Entry block is already created in new()
        for node in &dag.nodes {
            match &node.operation {
                Operation::Label { id } => {
                    let block_id = context.allocate_basic_block();
                    context.label_map.insert(*id, block_id);
                }
                Operation::Loop { .. } => {
                    // TODO: Recursively create blocks for loop sub-DAG
                    let _loop_block = context.allocate_basic_block();
                }
                _ => {} // Other operations don't create new blocks
            }
        }
        Ok(())
    }

    /// Pass 2: Generate MIR instructions from DAG nodes
    fn generate_instructions_from_dag(
        &self,
        dag: &BlocklessDag,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        for (node_idx, node) in dag.nodes.iter().enumerate() {
            match &node.operation {
                Operation::Inputs => {
                    // Handle function parameters by adding them to the MIR function's params
                    for (output_idx, _output_type) in node.output_types.iter().enumerate() {
                        let value_id = context.allocate_value_id();
                        context.mir_function.parameters.push(value_id); // Define as param
                        let value_origin = ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        };
                        context.value_map.insert(value_origin, value_id);
                    }
                }

                Operation::WASMOp(wasm_op) => {
                    // Convert WASM operation to MIR instruction
                    let mir_values = self.convert_wasm_op_to_mir(wasm_op, node, context)?;

                    // Map output values
                    for (output_idx, mir_value_id) in mir_values.iter().enumerate() {
                        let value_origin = ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        };
                        context.value_map.insert(value_origin, *mir_value_id);
                    }
                }

                Operation::Label { id } => {
                    let block_id = context.label_map[id];
                    // Link the previous block to this new one with a jump
                    if let Some(current_block) = context
                        .current_block_id
                        .map(|id| &mut context.mir_function.basic_blocks[id])
                    {
                        if !current_block.is_terminated() {
                            current_block.set_terminator(Terminator::jump(block_id));
                        }
                    }
                    context.set_current_block(block_id);

                    // Handle label inputs - these are values flowing into this label from branches
                    for (output_idx, _output_type) in node.output_types.iter().enumerate() {
                        let value_id = context.allocate_value_id();
                        let value_origin = ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        };
                        context.value_map.insert(value_origin, value_id);

                        // Try to resolve the actual value flowing into this label
                        if let Some(actual_value_id) = context.resolve_label_value(*id, output_idx)
                        {
                            // Use the actual value from the incoming branch
                            let instruction =
                                Instruction::assign(value_id, Value::operand(actual_value_id));
                            context.get_current_block().push_instruction(instruction);
                        } else {
                            // Fallback to placeholder if no value is recorded
                            let instruction = Instruction::assign(value_id, Value::integer(0));
                            context.get_current_block().push_instruction(instruction);
                        }
                    }
                }

                Operation::Br(target) => {
                    // This is either a jump or a return
                    if (&target.kind, target.depth) == (&TargetType::FunctionOrLoop, 0) {
                        // This is a function return
                        let return_values = node
                            .inputs
                            .iter()
                            .map(|vo| self.get_input_value(vo, context))
                            .collect::<Result<Vec<_>, _>>()?;
                        context
                            .get_current_block()
                            .set_terminator(Terminator::Return {
                                values: return_values,
                            });
                    } else {
                        // This is a jump to another block
                        let target_block = self.resolve_break_target(target, context)?;

                        // Record values being passed to the target label
                        if let TargetType::Label(label_id) = target.kind {
                            let input_values: Result<Vec<Value>, _> = node
                                .inputs
                                .iter()
                                .map(|vo| self.get_input_value(vo, context))
                                .collect();
                            let input_values = input_values?;

                            // Convert Values to ValueIds for tracking
                            let mut value_ids = Vec::new();
                            for value in input_values {
                                match value {
                                    Value::Operand(vid) => value_ids.push(vid),
                                    Value::Literal(_) => {
                                        // Create a temporary value for literals
                                        let temp_id = context.allocate_value_id();
                                        let instruction = Instruction::assign(temp_id, value);
                                        context.get_current_block().push_instruction(instruction);
                                        value_ids.push(temp_id);
                                    }
                                    Value::Error => {
                                        // Skip error values
                                    }
                                }
                            }

                            context.record_label_input(label_id, value_ids);
                        }

                        let terminator = Terminator::jump(target_block);
                        context.get_current_block().set_terminator(terminator);
                    }
                }

                Operation::BrIf(target) => {
                    // Conditional branch
                    let condition_value = self.get_input_value(&node.inputs[0], context)?;
                    let then_target = self.resolve_break_target(target, context)?;

                    // TODO: Create else target (next block or explicit target)
                    let else_target = context.allocate_basic_block();

                    let terminator = Terminator::branch(condition_value, then_target, else_target);
                    context.get_current_block().set_terminator(terminator);
                    context.set_current_block(else_target);
                }

                Operation::BrIfZero(target) => {
                    // Inverted conditional branch
                    let condition_value = self.get_input_value(&node.inputs[0], context)?;
                    let else_target = self.resolve_break_target(target, context)?;

                    // TODO: Create then target (next block)
                    let then_target = context.allocate_basic_block();

                    let terminator = Terminator::branch(condition_value, then_target, else_target);
                    context.get_current_block().set_terminator(terminator);
                    context.set_current_block(then_target);
                }

                Operation::BrTable { targets } => {
                    // Switch statement - convert to chain of conditional branches
                    // TODO: Implement proper switch handling
                    // For now, just branch to first target as placeholder
                    if let Some(first_target) = targets.first() {
                        let target_block =
                            self.resolve_break_target(&first_target.target, context)?;
                        let terminator = Terminator::jump(target_block);
                        context.get_current_block().set_terminator(terminator);
                    }
                }

                Operation::Loop {
                    sub_dag: _,
                    break_targets: _,
                } => {
                    // TODO: Implement loop handling
                    // This requires creating a separate context for the loop body
                    // and handling loop-carried values through phi-like constructs

                    // For now, create a placeholder jump
                    let loop_block = context.allocate_basic_block();
                    let terminator = Terminator::jump(loop_block);
                    context.get_current_block().set_terminator(terminator);
                    context.set_current_block(loop_block);
                }
            }
        }

        // Ensure the last block has a terminator
        if let Some(current_block_id) = context.current_block_id {
            let current_block = &mut context.mir_function.basic_blocks[current_block_id];
            if !current_block.is_terminated() {
                // Add a return terminator as fallback
                current_block.set_terminator(Terminator::Return { values: vec![] });
            }
        }

        Ok(())
    }

    /// Convert a WASM operation to MIR instructions
    fn convert_wasm_op_to_mir(
        &self,
        wasm_op: &Op,
        node: &womir::loader::blockless_dag::Node,
        context: &mut DagToMirContext,
    ) -> Result<Vec<ValueId>, DagToMirError> {
        let inputs: Result<Vec<Value>, _> = node
            .inputs
            .iter()
            .map(|input| self.get_input_value(input, context))
            .collect();
        let inputs = inputs?;

        match wasm_op {
            // Arithmetic operations
            Op::I32Add => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Add, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Sub => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Sub, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Mul => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Mul, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Comparison operations
            Op::I32Eq => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Eq, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Ne => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Neq, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32LtU => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Less, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32GtU => {
                let result_id = context.allocate_value_id();
                let instruction = Instruction::binary_op(
                    BinaryOp::U32GreaterEqual,
                    result_id,
                    inputs[0],
                    inputs[1],
                );
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32LeU => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::U32LessEqual, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32GeU => {
                let result_id = context.allocate_value_id();
                let instruction = Instruction::binary_op(
                    BinaryOp::U32GreaterEqual,
                    result_id,
                    inputs[0],
                    inputs[1],
                );
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Constants
            Op::I32Const { value } => {
                let result_id = context.allocate_value_id();
                let instruction = Instruction::assign(result_id, Value::integer(*value));
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Memory operations - TODO: Implement when MIR has memory support
            Op::I32Load { .. } => {
                todo!()
            }

            Op::I32Store { .. } => {
                todo!()
            }

            // Local variable operations
            Op::LocalGet { local_index } => {
                let result_id = context.allocate_value_id();

                // If we have a local variable stored, load its value
                if let Some(&local_value_id) = context.local_map.get(local_index) {
                    let instruction =
                        Instruction::assign(result_id, Value::operand(local_value_id));
                    context.get_current_block().push_instruction(instruction);
                } else {
                    // No local variable set yet, create a placeholder (uninitialized local)
                    let instruction = Instruction::assign(
                        result_id,
                        Value::integer(0), // Default to 0 for uninitialized locals
                    );
                    context.get_current_block().push_instruction(instruction);
                }
                Ok(vec![result_id])
            }

            Op::LocalSet { local_index } => {
                // Get the input value to store
                if !inputs.is_empty() {
                    let value_to_store = inputs[0];

                    // Create a new ValueId for this local variable
                    let local_value_id = context.allocate_value_id();

                    // Assign the input value to our local variable
                    let instruction = Instruction::assign(local_value_id, value_to_store);
                    context.get_current_block().push_instruction(instruction);

                    // Store the mapping from local index to ValueId
                    context.local_map.insert(*local_index, local_value_id);
                }
                Ok(vec![]) // LocalSet doesn't produce any output values
            }

            Op::LocalTee { .. } => {
                // TODO: Implement local tee (set and return value)
                if !inputs.is_empty() {
                    // Return the input value for now
                    if let Value::Operand(value_id) = inputs[0] {
                        Ok(vec![value_id])
                    } else {
                        // Create a new value for the literal
                        let result_id = context.allocate_value_id();
                        let instruction = Instruction::assign(result_id, inputs[0]);
                        context.get_current_block().push_instruction(instruction);
                        Ok(vec![result_id])
                    }
                } else {
                    Ok(vec![])
                }
            }

            Op::Call { function_index } => {
                let result_id = context.allocate_value_id();
                let callee_id = FunctionId::new(*function_index as usize);

                // Get signature from wasm module
                let signature = self.module.with_program(|program| {
                    let func_type = program.c.get_func_type(*function_index);
                    CalleeSignature {
                        param_types: func_type
                            .ty
                            .params()
                            .iter()
                            .map(|t| Self::wasm_type_to_mir_type(t).unwrap())
                            .collect(),
                        return_types: func_type
                            .ty
                            .results()
                            .iter()
                            .map(|t| Self::wasm_type_to_mir_type(t).unwrap())
                            .collect(),
                    }
                });

                let instruction = Instruction::call(vec![result_id], callee_id, inputs, signature);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            _ => {
                // Unsupported operation
                Err(DagToMirError::UnsupportedOperation(format!(
                    "{:?}",
                    wasm_op
                )))
            }
        }
    }

    /// Get MIR value for a WASM ValueOrigin
    fn get_input_value(
        &self,
        value_origin: &ValueOrigin,
        context: &DagToMirContext,
    ) -> Result<Value, DagToMirError> {
        if let Some(&value_id) = context.value_map.get(value_origin) {
            Ok(Value::operand(value_id))
        } else {
            Err(DagToMirError::ValueMappingError(format!(
                "Value not found: node {}, output {}",
                value_origin.node, value_origin.output_idx
            )))
        }
    }

    /// Resolve a WASM break target to a MIR BasicBlockId
    fn resolve_break_target(
        &self,
        target: &BreakTarget,
        context: &DagToMirContext,
    ) -> Result<BasicBlockId, DagToMirError> {
        match (&target.kind, target.depth) {
            (TargetType::Label(label_id), 0) => {
                context.label_map.get(label_id).copied().ok_or_else(|| {
                    DagToMirError::InvalidControlFlow(format!("Label {} not found", label_id))
                })
            }
            (TargetType::FunctionOrLoop, 0) => {
                // This should be handled as a Return terminator, not a block jump
                Err(DagToMirError::InvalidControlFlow(
                    "Return target should be handled directly in Br/BrIf".to_string(),
                ))
            }
            (_, depth) if depth > 0 => {
                // TODO: Implement proper nested scope handling
                Err(DagToMirError::InvalidControlFlow(format!(
                    "Nested break targets not yet supported: depth {}",
                    depth
                )))
            }
            _ => Err(DagToMirError::InvalidControlFlow(format!(
                "Unsupported break target: {:?}",
                target
            ))),
        }
    }

    /// Convert the DAG representation of the module to MIR
    pub fn to_mir(&self) -> Result<MirModule, DagToMirError> {
        let mut mir_module = MirModule::new();
        self.module.with_program(|program| {
            for (func_idx, _) in program.functions.iter() {
                let function_id = FunctionId::new(*func_idx as usize);
                let mir_function = self.function_to_mir(func_idx)?;
                mir_module
                    .function_names
                    .insert(mir_function.name.clone(), function_id);
                mir_module.functions.insert(function_id, mir_function);
            }
            Ok::<(), DagToMirError>(())
        })?;
        Ok(mir_module)
    }
}
