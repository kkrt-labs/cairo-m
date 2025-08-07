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
pub enum WasmModuleToMirError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
    #[error("Unsupported WASM operation: {0:?}")]
    UnsupportedOperation(String),
    #[error("Invalid control flow: {0}")]
    InvalidControlFlow(String),
    #[error("Value mapping error: {0}")]
    ValueMappingError(String),
}

pub struct WasmModuleToMir {
    module: BlocklessDagModule,
}

/// Context for converting a single DAG to MIR
struct DagToMirContext {
    /// Map from ValueOrigin (WASM DAG values) to MIR ValueId
    value_map: HashMap<ValueOrigin, ValueId>,
    /// Map from WASM label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
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
        self.current_block_id = Some(block_id);
    }
}

impl WasmModuleToMir {
    pub const fn new(module: BlocklessDagModule) -> Self {
        Self { module }
    }

    /// Convert a single WASM function to MIR using two-pass algorithm
    fn function_to_mir(&self, func_idx: &u32) -> Result<MirFunction, WasmModuleToMirError> {
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
                WasmModuleToMirError::ValueMappingError(format!("Function {} not found", func_idx))
            })?;

            // Two-pass algorithm inspired by blockless DAG to LLVM guide

            // Pass 1: Create all basic blocks for labels
            self.create_basic_blocks_for_labels(func, &mut context)?;

            // Pass 2: Generate instructions and control flow
            self.generate_instructions_from_dag(func, &mut context)?;

            Ok::<(), WasmModuleToMirError>(())
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
    ) -> Result<(), WasmModuleToMirError> {
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
    ) -> Result<(), WasmModuleToMirError> {
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
    ) -> Result<Vec<ValueId>, WasmModuleToMirError> {
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
                    Instruction::binary_op(BinaryOp::Add, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Sub => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::Sub, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Mul => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::Mul, result_id, inputs[0], inputs[1]);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            // Comparison operations
            Op::I32Eq => {
                let result_id = context.allocate_value_id();
                let instruction =
                    Instruction::binary_op(BinaryOp::Eq, result_id, inputs[0], inputs[1]);
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
                // TODO: Implement memory load operations
                let result_id = context.allocate_value_id();
                let instruction = Instruction::assign(
                    result_id,
                    Value::integer(0), // Placeholder
                );
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::I32Store { .. } => {
                // TODO: Implement memory store operations
                // Store operations typically don't produce values
                Ok(vec![])
            }

            // Local variable operations
            Op::LocalGet { local_index } => {
                // TODO: Map local variables properly
                // For now, create a placeholder value
                let result_id = context.allocate_value_id();
                let instruction = Instruction::assign(
                    result_id,
                    Value::integer(*local_index as i32), // Placeholder
                );
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            Op::LocalSet { .. } => {
                // TODO: Implement local variable assignment
                Ok(vec![])
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

            // Function calls - TODO: Implement when MIR has call support
            Op::Call { function_index } => {
                // TODO: Implement function calls
                let result_id = context.allocate_value_id();
                let callee_id = FunctionId::new(*function_index as usize);

                // TODO: Get signature from wasm module
                let signature = CalleeSignature {
                    param_types: vec![MirType::U32, MirType::U32],
                    return_types: vec![MirType::U32],
                };

                let instruction = Instruction::call(vec![result_id], callee_id, inputs, signature);
                context.get_current_block().push_instruction(instruction);
                Ok(vec![result_id])
            }

            _ => {
                // Unsupported operation
                Err(WasmModuleToMirError::UnsupportedOperation(format!(
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
    ) -> Result<Value, WasmModuleToMirError> {
        if let Some(&value_id) = context.value_map.get(value_origin) {
            Ok(Value::operand(value_id))
        } else {
            Err(WasmModuleToMirError::ValueMappingError(format!(
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
    ) -> Result<BasicBlockId, WasmModuleToMirError> {
        match (&target.kind, target.depth) {
            (TargetType::Label(label_id), 0) => {
                context.label_map.get(label_id).copied().ok_or_else(|| {
                    WasmModuleToMirError::InvalidControlFlow(format!(
                        "Label {} not found",
                        label_id
                    ))
                })
            }
            (TargetType::FunctionOrLoop, 0) => {
                // This should be handled as a Return terminator, not a block jump
                Err(WasmModuleToMirError::InvalidControlFlow(
                    "Return target should be handled directly in Br/BrIf".to_string(),
                ))
            }
            (_, depth) if depth > 0 => {
                // TODO: Implement proper nested scope handling
                Err(WasmModuleToMirError::InvalidControlFlow(format!(
                    "Nested break targets not yet supported: depth {}",
                    depth
                )))
            }
            _ => Err(WasmModuleToMirError::InvalidControlFlow(format!(
                "Unsupported break target: {:?}",
                target
            ))),
        }
    }

    pub fn to_mir(&self) -> Result<MirModule, WasmModuleToMirError> {
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
            Ok::<(), WasmModuleToMirError>(())
        })?;
        Ok(mir_module)
    }
}
