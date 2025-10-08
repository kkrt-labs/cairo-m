use super::{wasm_type_to_mir_type, DagToCasmContext, DagToCasmError};
use crate::loader::BlocklessDagModule;
use crate::lowering::context::ActiveLoop;
use cairo_m_compiler_codegen::Label;
use cairo_m_compiler_mir::{MirType, Value, ValueId};
use womir::loader::blockless_dag::{BlocklessDag, BreakTarget, Node, Operation, TargetType};
use womir::loader::dag::ValueOrigin;

pub(super) enum SolvedBreakTarget {
    Label(String),
    Return(Vec<Value>),
}

impl DagToCasmContext {
    /// Generate CASM instructions from DAG nodes
    pub(super) fn generate_instructions_from_dag(
        &mut self,
        dag: &BlocklessDag,
        module: &BlocklessDagModule,
    ) -> Result<(), DagToCasmError> {
        // First pass: pre-register all labels and allocate their slots
        // This must be done separately to handle forward references where branches
        // to a label appear before the label definition in the DAG
        for (node_idx, node) in dag.nodes.iter().enumerate() {
            if let Operation::Label { id } = &node.operation {
                // Register label name
                let label_name = self.casm_builder.emit_new_label_name(".L");
                self.label_names.insert(*id, label_name.clone());

                // Allocate slots for label outputs (values flowing through this merge point)
                let mut slots = Vec::new();
                for (output_idx, output_type) in node.output_types.iter().enumerate() {
                    let slot_id = self.new_typed_value_id(wasm_type_to_mir_type(
                        output_type,
                        &self.casm_builder.layout.name,
                        "label output",
                    )?);
                    slots.push(slot_id);

                    // Register the slot as this label node's output
                    self.insert_value(
                        ValueOrigin {
                            node: node_idx,
                            output_idx: output_idx as u32,
                        },
                        slot_id,
                    );
                }
                self.label_slots.insert(*id, slots);
            }
        }

        // Second pass: generate instructions and register label outputs
        for (node_idx, node) in dag.nodes.iter().enumerate() {
            match &node.operation {
                Operation::Inputs => {}

                Operation::WASMOp(wasm_op) => {
                    // Convert WASM operation to CASM instruction
                    let casm_value =
                        self.convert_wasm_op_to_casm(node_idx, wasm_op, node, module)?;

                    if let Some(mir_value) = casm_value {
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
                    // Emit label (outputs were already registered in first pass)
                    let label_name = self.label_names.get(id).cloned().ok_or_else(|| {
                        DagToCasmError::InvalidControlFlow {
                            function_name: self.casm_builder.layout.name.clone(),
                            reason: format!("Label {} not pre-registered", id),
                            operation_context: "emitting label".to_string(),
                        }
                    })?;
                    self.casm_builder.emit_add_label(Label {
                        name: label_name,
                        address: None,
                    });
                }

                Operation::Br(target) => {
                    // Get branch values before resolving target
                    let branch_values = self.get_branch_values(node)?;

                    // This is either a jump or a return
                    let resolved_target = self.resolve_break_target(node_idx, node, target)?;

                    match resolved_target {
                        SolvedBreakTarget::Label(label) => {
                            // Store values to the label's stack slots before jumping
                            self.store_to_label_slots(target, &branch_values)?;
                            self.casm_builder.jump(label.as_str());
                        }
                        SolvedBreakTarget::Return(return_values) => {
                            // Return from function with values
                            self.casm_builder.return_values(
                                &return_values,
                                &return_values
                                    .iter()
                                    .map(|_| MirType::U32)
                                    .collect::<Vec<_>>(),
                            )?;
                        }
                    }
                }

                Operation::BrIf(target) => {
                    // Conditional branch - in our DAG, the condition is the last input
                    // BrIf takes the branch when condition is NON-ZERO
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToCasmError::InvalidControlFlow {
                            function_name: self.casm_builder.layout.name.clone(),
                            reason: "BrIf without condition input".to_string(),
                            operation_context: "resolving BrIf condition".to_string(),
                        }
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx])?;

                    // Get branch values before resolving target
                    let branch_values = self.get_branch_values(node)?;

                    let resolved_target = self.resolve_break_target(node_idx, node, target)?;

                    match resolved_target {
                        SolvedBreakTarget::Label(label) => {
                            // Create a label for the taken path
                            let taken_label = self.casm_builder.emit_new_label_name(".br_taken");

                            // If condition is non-zero, jump to taken path
                            self.casm_builder
                                .jnz(condition_value, taken_label.as_str())?;

                            // Fallthrough path: continue execution
                            let fallthrough_label =
                                self.casm_builder.emit_new_label_name(".br_fallthrough");
                            self.casm_builder.jump(fallthrough_label.as_str());

                            // Taken path: store values and jump to target
                            self.casm_builder.emit_add_label(Label {
                                name: taken_label,
                                address: None,
                            });
                            self.store_to_label_slots(target, &branch_values)?;
                            self.casm_builder.jump(label.as_str());

                            // Fallthrough continues here
                            self.casm_builder.emit_add_label(Label {
                                name: fallthrough_label,
                                address: None,
                            });
                        }
                        SolvedBreakTarget::Return(_return_values) => {
                            // These seem to never be generated by WOMIR
                            Err(DagToCasmError::InvalidControlFlow {
                                function_name: self.casm_builder.layout.name.clone(),
                                reason: "Conditional return not yet implemented".to_string(),
                                operation_context: "resolving BrIf return".to_string(),
                            })?;
                        }
                    }
                }

                Operation::BrIfZero(target) => {
                    // Inverted conditional branch - condition is the last input
                    // BrIfZero takes the branch when condition is ZERO
                    let cond_idx = node.inputs.len().checked_sub(1).ok_or_else(|| {
                        DagToCasmError::InvalidControlFlow {
                            function_name: self.casm_builder.layout.name.clone(),
                            reason: "BrIfZero without condition input".to_string(),
                            operation_context: "resolving BrIfZero condition".to_string(),
                        }
                    })?;
                    let condition_value = self.get_input_value(&node.inputs[cond_idx])?;

                    // Get branch values before resolving target
                    let branch_values = self.get_branch_values(node)?;

                    let resolved_target = self.resolve_break_target(node_idx, node, target)?;

                    match resolved_target {
                        SolvedBreakTarget::Label(label) => {
                            // Create a label for the fallthrough path
                            let fallthrough_label =
                                self.casm_builder.emit_new_label_name(".fallthrough");

                            // If condition is non-zero, skip the branch (jump to fallthrough)
                            self.casm_builder
                                .jnz(condition_value, fallthrough_label.as_str())?;

                            // Taken path (when zero): store values and jump to target
                            self.store_to_label_slots(target, &branch_values)?;
                            self.casm_builder.jump(label.as_str());

                            // Fallthrough path continues here
                            self.casm_builder.emit_add_label(Label {
                                name: fallthrough_label,
                                address: None,
                            });
                        }
                        SolvedBreakTarget::Return(_return_values) => {
                            // These seem to never be generated by WOMIR
                            Err(DagToCasmError::InvalidControlFlow {
                                function_name: self.casm_builder.layout.name.clone(),
                                reason: "Conditional return not yet implemented".to_string(),
                                operation_context: "resolving BrIfZero return".to_string(),
                            })?;
                        }
                    }
                }

                Operation::BrTable { targets: _ } => {
                    todo!()
                }

                Operation::Loop {
                    sub_dag,
                    break_targets: _,
                } => {
                    // Get loop input types from the sub-DAG's Inputs node
                    let sub_inputs_idx = 0;
                    let input_node = &sub_dag.nodes[sub_inputs_idx];
                    assert!(
                        matches!(input_node.operation, Operation::Inputs),
                        "Loop sub-DAG must start with Inputs node"
                    );

                    // Create stack slots for loop-carried values
                    let header_mir_types = input_node
                        .output_types
                        .iter()
                        .map(|t| {
                            wasm_type_to_mir_type(t, &self.casm_builder.layout.name, "loop input")
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    let header_slots: Vec<ValueId> = header_mir_types
                        .iter()
                        .map(|t| self.new_typed_value_id(t.clone()))
                        .collect();

                    // Store initial values (from outer scope) into the loop slots
                    let initial_values = self.get_branch_values(node)?;
                    for (slot, value) in header_slots.iter().zip(initial_values.iter()) {
                        let mir_type = self.value_types.get(slot).ok_or_else(|| {
                            DagToCasmError::InvalidControlFlow {
                                function_name: self.casm_builder.layout.name.clone(),
                                reason: format!("Type not found for slot {:?}", slot),
                                operation_context: "loop initialization".to_string(),
                            }
                        })?;
                        self.casm_builder.assign(*slot, *value, mir_type, None)?;
                    }

                    // Create header label - this is where continues jump to
                    let header_label = self.casm_builder.emit_new_label_name(".loop_header");
                    self.casm_builder.emit_add_label(Label {
                        name: header_label.clone(),
                        address: None,
                    });

                    self.loop_stack.push(ActiveLoop {
                        header_label: header_label.clone(),
                        header_slots: header_slots.clone(),
                    });

                    // Enter a new value scope for the loop body to avoid ValueOrigin collisions
                    self.push_scope();

                    // Map the sub-DAG's Inputs node (node 0) to load from header slots
                    for (output_idx, slot_id) in header_slots.iter().enumerate() {
                        self.insert_value(
                            ValueOrigin {
                                node: 0,
                                output_idx: output_idx as u32,
                            },
                            *slot_id,
                        );
                    }

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

    /// Store branch values to the appropriate stack slots before jumping
    pub(super) fn store_to_label_slots(
        &mut self,
        target: &BreakTarget,
        branch_values: &[Value],
    ) -> Result<(), DagToCasmError> {
        // If there are no values to store, nothing to do
        if branch_values.is_empty() {
            return Ok(());
        }

        match &target.kind {
            TargetType::Label(label_id) => {
                // Get slots that were pre-allocated in first pass
                let slots = self
                    .label_slots
                    .get(label_id)
                    .ok_or_else(|| DagToCasmError::InvalidControlFlow {
                        function_name: self.casm_builder.layout.name.clone(),
                        reason: format!(
                            "Label {} has no allocated slots but {} values need to be stored",
                            self.label_names
                                .get(label_id)
                                .map(|s| s.as_str())
                                .unwrap_or("?"),
                            branch_values.len()
                        ),
                        operation_context: "storing to label slots".to_string(),
                    })?
                    .clone();

                // Store each value to its corresponding slot
                for (i, (slot, value)) in slots.iter().zip(branch_values.iter()).enumerate() {
                    let mir_type = self.value_types.get(slot).ok_or_else(|| {
                        DagToCasmError::InvalidControlFlow {
                            function_name: self.casm_builder.layout.name.clone(),
                            reason: format!("Type not found for slot {:?} at index {}", slot, i),
                            operation_context: "storing to label slots".to_string(),
                        }
                    })?;
                    self.casm_builder.assign(*slot, *value, mir_type, None)?;
                }
                Ok(())
            }
            TargetType::FunctionOrLoop => {
                // Store to loop header slots
                let d = target.depth as usize;
                if !self.loop_stack.is_empty() && d < self.loop_stack.len() {
                    let idx = self.loop_stack.len() - 1 - d;
                    let header_slots = self.loop_stack[idx].header_slots.clone();

                    // Store each value to its corresponding loop slot
                    for (i, (slot, value)) in
                        header_slots.iter().zip(branch_values.iter()).enumerate()
                    {
                        let mir_type = self.value_types.get(slot).ok_or_else(|| {
                            DagToCasmError::InvalidControlFlow {
                                function_name: self.casm_builder.layout.name.clone(),
                                reason: format!(
                                    "Type not found for loop slot {:?} at index {}",
                                    slot, i
                                ),
                                operation_context: "storing to loop slots".to_string(),
                            }
                        })?;
                        self.casm_builder.assign(*slot, *value, mir_type, None)?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Resolve a WASM break target to a MIR BasicBlockId
    pub(super) fn resolve_break_target(
        &mut self,
        node_idx: usize,
        node: &Node,
        target: &BreakTarget,
    ) -> Result<SolvedBreakTarget, DagToCasmError> {
        match (&target.kind, target.depth) {
            (TargetType::Label(label_id), _depth) => {
                // Direct jump to a label at current scope
                self.label_names.get(label_id).map_or_else(
                    || {
                        Err(DagToCasmError::InvalidControlFlow {
                            function_name: self.casm_builder.layout.name.clone(),
                            reason: format!("Label {} not found in label_map", label_id),
                            operation_context: "resolving break target".to_string(),
                        })
                    },
                    |label| Ok(SolvedBreakTarget::Label(label.clone())),
                )
            }

            (TargetType::FunctionOrLoop, depth) => {
                // If inside a loop, this is a continue to the appropriate loop header.
                // depth == 0 => current loop, depth > 0 => outer loops
                let d = depth as usize;
                if !self.loop_stack.is_empty() && d < self.loop_stack.len() {
                    let idx = self.loop_stack.len() - 1 - d;
                    let loop_info = &self.loop_stack[idx];
                    Ok(SolvedBreakTarget::Label(loop_info.header_label.clone()))
                } else if d >= self.loop_stack.len() && !self.loop_stack.is_empty() {
                    return Err(DagToCasmError::LoopDepthError {
                        function_name: self.casm_builder.layout.name.clone(),
                        node_idx,
                        requested_depth: depth,
                        available_depth: self.loop_stack.len(),
                    });
                } else {
                    let return_values: Vec<Value> = node
                        .inputs
                        .iter()
                        .map(|input| self.get_input_value(input))
                        .collect::<Result<_, _>>()?;

                    Ok(SolvedBreakTarget::Return(return_values))
                }
            }
        }
    }

    /// Extract branch values from a node, excluding conditional inputs
    pub(super) fn get_branch_values(&self, node: &Node) -> Result<Vec<Value>, DagToCasmError> {
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
}
