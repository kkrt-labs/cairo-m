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
    /// Stack of value maps to scope ValueOrigin -> ValueId per DAG (avoids collisions)
    value_maps: Vec<HashMap<ValueOrigin, ValueId>>,
    /// Mapping from DAG label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
    /// Current basic block being filled
    current_block_id: Option<BasicBlockId>,
    /// Track the current loop body block for break targets
    current_loop_body: Option<BasicBlockId>,
    /// Local variable mapping for LocalGet/LocalSet
    local_map: HashMap<u32, ValueId>,
    /// Current source block for tracking control flow
    current_source_block: Option<BasicBlockId>,
    /// For each label id, the pre-allocated output ValueIds (one per label parameter)
    label_output_values: HashMap<u32, Vec<ValueId>>,
    /// For each loop, the pre-allocated header slot ValueIds (one per loop parameter)
    loop_header_slots: HashMap<u32, Vec<ValueId>>,
    /// Stack of active loops to support continues and loop-carried variables
    loop_stack: Vec<ActiveLoop>,
}

/// Information about an active loop during lowering
struct ActiveLoop {
    /// Body basic block for this loop
    body_block: BasicBlockId,
    /// Canonical storages (slots) for loop-carried values (header parameters)
    header_slots: Vec<ValueId>,
}

impl DagToMirContext {
    fn new(func_name: String) -> Self {
        let mir_function = MirFunction::new(func_name);

        Self {
            value_maps: vec![HashMap::new()],
            label_map: HashMap::new(),
            local_map: HashMap::new(),
            current_source_block: None,
            label_output_values: HashMap::new(),
            loop_header_slots: HashMap::new(),
            mir_function,
            current_block_id: Some(0.into()),
            current_loop_body: None,
            loop_stack: Vec::new(),
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

    fn insert_value(&mut self, origin: ValueOrigin, value_id: ValueId) {
        if let Some(map) = self.value_maps.last_mut() {
            map.insert(origin, value_id);
        }
    }

    fn get_value(&self, origin: &ValueOrigin) -> Option<ValueId> {
        for map in self.value_maps.iter().rev() {
            if let Some(v) = map.get(origin) {
                return Some(*v);
            }
        }
        None
    }

    fn push_scope(&mut self) {
        self.value_maps.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.value_maps.pop();
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

        // Allocate parameters
        for (i, param_type) in param_types.iter().enumerate() {
            let param_id = context.mir_function.new_typed_value_id(param_type.clone());
            context.mir_function.parameters.push(param_id);
            context.insert_value(
                ValueOrigin {
                    node: 0,
                    output_idx: i as u32,
                },
                param_id,
            );
        }

        // Get the DAG for this function
        let result = self.module.with_program(|program| {
            let func = program.functions.get(func_idx).ok_or_else(|| {
                DagToMirError::ValueMappingError(format!("Function {} not found", func_idx))
            })?;

            // Preallocate all the blocks associated with DAG labels and loops
            self.allocate_blocks_and_slots(func, &mut context)?;

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

        // Define return values from the function signature (types/arity only).
        // The actual values returned are supplied by each Return terminator.
        context.mir_function.return_values = return_types
            .iter()
            .map(|ty| context.mir_function.new_typed_value_id(ty.clone()))
            .collect();

        Ok(context.mir_function)
    }

    /// Pass 1: Preallocate all the blocks associated with DAG labels and loops
    /// Allocate variables to block inputs and loop header slots
    fn allocate_blocks_and_slots(
        &self,
        func: &BlocklessDag,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        for (node_idx, node) in func.nodes.iter().enumerate() {
            match &node.operation {
                Operation::Label { id } => {
                    let block_id = context.mir_function.add_basic_block();
                    context.label_map.insert(*id, block_id);
                    let mut output_value_ids: Vec<ValueId> = Vec::new();
                    for (output_idx, output_type) in node.output_types.iter().enumerate() {
                        let mir_type = Self::wasm_type_to_mir_type(output_type)?;
                        // Pre-allocate the label output value id
                        let output_value_id =
                            context.mir_function.new_typed_value_id(mir_type.clone());
                        context.insert_value(
                            ValueOrigin {
                                node: node_idx,
                                output_idx: output_idx as u32,
                            },
                            output_value_id,
                        );
                        output_value_ids.push(output_value_id);

                        // Allocate a dedicated slot for this label parameter
                        context.mir_function.new_typed_value_id(mir_type);
                    }
                    context.label_output_values.insert(*id, output_value_ids);
                }
                Operation::Loop { sub_dag, .. } => {
                    // Pre-allocate loop header slots from the sub-DAG's Inputs node
                    let sub_inputs_idx = 0;
                    let input_node = &sub_dag.nodes[sub_inputs_idx];
                    assert!(
                        matches!(input_node.operation, Operation::Inputs),
                        "Loop sub-DAG must start with Inputs node"
                    );

                    let mut header_slots = Vec::new();
                    for output_type in &input_node.output_types {
                        let mir_type = Self::wasm_type_to_mir_type(output_type)?;
                        let slot_id = context.mir_function.new_typed_value_id(mir_type);
                        header_slots.push(slot_id);
                    }

                    // Use node_idx as the loop identifier
                    context
                        .loop_header_slots
                        .insert(node_idx as u32, header_slots);
                }
                _ => {}
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
                Operation::Inputs => {}

                Operation::WASMOp(wasm_op) => {
                    // Convert WASM operation to MIR instruction
                    let mir_values = self.convert_wasm_op_to_mir(wasm_op, node, context)?;

                    // Map output values
                    for (output_idx, mir_value_id) in mir_values.iter().enumerate() {
                        let value_origin = ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        };
                        context.insert_value(value_origin, *mir_value_id);
                    }
                }

                Operation::Label { id } => {
                    context.set_current_block(context.label_map.get(id).copied().unwrap());
                }

                Operation::Br(target) => {
                    // This is either a jump or a return
                    let target_block = self.resolve_break_target(node, target, context)?;

                    // Edge copies
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.copy_branch_values_to_label(node, *label_id, context)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.copy_branch_values_to_loop(node, target.depth, context)?;
                        }
                    }

                    let terminator = Terminator::jump(target_block);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(target_block);
                }

                Operation::BrIf(target) => {
                    // Conditional branch - in our DAG, the condition is the last input
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToMirError::InvalidControlFlow(
                            "BrIf without condition input".to_string(),
                        )
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx], context)?;
                    let target_block = self.resolve_break_target(node, target, context)?;
                    let else_block = context.mir_function.add_basic_block();

                    // Edge copies on the taken edge
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.copy_branch_values_to_label(node, *label_id, context)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.copy_branch_values_to_loop(node, target.depth, context)?;
                        }
                    }

                    let terminator = Terminator::branch(condition_value, target_block, else_block);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(else_block);
                }

                Operation::BrIfZero(target) => {
                    // Inverted conditional branch - condition is the last input
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToMirError::InvalidControlFlow(
                            "BrIfZero without condition input".to_string(),
                        )
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx], context)?;
                    let else_target = self.resolve_break_target(node, target, context)?;
                    let then_target = context.mir_function.add_basic_block();

                    // Edge copies on the taken edge
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.copy_branch_values_to_label(node, *label_id, context)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.copy_branch_values_to_loop(node, target.depth, context)?;
                        }
                    }

                    let terminator = Terminator::branch(condition_value, then_target, else_target);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(then_target);
                }

                Operation::BrTable { targets: _ } => {
                    todo!()
                }

                Operation::Loop {
                    sub_dag,
                    break_targets: _,
                } => {
                    // Build a normal loop (header/body/exit) from the sub-DAG
                    // Create header block and get pre-allocated header slots
                    let header_block = context.mir_function.add_basic_block();
                    let header_slots = context
                        .loop_header_slots
                        .get(&(node_idx as u32))
                        .cloned()
                        .ok_or_else(|| {
                        DagToMirError::InvalidControlFlow(
                            "Loop header slots not pre-allocated".to_string(),
                        )
                    })?;

                    let terminator = Terminator::jump(header_block);
                    context.get_current_block()?.set_terminator(terminator);

                    // Enter header (no materialization needed since we use slots directly)
                    context.set_current_block(header_block);

                    // Copy the loop inputs into header slots
                    for (input_idx, input) in node.inputs.iter().enumerate() {
                        let slot_value_id = header_slots[input_idx];
                        let source_value_id = context.get_value(input).unwrap();
                        let assign_instruction =
                            Instruction::assign_u32(slot_value_id, Value::operand(source_value_id));
                        context
                            .get_current_block()?
                            .push_instruction(assign_instruction);
                    }

                    // Allocate a new block for the loop body
                    let body_block = context.mir_function.add_basic_block();
                    let terminator = Terminator::jump(body_block);
                    context.get_current_block()?.set_terminator(terminator);
                    context.set_current_block(body_block);

                    // Push loop on the stack and lower the body sub-DAG
                    let prev_loop_body = context.current_loop_body;
                    context.current_loop_body = Some(body_block);
                    context.loop_stack.push(ActiveLoop {
                        body_block,
                        header_slots: header_slots.clone(),
                    });

                    // Enter a new value scope for the loop body to avoid ValueOrigin collisions
                    context.push_scope();

                    // Map the sub-DAG's Inputs node (node 0) to header slots
                    for (output_idx, slot_value_id) in header_slots.iter().enumerate() {
                        context.insert_value(
                            ValueOrigin {
                                node: 0,
                                output_idx: output_idx as u32,
                            },
                            *slot_value_id,
                        );
                    }

                    // Pre-allocate labels inside the loop sub-DAG
                    self.allocate_blocks_and_slots(sub_dag, context)?;
                    // Lower the body
                    self.generate_instructions_from_dag(sub_dag, context)?;
                    // Exit the loop body's value scope
                    context.pop_scope();

                    // Pop loop and restore state
                    context.loop_stack.pop();
                    context.current_loop_body = prev_loop_body;
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

            Op::I32DivU => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::binary_op(BinaryOp::U32Div, result_id, inputs[0], inputs[1]);
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
                    // No local variable set yet, raise an error
                    return Err(DagToMirError::ValueMappingError(format!(
                        "Local variable {} not set",
                        local_index
                    )));
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
        context.get_value(value_origin).map_or_else(
            || {
                Err(DagToMirError::ValueMappingError(format!(
                    "Value not found: node {}, output {}",
                    value_origin.node, value_origin.output_idx
                )))
            },
            |value_id| Ok(Value::operand(value_id)),
        )
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

            (TargetType::FunctionOrLoop, depth) => {
                // If inside a loop, this is a continue to the appropriate loop header.
                // depth == 0 => current loop, depth > 0 => outer loops
                let d = depth as usize;
                if !context.loop_stack.is_empty() && d < context.loop_stack.len() {
                    let idx = context.loop_stack.len() - 1 - d;
                    let loop_info = &context.loop_stack[idx];
                    Ok(loop_info.body_block)
                } else {
                    // No active loop at this depth: treat as function return
                    let return_block = context.mir_function.add_basic_block();

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
            }
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
        // For BrIf / BrIfZero, the last input is the condition; exclude it from data copies
        let data_inputs_start_index = 0;
        let data_inputs_end_index = match &node.operation {
            Operation::BrIf(_) | Operation::BrIfZero(_) => node.inputs.len().saturating_sub(1),
            _ => node.inputs.len(),
        };

        // Get the values that should be copied to the label's slots
        let branch_values: Result<Vec<Value>, DagToMirError> = node.inputs
            [data_inputs_start_index..data_inputs_end_index]
            .iter()
            .map(|input| self.get_input_value(input, context))
            .collect();
        let branch_values = branch_values?;

        // Copy into the label's dedicated slots
        let slot_value_ids = context
            .label_output_values
            .get(&label_id)
            .cloned()
            .ok_or_else(|| {
                DagToMirError::InvalidControlFlow(format!(
                    "No slots allocated for label {}",
                    label_id
                ))
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

    /// Copy branch values to loop header storages when continuing to a loop
    fn copy_branch_values_to_loop(
        &self,
        node: &Node,
        depth: u32,
        context: &mut DagToMirContext,
    ) -> Result<(), DagToMirError> {
        // Determine which inputs represent data (exclude condition for conditional branches)
        // For BrIf / BrIfZero, the last input is the condition; exclude it from data copies
        let data_inputs_start_index = 0;
        let data_inputs_end_index = match &node.operation {
            Operation::BrIf(_) | Operation::BrIfZero(_) => node.inputs.len().saturating_sub(1),
            _ => node.inputs.len(),
        };

        // Compute target loop based on depth
        let d = depth as usize;
        if context.loop_stack.is_empty() || d >= context.loop_stack.len() {
            return Ok(()); // Treat as return; copies handled by return slots elsewhere
        }
        let idx = context.loop_stack.len() - 1 - d;
        let loop_info = &context.loop_stack[idx];

        // Get the header slots for this loop
        let header_slots = loop_info.header_slots.clone();

        // Get the values that should be copied to the loop header slots
        let branch_values: Result<Vec<Value>, DagToMirError> = node.inputs
            [data_inputs_start_index..data_inputs_end_index]
            .iter()
            .map(|input| self.get_input_value(input, context))
            .collect();
        let branch_values = branch_values?;

        let count = core::cmp::min(branch_values.len(), header_slots.len());
        for i in 0..count {
            let slot_value_id = header_slots[i];
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
