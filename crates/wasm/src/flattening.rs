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
    /// MIR function being built
    mir_function: MirFunction,
    /// Mapping from WASM ValueOrigin to MIR ValueId
    value_map: HashMap<ValueOrigin, ValueId>,
    /// Mapping from DAG label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
    /// Current basic block being filled
    current_block_id: Option<BasicBlockId>,
    /// Next BasicBlockId to assign
    next_block_id: usize,
    /// Next ValueId to assign
    next_value_id: usize,
    /// Track the current loop exit block
    current_loop_exit: Option<BasicBlockId>,
    /// Track the current loop header block for break targets
    current_loop_header: Option<BasicBlockId>,
    /// Local variable mapping for LocalGet/LocalSet
    local_map: HashMap<u32, ValueId>,
    /// Track label inputs for phi-like resolution
    label_inputs: HashMap<u32, Vec<(BasicBlockId, Vec<ValueId>)>>,
    /// Current source block for tracking control flow
    current_source_block: Option<BasicBlockId>,
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
            current_loop_exit: None,
            current_loop_header: None,
            next_block_id: 0,
        }
    }

    fn allocate_value_id(&mut self) -> ValueId {
        let id = ValueId::from_usize(self.next_value_id);
        self.next_value_id += 1;
        id
    }

    fn allocate_basic_block(&mut self) -> BasicBlockId {
        let id = BasicBlockId::from_usize(self.next_block_id);
        self.next_block_id += 1;
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

    /// Handle loop operation conversion to MIR
    fn handle_loop_operation(
        &self,
        sub_dag: &BlocklessDag,
        node: &womir::loader::blockless_dag::Node,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        // Setup loop structure and get input values
        let (loop_header_block, loop_exit_block, loop_input_values) =
            self.setup_loop_structure(node, context)?;

        // Save current context for nested loops
        let loop_context = self.save_loop_context(context, loop_header_block, loop_exit_block);

        // Save and setup loop variable mappings
        let saved_mappings = self.setup_loop_variables(&loop_input_values, context)?;

        // Process the loop body
        self.generate_instructions_from_dag(sub_dag, context)?;

        // Restore context and finalize loop
        self.restore_loop_context(context, loop_context);
        self.restore_variable_mappings(context, saved_mappings);
        self.finalize_loop(context, loop_exit_block)?;

        Ok(())
    }

    /// Setup loop structure: create header/exit blocks and process input values
    fn setup_loop_structure(
        &self,
        node: &womir::loader::blockless_dag::Node,
        context: &mut DagToMirContext,
    ) -> Result<(BasicBlockId, BasicBlockId, Vec<Value>), DagToMirError> {
        // Create a loop header block where the loop begins
        let loop_header_block = context.allocate_basic_block();

        // Process loop inputs - these are the initial values for loop-carried variables
        let mut loop_input_values = Vec::new();
        for input in &node.inputs {
            let input_value = self.get_input_value(input, context)?;
            loop_input_values.push(input_value);
        }

        // Jump from current block to loop header
        let terminator = Terminator::jump(loop_header_block);
        context.get_current_block().set_terminator(terminator);

        // Switch to loop header block
        context.set_current_block(loop_header_block);

        // Create an exit block for when the loop terminates
        let loop_exit_block = context.allocate_basic_block();

        Ok((loop_header_block, loop_exit_block, loop_input_values))
    }

    /// Save current loop context for nested loop support
    const fn save_loop_context(
        &self,
        context: &mut DagToMirContext,
        loop_header_block: BasicBlockId,
        loop_exit_block: BasicBlockId,
    ) -> (Option<BasicBlockId>, Option<BasicBlockId>) {
        let previous_loop_exit = context.current_loop_exit;
        let previous_loop_header = context.current_loop_header;

        context.current_loop_exit = Some(loop_exit_block);
        context.current_loop_header = Some(loop_header_block);

        (previous_loop_exit, previous_loop_header)
    }

    /// Setup loop variable mappings and save previous mappings
    fn setup_loop_variables(
        &self,
        loop_input_values: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Vec<(ValueOrigin, Option<ValueId>)>, DagToMirError> {
        // Save current value mappings for loop variables to restore later
        let mut saved_mappings = Vec::new();
        for idx in 0..loop_input_values.len() {
            let value_origin = ValueOrigin {
                node: 0, // Loop body Input node
                output_idx: idx as u32,
            };
            saved_mappings.push((value_origin, context.value_map.get(&value_origin).copied()));
        }

        // Set up initial loop variable mappings
        for (idx, input_value) in loop_input_values.iter().enumerate() {
            let loop_var_id = context.allocate_value_id();
            let instruction = Instruction::assign(loop_var_id, *input_value);
            context.get_current_block().push_instruction(instruction);

            // Map this to the loop body's Input node outputs
            let value_origin = ValueOrigin {
                node: 0, // The Input node in sub_dag is always node 0
                output_idx: idx as u32,
            };
            context.value_map.insert(value_origin, loop_var_id);
        }

        Ok(saved_mappings)
    }

    /// Restore previous loop context
    const fn restore_loop_context(
        &self,
        context: &mut DagToMirContext,
        loop_context: (Option<BasicBlockId>, Option<BasicBlockId>),
    ) {
        let (previous_loop_exit, previous_loop_header) = loop_context;
        context.current_loop_exit = previous_loop_exit;
        context.current_loop_header = previous_loop_header;
    }

    /// Restore saved variable mappings
    fn restore_variable_mappings(
        &self,
        context: &mut DagToMirContext,
        saved_mappings: Vec<(ValueOrigin, Option<ValueId>)>,
    ) {
        for (value_origin, saved_value) in saved_mappings {
            if let Some(saved) = saved_value {
                context.value_map.insert(value_origin, saved);
            } else {
                context.value_map.remove(&value_origin);
            }
        }
    }

    /// Finalize loop: ensure termination and set exit block
    fn finalize_loop(
        &self,
        context: &mut DagToMirContext,
        loop_exit_block: BasicBlockId,
    ) -> Result<(), DagToMirError> {
        // If the loop body didn't end with a terminator, jump to exit
        if let Some(current_block_id) = context.current_block_id {
            let current_block = &mut context.mir_function.basic_blocks[current_block_id];
            if !current_block.is_terminated() {
                current_block.set_terminator(Terminator::jump(loop_exit_block));
            }
        }

        // Continue execution from the loop exit block
        context.set_current_block(loop_exit_block);
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

                        let value_id = context.allocate_value_id();

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
                    // Conditional branch - the condition is typically the last input
                    let condition_input_idx = node.inputs.len().saturating_sub(1);
                    let condition_value =
                        self.get_input_value(&node.inputs[condition_input_idx], context)?;
                    let then_target = self.resolve_break_target(target, context)?;

                    // Handle loop back-edge: update loop variables with new values
                    if matches!(target.kind, TargetType::FunctionOrLoop) && target.depth == 0 {
                        // This is a loop back-edge - update loop variables
                        // The inputs (except the last one which is the condition) are the updated loop values
                        for (idx, input) in node.inputs.iter().enumerate() {
                            if idx < node.inputs.len() - 1 {
                                // Skip the condition (last input)
                                let updated_value = self.get_input_value(input, context)?;

                                // Update the loop variable mapping for the next iteration
                                // Map this to the loop body's Input node outputs
                                let value_origin = ValueOrigin {
                                    node: 0, // Loop body Input node
                                    output_idx: idx as u32,
                                };

                                // Create a new value to represent the updated loop variable
                                let updated_var_id = context.allocate_value_id();
                                let instruction =
                                    Instruction::assign(updated_var_id, updated_value);
                                context.get_current_block().push_instruction(instruction);

                                // Update the mapping so the next iteration uses the updated value
                                context.value_map.insert(value_origin, updated_var_id);
                            }
                        }
                    }

                    // Create else target (fallthrough block)
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
                    sub_dag,
                    break_targets: _,
                } => {
                    self.handle_loop_operation(sub_dag, node, context)?;
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
                let instruction = Instruction::assign_u32(result_id, Value::integer(*value));
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
                // Direct jump to a label at current scope
                context.label_map.get(label_id).copied().ok_or_else(|| {
                    DagToMirError::InvalidControlFlow(format!("Label {} not found", label_id))
                })
            }
            (TargetType::Label(label_id), 1) => {
                // Break out of current loop and go to outer label
                // This is typically the loop exit - for now, map to the label directly
                // In a more complete implementation, we'd need to track scope depth
                context.label_map.get(label_id).copied().ok_or_else(|| {
                    DagToMirError::InvalidControlFlow(format!("Outer label {} not found", label_id))
                })
            }
            (TargetType::FunctionOrLoop, 0) => {
                // This is a loop back-edge - jump to current loop header
                context.current_loop_header.ok_or_else(|| {
                    DagToMirError::InvalidControlFlow(
                        "Loop break target found but no current loop header".to_string(),
                    )
                })
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
