use std::collections::HashMap;

use cairo_m_compiler_mir::instruction::{Instruction, InstructionKind};
use cairo_m_compiler_mir::{BasicBlock, BasicBlockId, MirFunction, MirType, Value, ValueId};
use womir::loader::dag::ValueOrigin;

use super::DagToMirError;

/// Context for converting a single function DAG to MIR
pub struct DagToMirContext {
    pub(crate) mir_function: MirFunction,
    pub(crate) value_maps: Vec<HashMap<ValueOrigin, ValueId>>,
    pub(crate) label_map: HashMap<u32, BasicBlockId>,
    pub(crate) current_block_id: Option<BasicBlockId>,
    pub(crate) current_source_block: Option<BasicBlockId>,
    pub(crate) label_phi_nodes: HashMap<u32, Vec<ValueId>>,
    pub(crate) loop_stack: Vec<ActiveLoop>,
    pub(crate) deferred_phi_operands: Vec<(BasicBlockId, ValueId, BasicBlockId, Value)>,
}

/// Information about an active loop during lowering
pub struct ActiveLoop {
    pub(crate) header_block: BasicBlockId,
    pub(crate) header_phi_nodes: Vec<ValueId>,
}

impl DagToMirContext {
    pub(crate) fn new(func_name: String) -> Self {
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

    pub(crate) fn get_current_block(&mut self) -> Result<&mut BasicBlock, DagToMirError> {
        let block_id = self
            .current_block_id
            .ok_or_else(|| DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: "No current block set - invalid state".to_string(),
                operation_context: "attempting to get current block".to_string(),
            })?;

        self.block_mut(block_id)
    }

    pub(crate) const fn set_current_block(&mut self, block_id: BasicBlockId) {
        self.current_source_block = self.current_block_id;
        self.current_block_id = Some(block_id);
    }

    pub(crate) fn block_mut(
        &mut self,
        block_id: BasicBlockId,
    ) -> Result<&mut BasicBlock, DagToMirError> {
        self.mir_function
            .basic_blocks
            .get_mut(block_id)
            .ok_or_else(|| DagToMirError::InvalidControlFlow {
                function_name: self.mir_function.name.clone(),
                reason: format!("Block {:?} does not exist", block_id),
                operation_context: "attempting to access basic block".to_string(),
            })
    }

    pub(crate) fn insert_value(&mut self, origin: ValueOrigin, value_id: ValueId) {
        if let Some(map) = self.value_maps.last_mut() {
            map.insert(origin, value_id);
        }
    }

    pub(crate) fn get_value(&self, origin: &ValueOrigin) -> Option<ValueId> {
        for map in self.value_maps.iter().rev() {
            if let Some(v) = map.get(origin) {
                return Some(*v);
            }
        }
        None
    }

    pub(crate) fn push_scope(&mut self) {
        self.value_maps.push(HashMap::new());
    }
    pub(crate) fn pop_scope(&mut self) {
        self.value_maps.pop();
    }

    /// Add a deferred phi operand to be processed later
    pub(crate) fn add_deferred_phi_operand(
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
    pub(crate) fn finalize_phi_nodes(&mut self) -> Result<(), DagToMirError> {
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
            let block =
                self.block_mut(block_id)
                    .map_err(|_| DagToMirError::InvalidControlFlow {
                        function_name: function_name.clone(),
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

    /// Get MIR value for a WASM ValueOrigin
    pub(crate) fn get_input_value(
        &self,
        value_origin: &ValueOrigin,
    ) -> Result<Value, DagToMirError> {
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

    /// Utility: create empty phi nodes for a list of MIR types in a block, returning their ValueIds
    pub(crate) fn create_phi_nodes(
        &mut self,
        block: BasicBlockId,
        types: &[MirType],
    ) -> Vec<ValueId> {
        let mut ids = Vec::with_capacity(types.len());
        for ty in types {
            let id = self.mir_function.new_typed_value_id(ty.clone());
            let phi = Instruction::empty_phi(id, ty.clone());
            // Safe: block allocated by caller
            self.mir_function
                .get_basic_block_mut(block)
                .unwrap()
                .push_phi_front(phi);
            ids.push(id);
        }
        ids
    }
}
