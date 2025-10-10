use super::DagToCasmError;
use cairo_m_compiler_codegen::{CasmBuilder, FunctionLayout};
use cairo_m_compiler_mir::{MirType, Value, ValueId};
use std::collections::HashMap;
use womir::loader::dag::ValueOrigin;

/// Context for converting a single function DAG to CASM
pub struct DagToCasmContext {
    pub(crate) casm_builder: CasmBuilder,
    /// Maps label IDs to their allocated stack slots for incoming values
    pub(crate) label_names: HashMap<u32, String>,
    pub(crate) label_slots: HashMap<u32, Vec<ValueId>>,
    pub(crate) loop_stack: Vec<ActiveLoop>,
    pub(crate) next_value_id: usize,
    pub(crate) value_types: HashMap<ValueId, MirType>,
    pub(crate) value_maps: Vec<HashMap<ValueOrigin, ValueId>>,
}

/// Information about an active loop during lowering
pub struct ActiveLoop {
    pub(crate) header_label: String,
    /// Stack slots allocated for loop-carried values
    pub(crate) header_slots: Vec<ValueId>,
}

impl DagToCasmContext {
    pub(crate) fn new(func_layout: FunctionLayout, label_counter: usize) -> Self {
        Self {
            casm_builder: CasmBuilder::new(func_layout, label_counter),
            label_names: HashMap::new(),
            label_slots: HashMap::new(),
            loop_stack: Vec::new(),
            next_value_id: 0,
            value_types: HashMap::new(),
            value_maps: vec![HashMap::new()], // Initialize with one scope for function-level values
        }
    }

    /// Generates a new unique value ID with type information
    pub fn new_typed_value_id(&mut self, mir_type: MirType) -> ValueId {
        self.next_value_id += 1;
        let id = ValueId::new(self.next_value_id);
        self.value_types.insert(id, mir_type);
        let _ = self.casm_builder.layout.allocate_local(id, 2);
        id
    }

    /// Generates a new unique value ID with type information, without allocating
    pub fn new_value_id_for_type(&mut self, mir_type: MirType) -> ValueId {
        self.next_value_id += 1;
        let id = ValueId::new(self.next_value_id);
        self.value_types.insert(id, mir_type);
        id
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

    /// Get MIR value for a WASM ValueOrigin
    pub(crate) fn get_input_value(
        &self,
        value_origin: &ValueOrigin,
    ) -> Result<Value, DagToCasmError> {
        self.get_value(value_origin).map_or_else(
            || {
                let available_count = self.value_maps.iter().map(|map| map.len()).sum::<usize>();
                Err(DagToCasmError::ValueMappingError {
                    function_name: self.casm_builder.layout.name.clone(),
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
}
