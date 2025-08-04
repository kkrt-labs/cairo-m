use std::cell::RefCell;

use stwo_prover::core::fields::m31::M31;

use crate::state::MemoryEntry;
use crate::State;

#[derive(Debug, Default, Clone)]
pub struct Segment {
    pub initial_memory: Vec<M31>,
    pub memory_trace: RefCell<Vec<MemoryEntry>>,
    pub trace: Vec<State>,
}

impl Segment {
    /// Serializes a segment's trace to a byte vector.
    ///
    /// Each trace entry consists of `fp` and `pc` values, both `u32`.
    /// This function serializes the trace as a flat sequence of bytes.
    /// For each entry, it first serializes `fp` into little-endian bytes,
    /// followed by the little-endian bytes of `pc`.
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized trace data for the segment.
    pub fn serialize_segment_trace(&self) -> Vec<u8> {
        // Each entry has 2 u32 values (fp and pc), each u32 is 4 bytes
        let capacity = self.trace.len() * 2 * 4;
        let mut result = Vec::with_capacity(capacity);

        for entry in &self.trace {
            result.extend_from_slice(&entry.fp.0.to_le_bytes());
            result.extend_from_slice(&entry.pc.0.to_le_bytes());
        }

        result
    }

    /// Serializes a segment's memory trace to a byte vector.
    ///
    /// Each memory entry consists of an address (1 `u32`) and a value (1 `u32`).
    /// This function serializes the memory trace as a flat sequence of bytes.
    /// For each entry, it first serializes the address into little-endian bytes,
    /// followed by the little-endian bytes of the `u32` component of the M31 value.
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized memory trace data for the segment.
    pub fn serialize_segment_memory_trace(&self) -> Vec<u8> {
        let memory_trace = self.memory_trace.borrow();
        // Each entry has 2 u32 values, each u32 is 4 bytes
        let capacity = memory_trace.len() * 2 * 4;
        let mut result = Vec::with_capacity(capacity);

        for entry in memory_trace.iter() {
            result.extend_from_slice(&entry.addr.0.to_le_bytes());
            result.extend_from_slice(&entry.value.0.to_le_bytes());
        }

        result
    }
}
