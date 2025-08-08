//! This module provides functionality for converting the WOMIR BlockLess DAG representation of a WASM module to MIR.

use crate::loader::{BlocklessDagModule, WasmLoadError};
use cairo_m_compiler_mir::{
    instruction::CalleeSignature, BasicBlock, BasicBlockId, BinaryOp, FunctionId, Instruction,
    MirFunction, MirModule, MirType, Terminator, Value, ValueId,
};
use std::collections::HashMap;
use thiserror::Error;
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::{BlocklessDag, BreakTarget, Node, Operation, TargetType};
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
    /// MIR function being built
    mir_function: MirFunction,
    /// Mapping from WASM ValueOrigin to MIR ValueId
    value_map: HashMap<ValueOrigin, ValueId>,
    /// Mapping from DAG label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
    /// Current basic block being filled
    current_block_id: Option<BasicBlockId>,
    /// Track the current loop header block for break targets
    current_loop_header: Option<BasicBlockId>,
    /// Local variable mapping for LocalGet/LocalSet
    local_map: HashMap<u32, ValueId>,
    /// Current source block for tracking control flow
    current_source_block: Option<BasicBlockId>,
    /// For each label id, a vector of slot ValueIds (one per label parameter)
    label_slots: HashMap<u32, Vec<ValueId>>,
    /// For each label id, the pre-allocated output ValueIds (one per label parameter)
    label_output_values: HashMap<u32, Vec<ValueId>>,
}

impl DagToMirContext {
    fn new(func_name: String) -> Self {
        let mir_function = MirFunction::new(func_name);

        Self {
            value_map: HashMap::new(),
            label_map: HashMap::new(),
            local_map: HashMap::new(),
            current_source_block: None,
            label_slots: HashMap::new(),
            label_output_values: HashMap::new(),
            mir_function,
            current_block_id: Some(0.into()),
            current_loop_header: None,
        }
    }

    fn get_current_block(&mut self) -> Result<&mut BasicBlock, DagToMirError> {
        let block_id = self.current_block_id.ok_or_else(|| {
            DagToMirError::InvalidControlFlow("No current block set - invalid state".to_string())
        })?;

        self.mir_function
            .basic_blocks
            .get_mut(block_id)
            .ok_or_else(|| {
                DagToMirError::InvalidControlFlow(format!("Block {:?} does not exist", block_id))
            })
    }

    const fn set_current_block(&mut self, block_id: BasicBlockId) {
        self.current_source_block = self.current_block_id;
        self.current_block_id = Some(block_id);
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

        // Get function type information for parameters and return types
        let (param_types, return_types) = self.module.with_program(|program| {
            let func_type = program.c.get_func_type(*func_idx);

            // Handle param types with proper error handling
            let param_types: Result<Vec<MirType>, DagToMirError> = func_type
                .ty
                .params()
                .iter()
                .map(Self::wasm_type_to_mir_type)
                .collect();

            // Handle return types with proper error handling
            let return_types: Result<Vec<MirType>, DagToMirError> = func_type
                .ty
                .results()
                .iter()
                .map(Self::wasm_type_to_mir_type)
                .collect();

            (param_types, return_types)
        });

        let param_types = param_types?;
        let return_types = return_types?;

        // Get the DAG for this function
        let result = self.module.with_program(|program| {
            let func = program.functions.get(func_idx).ok_or_else(|| {
                DagToMirError::ValueMappingError(format!("Function {} not found", func_idx))
            })?;

            // Preallocate all the blocks associated with DAG lebels
            self.allocate_labeled_blocks(func, &mut context)?;

            // Generate instructions and control flow
            self.generate_instructions_from_dag(func, &mut context)?;

            Ok::<(), DagToMirError>(())
        });

        result?;

        // Set entry block if we created any blocks
        if !context.mir_function.basic_blocks.is_empty() {
            context.mir_function.entry_block = 0.into();
        }

        // Populate parameter types
        for (i, &param_value_id) in context.mir_function.parameters.iter().enumerate() {
            if let Some(param_type) = param_types.get(i) {
                context
                    .mir_function
                    .value_types
                    .insert(param_value_id, param_type.clone());
            }
        }

        // Extract return values from return terminators and set their types
        let mut return_value_ids = Vec::new();
        for block in &context.mir_function.basic_blocks {
            if let Terminator::Return { values } = &block.terminator {
                for value in values {
                    if let Value::Operand(value_id) = value {
                        return_value_ids.push(*value_id);
                    }
                }
            }
        }

        // Set return values and their types
        if !return_value_ids.is_empty() {
            context.mir_function.return_values = return_value_ids.clone();
            for (i, &return_value_id) in return_value_ids.iter().enumerate() {
                if let Some(return_type) = return_types.get(i) {
                    context
                        .mir_function
                        .value_types
                        .insert(return_value_id, return_type.clone());
                }
            }
        }

        Ok(context.mir_function)
    }

    /// Pass 1: Preallocate all the blocks associated with DAG labels
    /// Allocate variables to block inputs
    fn allocate_labeled_blocks(
        &self,
        func: &BlocklessDag,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        for (node_idx, node) in func.nodes.iter().enumerate() {
            if let Operation::Label { id } = &node.operation {
                let block_id = context.mir_function.add_basic_block();
                context.label_map.insert(*id, block_id);
                let mut output_value_ids: Vec<ValueId> = Vec::new();
                let mut slot_value_ids: Vec<ValueId> = Vec::new();
                for (output_idx, output_type) in node.output_types.iter().enumerate() {
                    let mir_type = Self::wasm_type_to_mir_type(output_type)?;
                    // Pre-allocate the label output value id
                    let output_value_id = context.mir_function.new_typed_value_id(mir_type.clone());
                    context.value_map.insert(
                        ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        },
                        output_value_id,
                    );
                    output_value_ids.push(output_value_id);

                    // Allocate a dedicated slot for this label parameter
                    let slot_value_id = context.mir_function.new_typed_value_id(mir_type);
                    slot_value_ids.push(slot_value_id);
                }
                context.label_output_values.insert(*id, output_value_ids);
                context.label_slots.insert(*id, slot_value_ids);
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
                    // Handle function parameters or loop inputs
                    for (output_idx, _output_type) in node.output_types.iter().enumerate() {
                        let value_origin = ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        };

                        // If we're in a loop and this Input mapping already exists (from loop initialization),
                        // don't overwrite it - use the existing loop variable mapping
                        if context.current_loop_header.is_some()
                            && context.value_map.contains_key(&value_origin)
                        {
                            // Loop variable mapping already exists, skip creating a new one
                            continue;
                        }

                        let value_id = context.mir_function.new_typed_value_id(MirType::U32);

                        // Only add to function parameters if this is the main function inputs
                        // (not loop sub-DAG inputs)
                        if context.current_loop_header.is_none() {
                            context.mir_function.parameters.push(value_id); // Define as param
                        }

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
                    context.set_current_block(context.label_map.get(id).copied().unwrap());
                    // At label entry, load from the slots into the label outputs
                    let output_value_ids = context.label_output_values.get(id).cloned();
                    let slot_value_ids = context.label_slots.get(id).cloned();
                    if let (Some(output_value_ids), Some(slot_value_ids)) =
                        (output_value_ids, slot_value_ids)
                    {
                        let count = core::cmp::min(output_value_ids.len(), slot_value_ids.len());
                        for i in 0..count {
                            let dest_value_id = output_value_ids[i];
                            let slot_value_id = slot_value_ids[i];
                            let load_inst = Instruction::assign_u32(
                                dest_value_id,
                                Value::operand(slot_value_id),
                            );
                            context.get_current_block()?.push_instruction(load_inst);
                        }
                    }
                }

                Operation::Br(target) => {
                    // This is either a jump or a return
                    let target_block = self.resolve_break_target(node, target, context)?;

                    // If this is a branch to a label, copy the branch values to the label's outputs
                    if let TargetType::Label(label_id) = &target.kind {
                        self.copy_branch_values_to_label(node, *label_id, context)?;
                    }

                    let terminator = Terminator::jump(target_block);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(target_block);
                }

                Operation::BrIf(target) => {
                    // Conditional branch - the condition is typically the last input
                    let condition_value = self.get_input_value(&node.inputs[0], context)?;
                    let target_block = self.resolve_break_target(node, target, context)?;
                    let else_block = context.mir_function.add_basic_block();

                    // If this is a branch to a label, copy the branch values to the label's outputs
                    if let TargetType::Label(label_id) = &target.kind {
                        self.copy_branch_values_to_label(node, *label_id, context)?;
                    }

                    let terminator = Terminator::branch(condition_value, target_block, else_block);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(else_block);
                }

                Operation::BrIfZero(target) => {
                    // Inverted conditional branch
                    let condition_value = self.get_input_value(&node.inputs[0], context)?;
                    let else_target = self.resolve_break_target(node, target, context)?;
                    let then_target = context.mir_function.add_basic_block();

                    // If this is a branch to a label, copy the branch values to the label's outputs
                    if let TargetType::Label(label_id) = &target.kind {
                        self.copy_branch_values_to_label(node, *label_id, context)?;
                    }

                    let terminator = Terminator::branch(condition_value, then_target, else_target);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(then_target);
                }

                Operation::BrTable { targets: _ } => {
                    todo!()
                }

                Operation::Loop {
                    sub_dag: _,
                    break_targets: _,
                } => {
                    todo!()
                }
            }
        }

        Ok(())
    }

    /// Convert a WASM operation to MIR instructions
    fn convert_wasm_op_to_mir(
        &self,
        wasm_op: &Op,
        node: &Node,
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
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Add, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Sub => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Sub, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Mul => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Mul, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Comparison operations
            Op::I32Eq => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Eq, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Ne => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Neq, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32LtU => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Less, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32GtU => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Greater, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32LeU => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32LessEqual, result_id, inputs[0], inputs[1]);
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32GeU => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction = Instruction::binary_op(
                    BinaryOp::U32GreaterEqual,
                    result_id,
                    inputs[0],
                    inputs[1],
                );
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Constants
            Op::I32Const { value } => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction = Instruction::assign_u32(result_id, Value::integer(*value));
                context.get_current_block()?.push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Local variable operations
            Op::LocalGet { local_index } => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);

                // If we have a local variable stored, load its value
                if let Some(&local_value_id) = context.local_map.get(local_index) {
                    let instruction =
                        Instruction::assign_u32(result_id, Value::operand(local_value_id));
                    context.get_current_block()?.push_instruction(instruction);
                } else {
                    // No local variable set yet, create a placeholder (uninitialized local)
                    let instruction = Instruction::assign_u32(
                        result_id,
                        Value::integer(0), // Default to 0 for uninitialized locals
                    );
                    context.get_current_block()?.push_instruction(instruction);
                }
                Ok(vec![result_id])
            }

            Op::LocalSet { local_index } => {
                // Get the input value to store
                if !inputs.is_empty() {
                    let value_to_store = inputs[0];

                    // Create a new ValueId for this local variable
                    let local_value_id = context.mir_function.new_typed_value_id(MirType::U32);

                    // Assign the input value to our local variable
                    let instruction = Instruction::assign_u32(local_value_id, value_to_store);
                    context.get_current_block()?.push_instruction(instruction);

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
                        let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                        let instruction = Instruction::assign_u32(result_id, inputs[0]);
                        context.get_current_block()?.push_instruction(instruction);
                        Ok(vec![result_id])
                    }
                } else {
                    Ok(vec![])
                }
            }

            Op::Call { function_index } => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let callee_id = FunctionId::new(*function_index as usize);

                // Get signature from wasm module
                let signature = self.module.with_program(|program| {
                    let func_type = program.c.get_func_type(*function_index);

                    // Handle param types with proper error handling
                    let param_types: Result<Vec<MirType>, DagToMirError> = func_type
                        .ty
                        .params()
                        .iter()
                        .map(Self::wasm_type_to_mir_type)
                        .collect();

                    // Handle return types with proper error handling
                    let return_types: Result<Vec<MirType>, DagToMirError> = func_type
                        .ty
                        .results()
                        .iter()
                        .map(Self::wasm_type_to_mir_type)
                        .collect();

                    // Return both results
                    (param_types, return_types)
                });

                // Handle the errors from type conversion
                let (param_types, return_types) = signature;
                let param_types = param_types?;
                let return_types = return_types?;

                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                let instruction = Instruction::call(vec![result_id], callee_id, inputs, signature);
                context.get_current_block()?.push_instruction(instruction);
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
        node: &Node,
        target: &BreakTarget,
        context: &mut DagToMirContext,
    ) -> Result<BasicBlockId, DagToMirError> {
        match (&target.kind, target.depth) {
            (TargetType::Label(label_id), _depth) => {
                // Direct jump to a label at current scope
                context.label_map.get(label_id).map_or_else(
                    || {
                        Err(DagToMirError::InvalidControlFlow(format!(
                            "Label {} not found in label_map",
                            label_id
                        )))
                    },
                    |block_id| Ok(*block_id),
                )
            }

            (TargetType::FunctionOrLoop, 0) => {
                // We suppose this is a return for now
                // Allocate a new block containing only the return instruction
                let return_block = context.mir_function.add_basic_block();

                // TODO: fix returns
                let node_inputs = node.inputs.clone();
                let return_values: Result<Vec<Value>, DagToMirError> = node_inputs
                    .iter()
                    .map(|input| self.get_input_value(input, context))
                    .collect();

                let return_values = return_values?;
                let terminator = Terminator::return_values(return_values);
                context
                    .mir_function
                    .get_basic_block_mut(return_block)
                    .unwrap()
                    .set_terminator(terminator);
                Ok(return_block)
            }
            (_, depth) if depth > 1 => {
                // TODO: Implement proper nested scope handling for deeper nesting
                Err(DagToMirError::InvalidControlFlow(format!(
                    "Deeply nested break targets not yet supported (depth: {})",
                    depth
                )))
            }
            _ => Err(DagToMirError::InvalidControlFlow(format!(
                "Unsupported break target: {:?}",
                target
            ))),
        }
    }

    /// Copy branch values to label output variables when branching to a label
    fn copy_branch_values_to_label(
        &self,
        node: &Node,
        label_id: u32,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        // Determine which inputs represent data (exclude condition for conditional branches)
        let data_inputs_start_index = match &node.operation {
            Operation::BrIf(_) | Operation::BrIfZero(_) => 1,
            _ => 0,
        };

        // Get the values that should be copied to the label's slots
        let branch_values: Result<Vec<Value>, DagToMirError> = node
            .inputs
            .iter()
            .skip(data_inputs_start_index)
            .map(|input| self.get_input_value(input, context))
            .collect();
        let branch_values = branch_values?;

        // Copy into the label's dedicated slots
        let slot_value_ids = context.label_slots.get(&label_id).cloned().ok_or_else(|| {
            DagToMirError::InvalidControlFlow(format!("No slots allocated for label {}", label_id))
        })?;

        let count = core::cmp::min(branch_values.len(), slot_value_ids.len());
        for i in 0..count {
            let slot_value_id = slot_value_ids[i];
            let assign_instruction = Instruction::assign_u32(slot_value_id, branch_values[i]);
            context
                .get_current_block()?
                .push_instruction(assign_instruction);
        }

        Ok(())
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
