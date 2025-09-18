//! Lowering from WOMIR BlockLess DAG to Cairo-M MIR.

use crate::loader::{BlocklessDagModule, WasmLoadError};
use cairo_m_compiler_mir::{
    instruction::{CalleeSignature, InstructionKind},
    BasicBlock, BasicBlockId, BinaryOp, FunctionId, Instruction, MirFunction, MirModule, MirType,
    PassManager, Terminator, Value, ValueId,
};
use std::collections::HashMap;
use thiserror::Error;
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::{BlocklessDag, BreakTarget, Node, Operation, TargetType};
use womir::loader::dag::ValueOrigin;
use womir::loader::FunctionProcessingStage;

#[derive(Error, Debug)]
pub enum DagToMirError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
    #[error("Unsupported WASM operation {op:?} in function '{function_name}' at node {node_idx}: {suggestion}")]
    UnsupportedOperation {
        op: String,
        function_name: String,
        node_idx: usize,
        suggestion: String,
    },
    #[error("Invalid control flow in function '{function_name}': {reason}")]
    InvalidControlFlow {
        function_name: String,
        reason: String,
        operation_context: String,
    },
    #[error("Value mapping error in function '{function_name}' at node {node_idx}: {reason} (available: {available_count} values)")]
    ValueMappingError {
        function_name: String,
        node_idx: usize,
        reason: String,
        available_count: usize,
    },
    #[error("Unsupported WASM type {wasm_type:?} in function '{function_name}': {context}")]
    UnsupportedWasmType {
        wasm_type: wasmparser::ValType,
        function_name: String,
        context: String,
    },
    #[error("Loop structure error in function '{function_name}' at node {node_idx}: depth {requested_depth} exceeds available {available_depth}")]
    LoopDepthError {
        function_name: String,
        node_idx: usize,
        requested_depth: u32,
        available_depth: usize,
    },
}

/// Context for converting a single function DAG to MIR
pub struct DagToMirContext {
    /// MIR function being built
    mir_function: MirFunction,
    /// Stack of value maps to scope ValueOrigin -> ValueId per DAG (avoids collisions)
    value_maps: Vec<HashMap<ValueOrigin, ValueId>>,
    /// Mapping from DAG label IDs to MIR BasicBlockId
    label_map: HashMap<u32, BasicBlockId>,
    /// Current basic block being filled
    current_block_id: Option<BasicBlockId>,
    /// Current source block for tracking control flow (unused for now)
    current_source_block: Option<BasicBlockId>,
    /// For each label id, the phi nodes that need to be populated (dest ValueId -> phi instruction)
    label_phi_nodes: HashMap<u32, Vec<ValueId>>,
    /// Stack of active loops to support continues and loop-carried variables
    loop_stack: Vec<ActiveLoop>,
    /// Deferred phi operands: (block_id, dest_value_id, source_block_id, source_value)
    deferred_phi_operands: Vec<(BasicBlockId, ValueId, BasicBlockId, Value)>,
}

/// Information about an active loop during lowering
struct ActiveLoop {
    /// Header basic block for this loop
    header_block: BasicBlockId,
    /// Phi nodes in the header for loop-carried values
    header_phi_nodes: Vec<ValueId>,
}

/// Convert the DAG representation of the module to MIR
pub fn lower_program_to_mir(
    module: &BlocklessDagModule,
    mut pipeline: PassManager,
) -> Result<MirModule, DagToMirError> {
    let mut mir_module = MirModule::new();
    let program = &module.0;
    for (func_idx, _) in program.functions.iter().enumerate() {
        let function_id = FunctionId::new(func_idx);
        let mut mir_function = function_to_mir(module, func_idx)?;
        pipeline.run(&mut mir_function);
        mir_module
            .function_names
            .insert(mir_function.name.clone(), function_id);
        mir_module.functions.insert(function_id, mir_function);
    }
    Ok(mir_module)
}

/// Convert WASM type to MIR type
/// For now, we only support i32
fn wasm_type_to_mir_type(
    wasm_type: &wasmparser::ValType,
    function_name: &str,
    context: &str,
) -> Result<MirType, DagToMirError> {
    match wasm_type {
        wasmparser::ValType::I32 => Ok(MirType::U32),
        _ => Err(DagToMirError::UnsupportedWasmType {
            wasm_type: *wasm_type,
            function_name: function_name.to_string(),
            context: context.to_string(),
        }),
    }
}

fn function_to_mir(
    module: &BlocklessDagModule,
    func_idx: usize,
) -> Result<MirFunction, DagToMirError> {
    let program = &module.0;
    let func_name = program
        .m
        .exported_functions
        .get(&(func_idx as u32))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("func_{}", func_idx));

    let mut context = DagToMirContext::new(func_name.clone());

    // Get function type information for parameters and return types
    let func_type = program.m.get_func_type(func_idx as u32);

    // Handle param types with proper error handling
    let param_types: Vec<MirType> = func_type
        .ty
        .params()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function parameters"))
        .collect::<Result<Vec<MirType>, DagToMirError>>()?;

    // Handle return types with proper error handling
    let return_types: Vec<MirType> = func_type
        .ty
        .results()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function return types"))
        .collect::<Result<Vec<MirType>, DagToMirError>>()?;

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
    let dag = match program.functions.get(func_idx) {
        Some(FunctionProcessingStage::BlocklessDag(dag)) => dag,
        Some(_) => {
            return Err(DagToMirError::InvalidControlFlow {
                function_name: func_name,
                reason: "Function not at BlocklessDag stage".to_string(),
                operation_context: "lowering function".to_string(),
            });
        }
        None => {
            return Err(DagToMirError::ValueMappingError {
                function_name: func_name,
                node_idx: 0,
                reason: format!("Function {} not found", func_idx),
                available_count: program.functions.len(),
            });
        }
    };

    // Preallocate all the blocks associated with DAG labels and loops
    context.allocate_blocks_and_phi_nodes(dag)?;

    // Generate instructions and control flow
    context.generate_instructions_from_dag(dag, module)?;

    // Finalize all phi nodes with their operands
    context.finalize_phi_nodes()?;

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

impl DagToMirContext {}

impl DagToMirContext {
    fn new(func_name: String) -> Self {
        let mir_function = MirFunction::new(func_name);

        Self {
            value_maps: vec![HashMap::new()],
            label_map: HashMap::new(),
            current_source_block: None,
            label_phi_nodes: HashMap::new(),

            mir_function,
            current_block_id: Some(0.into()),
            loop_stack: Vec::new(),
            deferred_phi_operands: Vec::new(),
        }
    }

    fn get_current_block(&mut self) -> Result<&mut BasicBlock, DagToMirError> {
        let block_id = self
            .current_block_id
            .ok_or_else(|| DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: "No current block set - invalid state".to_string(),
                operation_context: "attempting to get current block".to_string(),
            })?;

        self.mir_function
            .basic_blocks
            .get_mut(block_id)
            .ok_or_else(|| DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: format!("Block {:?} does not exist", block_id),
                operation_context: "attempting to access basic block".to_string(),
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

    /// Add a deferred phi operand to be processed later
    fn add_deferred_phi_operand(
        &mut self,
        dest_block: BasicBlockId,
        dest_value: ValueId,
        source_block: BasicBlockId,
        source_value: Value,
    ) {
        self.deferred_phi_operands
            .push((dest_block, dest_value, source_block, source_value));
    }

    /// Finalize all phi nodes by adding their operands
    fn finalize_phi_nodes(&mut self) -> Result<(), DagToMirError> {
        // Group deferred operands by (block_id, dest_value_id)
        let mut phi_operands: HashMap<(BasicBlockId, ValueId), Vec<(BasicBlockId, Value)>> =
            HashMap::new();

        for (dest_block, dest_value, source_block, source_value) in &self.deferred_phi_operands {
            phi_operands
                .entry((*dest_block, *dest_value))
                .or_default()
                .push((*source_block, *source_value));
        }

        // Update phi instructions with their operands
        for ((block_id, dest_value_id), operands) in phi_operands {
            let function_name = self.mir_function.name.clone();
            let block = self
                .mir_function
                .get_basic_block_mut(block_id)
                .ok_or_else(|| DagToMirError::InvalidControlFlow {
                    function_name,
                    reason: format!("Block {:?} not found when finalizing phi nodes", block_id),
                    operation_context: "finalizing phi nodes".to_string(),
                })?;

            // Find the phi instruction with this destination
            for instruction in block.phi_instructions_mut() {
                // Get the destination value id before borrowing mutably
                let phi_dest = if let InstructionKind::Phi { dest, .. } = &instruction.kind {
                    Some(*dest)
                } else {
                    None
                };

                if let Some(dest) = phi_dest {
                    if dest == dest_value_id {
                        if let Some(phi_operands_mut) = instruction.phi_operands_mut() {
                            *phi_operands_mut = operands.clone();
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Pass 1: Preallocate all the blocks associated with DAG labels and loops
    /// Create phi nodes for label outputs that will be populated later
    fn allocate_blocks_and_phi_nodes(&mut self, func: &BlocklessDag) -> Result<(), DagToMirError> {
        for (node_idx, node) in func.nodes.iter().enumerate() {
            let id = match &node.operation {
                Operation::Label { id } => *id,
                _ => continue,
            };
            let block_id = self.mir_function.add_basic_block();
            self.label_map.insert(id, block_id);
            let mut phi_value_ids: Vec<ValueId> = Vec::new();

            // Create phi nodes for each label output
            for (output_idx, output_type) in node.output_types.iter().enumerate() {
                let mir_type = wasm_type_to_mir_type(output_type, "unknown", "label output")?;
                let phi_value_id = self.mir_function.new_typed_value_id(mir_type.clone());

                // Create empty phi node that will be populated later
                let phi_instruction = Instruction::empty_phi(phi_value_id, mir_type);
                self.mir_function
                    .get_basic_block_mut(block_id)
                    .unwrap()
                    .push_phi_front(phi_instruction);

                // Map the label output to this phi node value
                self.insert_value(
                    ValueOrigin {
                        node: node_idx,
                        output_idx: output_idx as u32,
                    },
                    phi_value_id,
                );
                phi_value_ids.push(phi_value_id);
            }
            self.label_phi_nodes.insert(id, phi_value_ids);
        }
        Ok(())
    }

    /// Pass 2: Generate MIR instructions from DAG nodes
    fn generate_instructions_from_dag(
        &mut self,
        dag: &BlocklessDag,
        module: &BlocklessDagModule,
    ) -> Result<(), DagToMirError> {
        for (node_idx, node) in dag.nodes.iter().enumerate() {
            match &node.operation {
                Operation::Inputs => {}

                Operation::WASMOp(wasm_op) => {
                    // Convert WASM operation to MIR instruction
                    let mir_value = self.convert_wasm_op_to_mir(node_idx, wasm_op, node, module)?;
                    self.insert_value(
                        ValueOrigin {
                            node: node_idx,
                            output_idx: 0,
                        },
                        mir_value,
                    );
                }

                Operation::Label { id } => {
                    let block_id = self.label_map.get(id).copied().ok_or_else(|| {
                        DagToMirError::InvalidControlFlow {
                            function_name: self.mir_function.name.clone(),
                            reason: format!("Label {:?} not found", id),
                            operation_context: "resolving label".to_string(),
                        }
                    })?;
                    self.set_current_block(block_id);
                }

                Operation::Br(target) => {
                    // This is either a jump or a return
                    let target_block = self.resolve_break_target(node_idx, node, target)?;

                    // Edge copies
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.record_branch_values_for_label(node, *label_id)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.record_branch_values_for_loop(node, target.depth)?;
                        }
                    }

                    let terminator = Terminator::jump(target_block);
                    self.get_current_block()?.set_terminator(terminator);
                    self.set_current_block(target_block);
                }

                Operation::BrIf(target) => {
                    // Conditional branch - in our DAG, the condition is the last input
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToMirError::InvalidControlFlow {
                            function_name: self.mir_function.name.clone(),
                            reason: "BrIf without condition input".to_string(),
                            operation_context: "resolving BrIf condition".to_string(),
                        }
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx])?;
                    let target_block = self.resolve_break_target(node_idx, node, target)?;
                    let else_block = self.mir_function.add_basic_block();

                    // Edge copies on the taken edge
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.record_branch_values_for_label(node, *label_id)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.record_branch_values_for_loop(node, target.depth)?;
                        }
                    }

                    let terminator = Terminator::branch(condition_value, target_block, else_block);
                    self.get_current_block()?.set_terminator(terminator);
                    self.set_current_block(else_block);
                }

                Operation::BrIfZero(target) => {
                    // Inverted conditional branch - condition is the last input
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToMirError::InvalidControlFlow {
                            function_name: self.mir_function.name.clone(),
                            reason: "BrIfZero without condition input".to_string(),
                            operation_context: "resolving BrIfZero condition".to_string(),
                        }
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx])?;
                    let else_target = self.resolve_break_target(node_idx, node, target)?;
                    let then_target = self.mir_function.add_basic_block();

                    // Edge copies on the taken edge
                    match &target.kind {
                        TargetType::Label(label_id) => {
                            self.record_branch_values_for_label(node, *label_id)?;
                        }
                        TargetType::FunctionOrLoop => {
                            self.record_branch_values_for_loop(node, target.depth)?;
                        }
                    }

                    let terminator = Terminator::branch(condition_value, then_target, else_target);
                    self.get_current_block()?.set_terminator(terminator);
                    self.set_current_block(then_target);
                }

                Operation::BrTable { targets: _ } => {
                    todo!()
                }

                Operation::Loop {
                    sub_dag,
                    break_targets: _,
                } => {
                    // Build a normal loop (header/body/exit) from the sub-DAG
                    // Create header block and allocate phi nodes for loop-carried values
                    let header_block = self.mir_function.add_basic_block();

                    // Get loop input types from the sub-DAG's Inputs node
                    let sub_inputs_idx = 0;
                    let input_node = &sub_dag.nodes[sub_inputs_idx];
                    assert!(
                        matches!(input_node.operation, Operation::Inputs),
                        "Loop sub-DAG must start with Inputs node"
                    );

                    // Create phi nodes in the header for loop-carried values
                    let mut header_phi_nodes = Vec::new();
                    for output_type in &input_node.output_types {
                        let mir_type = wasm_type_to_mir_type(
                            output_type,
                            &self.mir_function.name,
                            "loop input",
                        )?;
                        let phi_value_id = self.mir_function.new_typed_value_id(mir_type.clone());

                        // Create empty phi node that will be populated later
                        let phi_instruction = Instruction::empty_phi(phi_value_id, mir_type);
                        self.mir_function
                            .get_basic_block_mut(header_block)
                            .unwrap()
                            .push_phi_front(phi_instruction);

                        header_phi_nodes.push(phi_value_id);
                    }

                    // Record initial phi operands from loop entry
                    let current_block = self.current_block_id.unwrap();
                    for (input_idx, input) in node.inputs.iter().enumerate() {
                        if let Some(phi_value_id) = header_phi_nodes.get(input_idx) {
                            let source_value_id = self.get_value(input).ok_or_else(|| {
                                let available_count =
                                    self.value_maps.iter().map(|map| map.len()).sum::<usize>();
                                DagToMirError::ValueMappingError {
                                    function_name: self.mir_function.name.clone(),
                                    node_idx: input.node,
                                    reason: format!(
                                        "Loop input {} (node {}, output {}) not found in value map",
                                        input_idx, input.node, input.output_idx
                                    ),
                                    available_count,
                                }
                            })?;

                            // Add phi operand for loop entry
                            self.add_deferred_phi_operand(
                                header_block,
                                *phi_value_id,
                                current_block,
                                Value::operand(source_value_id),
                            );
                        }
                    }

                    let terminator = Terminator::jump(header_block);
                    self.get_current_block()?.set_terminator(terminator);
                    self.set_current_block(header_block);

                    // Allocate a new block for the loop body
                    let body_block = self.mir_function.add_basic_block();
                    let terminator = Terminator::jump(body_block);
                    self.get_current_block()?.set_terminator(terminator);
                    self.set_current_block(body_block);

                    self.loop_stack.push(ActiveLoop {
                        header_block,
                        header_phi_nodes: header_phi_nodes.clone(),
                    });

                    // Enter a new value scope for the loop body to avoid ValueOrigin collisions
                    self.push_scope();

                    // Map the sub-DAG's Inputs node (node 0) to header phi nodes
                    for (output_idx, phi_value_id) in header_phi_nodes.iter().enumerate() {
                        self.insert_value(
                            ValueOrigin {
                                node: 0,
                                output_idx: output_idx as u32,
                            },
                            *phi_value_id,
                        );
                    }

                    // Pre-allocate labels inside the loop sub-DAG
                    self.allocate_blocks_and_phi_nodes(sub_dag)?;
                    // Lower the body
                    self.generate_instructions_from_dag(sub_dag, module)?;
                    // Exit the loop body's value scope
                    self.pop_scope();

                    // Pop loop and restore state
                    self.loop_stack.pop();
                }
            }
        }

        Ok(())
    }

    /// Convert a WASM binary opcode to a MIR binary opcode
    /// TODO : bit shifts, rotations, u8 operations, etc.
    fn wasm_binary_opcode_to_mir(
        &self,
        wasm_op: &Op,
        node_idx: usize,
        context: &Self,
    ) -> Result<BinaryOp, DagToMirError> {
        match wasm_op {
            Op::I32Add => Ok(BinaryOp::U32Add),
            Op::I32Sub => Ok(BinaryOp::U32Sub),
            Op::I32Mul => Ok(BinaryOp::U32Mul),
            Op::I32DivU => Ok(BinaryOp::U32Div),
            Op::I32Eq => Ok(BinaryOp::U32Eq),
            Op::I32Ne => Ok(BinaryOp::U32Neq),
            Op::I32GtU => Ok(BinaryOp::U32Greater),
            Op::I32GeU => Ok(BinaryOp::U32GreaterEqual),
            Op::I32LtU => Ok(BinaryOp::U32Less),
            Op::I32LeU => Ok(BinaryOp::U32LessEqual),
            Op::I32And => Ok(BinaryOp::U32BitwiseAnd),
            Op::I32Or => Ok(BinaryOp::U32BitwiseOr),
            Op::I32Xor => Ok(BinaryOp::U32BitwiseXor),
            _ => Err(DagToMirError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: context.mir_function.name.clone(),
                node_idx,
                suggestion: "".to_string(),
            }),
        }
    }

    fn convert_wasm_binop_to_mir(
        &mut self,
        node_idx: usize,
        wasm_op: &Op,
        left: Value,
        right: Value,
        dest_type: MirType,
    ) -> Result<ValueId, DagToMirError> {
        let result_id = self.mir_function.new_typed_value_id(dest_type);
        let mir_op = self.wasm_binary_opcode_to_mir(wasm_op, node_idx, self)?;
        let instruction = Instruction::binary_op(mir_op, result_id, left, right);
        self.get_current_block()?.push_instruction(instruction);
        Ok(result_id)
    }

    /// Convert a WASM operation to MIR instructions
    fn convert_wasm_op_to_mir(
        &mut self,
        node_idx: usize,
        wasm_op: &Op,
        node: &Node,
        module: &BlocklessDagModule,
    ) -> Result<ValueId, DagToMirError> {
        let inputs: Result<Vec<Value>, _> = node
            .inputs
            .iter()
            .map(|input| self.get_input_value(input))
            .collect();
        let inputs = inputs?;

        match wasm_op {
            // U32 Operations which are immediately convertible to MIR instructions
            Op::I32Add
            | Op::I32Sub
            | Op::I32Mul
            | Op::I32DivU
            | Op::I32And
            | Op::I32Or
            | Op::I32Xor => self.convert_wasm_binop_to_mir(
                node_idx,
                wasm_op,
                inputs[0],
                inputs[1],
                MirType::U32,
            ),

            // For comparisons, we produce a boolean result
            // This is not WASM compliant, but works if these values are only used in conditional branches
            // TODO : cast everything correctly or sync with VM so that comparisons between u32 produce u32 booleans
            Op::I32Eq | Op::I32Ne | Op::I32GtU | Op::I32GeU | Op::I32LtU | Op::I32LeU => self
                .convert_wasm_binop_to_mir(node_idx, wasm_op, inputs[0], inputs[1], MirType::Bool),

            // Signed comparison instructions, constructed by shifting the inputs by 2^31 and then comparing the results with unsigned opcodes
            Op::I32LtS | Op::I32GtS | Op::I32LeS | Op::I32GeS => {
                let temp1 = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction1 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp1,
                    inputs[0],
                    Value::integer(0x80000000),
                );
                let temp2 = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction2 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp2,
                    inputs[1],
                    Value::integer(0x80000000),
                );
                let result_id = self.mir_function.new_typed_value_id(MirType::Bool);
                let op = match wasm_op {
                    Op::I32LtS => BinaryOp::U32Less,
                    Op::I32GtS => BinaryOp::U32Greater,
                    Op::I32LeS => BinaryOp::U32LessEqual,
                    Op::I32GeS => BinaryOp::U32GreaterEqual,
                    _ => unreachable!(),
                };
                let instruction3 = Instruction::binary_op(
                    op,
                    result_id,
                    Value::operand(temp1),
                    Value::operand(temp2),
                );
                self.get_current_block()?.push_instruction(instruction1);
                self.get_current_block()?.push_instruction(instruction2);
                self.get_current_block()?.push_instruction(instruction3);
                Ok(result_id)
            }

            // Zero comparison instruction, constructed by comparing the input to 0
            // TODO : fix type of result_id
            Op::I32Eqz => {
                let result_id = self.mir_function.new_typed_value_id(MirType::Bool);
                let instruction = Instruction::binary_op(
                    BinaryOp::U32Eq,
                    result_id,
                    inputs[0],
                    Value::integer(0),
                );
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            // Assigning a constant to a variable
            Op::I32Const { value } => {
                let result_id = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::assign(result_id, Value::integer(*value as u32), MirType::U32);
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            // Local variable operations should be eliminated by WOMIR
            Op::LocalGet { .. } | Op::LocalSet { .. } | Op::LocalTee { .. } => {
                unreachable!()
            }

            Op::Call { function_index } => {
                let result_id = self.mir_function.new_typed_value_id(MirType::U32);
                let callee_id = FunctionId::new(*function_index as usize);

                // Get signature from wasm module
                let program = &module.0;
                let func_type = program.m.get_func_type(*function_index);

                // Handle param types with proper error handling
                let param_types: Vec<MirType> = func_type
                    .ty
                    .params()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call parameters"))
                    .collect::<Result<Vec<MirType>, DagToMirError>>()?;

                // Handle return types with proper error handling
                let return_types: Vec<MirType> = func_type
                    .ty
                    .results()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call return types"))
                    .collect::<Result<Vec<MirType>, DagToMirError>>()?;

                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                let instruction = Instruction::call(vec![result_id], callee_id, inputs, signature);
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            _ => {
                // Unsupported operation
                let suggestion = "This WASM operation is not yet implemented in the compiler";

                Err(DagToMirError::UnsupportedOperation {
                    op: format!("{:?}", wasm_op),
                    function_name: self.mir_function.name.clone(),
                    node_idx,
                    suggestion: suggestion.to_string(),
                })
            }
        }
    }

    /// Get MIR value for a WASM ValueOrigin
    fn get_input_value(&self, value_origin: &ValueOrigin) -> Result<Value, DagToMirError> {
        self.get_value(value_origin).map_or_else(
            || {
                let available_count = self.value_maps.iter().map(|map| map.len()).sum::<usize>();
                Err(DagToMirError::ValueMappingError {
                    function_name: self.mir_function.name.clone(),
                    node_idx: value_origin.node,
                    reason: format!(
                        "Value not found: node {}, output {}",
                        value_origin.node, value_origin.output_idx
                    ),
                    available_count,
                })
            },
            |value_id| Ok(Value::operand(value_id)),
        )
    }

    /// Resolve a WASM break target to a MIR BasicBlockId
    fn resolve_break_target(
        &mut self,
        node_idx: usize,
        node: &Node,
        target: &BreakTarget,
    ) -> Result<BasicBlockId, DagToMirError> {
        match (&target.kind, target.depth) {
            (TargetType::Label(label_id), _depth) => {
                // Direct jump to a label at current scope
                self.label_map.get(label_id).map_or_else(
                    || {
                        Err(DagToMirError::InvalidControlFlow {
                            function_name: self.mir_function.name.clone(),
                            reason: format!("Label {} not found in label_map", label_id),
                            operation_context: "resolving break target".to_string(),
                        })
                    },
                    |block_id| Ok(*block_id),
                )
            }

            (TargetType::FunctionOrLoop, depth) => {
                // If inside a loop, this is a continue to the appropriate loop header.
                // depth == 0 => current loop, depth > 0 => outer loops
                let d = depth as usize;
                if !self.loop_stack.is_empty() && d < self.loop_stack.len() {
                    let idx = self.loop_stack.len() - 1 - d;
                    let loop_info = &self.loop_stack[idx];
                    Ok(loop_info.header_block)
                } else if d >= self.loop_stack.len() && !self.loop_stack.is_empty() {
                    return Err(DagToMirError::LoopDepthError {
                        function_name: self.mir_function.name.clone(),
                        node_idx,
                        requested_depth: depth,
                        available_depth: self.loop_stack.len(),
                    });
                } else {
                    // No active loop at this depth: treat as function return
                    let return_block = self.mir_function.add_basic_block();

                    let node_inputs = node.inputs.clone();
                    let return_values: Result<Vec<Value>, DagToMirError> = node_inputs
                        .iter()
                        .map(|input| self.get_input_value(input))
                        .collect();

                    let return_values = return_values?;
                    let terminator = Terminator::return_values(return_values);
                    let function_name = self.mir_function.name.clone();
                    self.mir_function
                        .get_basic_block_mut(return_block)
                        .ok_or_else(|| DagToMirError::InvalidControlFlow {
                            function_name,
                            reason: format!("Block {:?} not found", return_block),
                            operation_context: "setting return terminator".to_string(),
                        })?
                        .set_terminator(terminator);
                    Ok(return_block)
                }
            }
        }
    }

    /// Extract branch values from a node, excluding conditional inputs
    fn get_branch_values(&self, node: &Node) -> Result<Vec<Value>, DagToMirError> {
        // Determine which inputs represent data (exclude condition for conditional branches)
        // For BrIf / BrIfZero, the last input is the condition; exclude it from data copies
        let data_inputs_start_index = 0;
        let data_inputs_end_index = match &node.operation {
            Operation::BrIf(_) | Operation::BrIfZero(_) => node.inputs.len().saturating_sub(1),
            _ => node.inputs.len(),
        };

        node.inputs[data_inputs_start_index..data_inputs_end_index]
            .iter()
            .map(|input| self.get_input_value(input))
            .collect()
    }

    /// Record branch values as phi operands for a label when branching to it
    fn record_branch_values_for_label(
        &mut self,
        node: &Node,
        label_id: u32,
    ) -> Result<(), DagToMirError> {
        let branch_values = self.get_branch_values(node)?;
        let current_block =
            self.current_block_id
                .ok_or_else(|| DagToMirError::InvalidControlFlow {
                    function_name: self.mir_function.name.clone(),
                    reason: "No current block when recording phi operands".to_string(),
                    operation_context: "recording branch values for label".to_string(),
                })?;

        let phi_value_ids = self
            .label_phi_nodes
            .get(&label_id)
            .cloned()
            .ok_or_else(|| DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: format!("No phi nodes allocated for label {}", label_id),
                operation_context: "recording branch values for label".to_string(),
            })?;

        let target_block = self.label_map.get(&label_id).copied().ok_or_else(|| {
            DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: format!("Label {} not found in label_map", label_id),
                operation_context: "recording branch values for label".to_string(),
            }
        })?;

        // Record phi operands for each value
        let count = core::cmp::min(branch_values.len(), phi_value_ids.len());
        for i in 0..count {
            self.add_deferred_phi_operand(
                target_block,
                phi_value_ids[i],
                current_block,
                branch_values[i],
            );
        }

        Ok(())
    }

    /// Record branch values as phi operands for loop header when continuing to a loop
    fn record_branch_values_for_loop(
        &mut self,
        node: &Node,
        depth: u32,
    ) -> Result<(), DagToMirError> {
        // Compute target loop based on depth
        let d = depth as usize;
        if self.loop_stack.is_empty() || d >= self.loop_stack.len() {
            return Ok(()); // Treat as return; phi operands handled by return elsewhere
        }
        let idx = self.loop_stack.len() - 1 - d;
        let loop_info = &self.loop_stack[idx];
        let header_phi_nodes = loop_info.header_phi_nodes.clone();
        let header_block = loop_info.header_block;

        let current_block =
            self.current_block_id
                .ok_or_else(|| DagToMirError::InvalidControlFlow {
                    function_name: self.mir_function.name.clone(),
                    reason: "No current block when recording phi operands".to_string(),
                    operation_context: "recording branch values for loop".to_string(),
                })?;

        let branch_values = self.get_branch_values(node)?;

        // Record phi operands for each loop-carried value
        let count = core::cmp::min(branch_values.len(), header_phi_nodes.len());
        for i in 0..count {
            self.add_deferred_phi_operand(
                header_block,
                header_phi_nodes[i],
                current_block,
                branch_values[i],
            );
        }

        Ok(())
    }
}
