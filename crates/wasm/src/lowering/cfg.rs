use cairo_m_compiler_mir::{Terminator, Value, ValueId};
use womir::loader::blockless_dag::{BlocklessDag, BreakTarget, Node, Operation, TargetType};
use womir::loader::dag::ValueOrigin;

use super::{DagToMirContext, DagToMirError, wasm_type_to_mir_type};
use crate::loader::BlocklessDagModule;
use crate::lowering::context::ActiveLoop;

impl DagToMirContext {
    /// Pass 1: Preallocate all the blocks associated with DAG labels and loops
    /// Create phi nodes for label outputs that will be populated later
    pub(super) fn allocate_blocks_and_phi_nodes(
        &mut self,
        func: &BlocklessDag,
    ) -> Result<(), DagToMirError> {
        for (node_idx, node) in func.nodes.iter().enumerate() {
            if let Operation::Label { id } = &node.operation {
                let block_id = self.mir_function.add_basic_block();
                self.label_map.insert(*id, block_id);
                let mir_types = node
                    .output_types
                    .iter()
                    .map(|t| wasm_type_to_mir_type(t, "unknown", "label output"))
                    .collect::<Result<Vec<_>, _>>()?;
                let phi_value_ids: Vec<ValueId> = self.create_phi_nodes(block_id, &mir_types);

                // Map the label outputs to these phi nodes
                for (output_idx, &phi_id) in phi_value_ids.iter().enumerate() {
                    self.insert_value(
                        ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        },
                        phi_id,
                    );
                }
                self.label_phi_nodes.insert(*id, phi_value_ids);
            }
        }
        Ok(())
    }

    /// Pass 2: Generate MIR instructions from DAG nodes
    pub(super) fn generate_instructions_from_dag(
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
                    if let Some(mir_value) = mir_value {
                        self.insert_value(
                            ValueOrigin {
                                node: node_idx,
                                output_idx: 0,
                            },
                            mir_value,
                        );
                    }
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
                    self.record_edge_values(node, target)?;

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
                    self.record_edge_values(node, target)?;

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
                    self.record_edge_values(node, target)?;

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
                    let header_mir_types = input_node
                        .output_types
                        .iter()
                        .map(|t| wasm_type_to_mir_type(t, &self.mir_function.name, "loop input"))
                        .collect::<Result<Vec<_>, _>>()?;
                    let header_phi_nodes = self.create_phi_nodes(header_block, &header_mir_types);

                    // Record initial phi operands from loop entry
                    let current_block = self.current_block_id.unwrap();
                    for (input_idx, input) in node.inputs.iter().enumerate() {
                        if let Some(&phi_value_id) = header_phi_nodes.get(input_idx) {
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
                            self.add_deferred_phi_operand(
                                header_block,
                                phi_value_id,
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

    /// Helper: record phi operands for the taken edge of a branch based on BreakTarget kind
    pub(super) fn record_edge_values(
        &mut self,
        node: &Node,
        target: &BreakTarget,
    ) -> Result<(), DagToMirError> {
        match &target.kind {
            TargetType::Label(label_id) => self.record_branch_values_for_label(node, *label_id),
            TargetType::FunctionOrLoop => self.record_branch_values_for_loop(node, target.depth),
        }
    }

    /// Resolve a WASM break target to a MIR BasicBlockId
    pub(super) fn resolve_break_target(
        &mut self,
        node_idx: usize,
        node: &Node,
        target: &BreakTarget,
    ) -> Result<cairo_m_compiler_mir::BasicBlockId, DagToMirError> {
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

                    let return_values: Vec<Value> = node
                        .inputs
                        .iter()
                        .map(|input| self.get_input_value(input))
                        .collect::<Result<_, _>>()?;

                    let terminator = Terminator::return_values(return_values);
                    self.block_mut(return_block)?.set_terminator(terminator);
                    Ok(return_block)
                }
            }
        }
    }

    /// Extract branch values from a node, excluding conditional inputs
    pub(super) fn get_branch_values(&self, node: &Node) -> Result<Vec<Value>, DagToMirError> {
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
    pub(super) fn record_branch_values_for_label(
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
    pub(super) fn record_branch_values_for_loop(
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
