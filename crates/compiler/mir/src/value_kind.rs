use crate::ValueId;
use rustc_hash::FxHashMap;

/// Distinguishes whether a ValueId represents a memory address or a computed value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// A computed value held in an SSA register (immutable)
    Value,
    /// A memory address/location (can be loaded from or stored to)
    Address,
    /// Function parameter (always a value, never an address)
    Parameter,
}

/// Tracks the kind of each ValueId in a function
/// This is a temporary solution during migration to proper SSA
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValueKindTracker {
    kinds: FxHashMap<ValueId, ValueKind>,
}

impl ValueKindTracker {
    pub fn new() -> Self {
        Self {
            kinds: FxHashMap::default(),
        }
    }

    /// Register a new value with its kind
    pub fn register(&mut self, id: ValueId, kind: ValueKind) {
        self.kinds.insert(id, kind);
    }

    /// Get the kind of a value
    pub fn get(&self, id: ValueId) -> Option<ValueKind> {
        self.kinds.get(&id).copied()
    }

    /// Check if a value is an address that needs loading
    pub fn needs_load(&self, id: ValueId) -> bool {
        matches!(self.get(id), Some(ValueKind::Address))
    }

    /// Check if a value can be used directly (no load needed)
    pub fn is_value(&self, id: ValueId) -> bool {
        matches!(self.get(id), Some(ValueKind::Value | ValueKind::Parameter))
    }
}
