use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::qm31::QM31;

use crate::state::MemoryEntry;
use crate::{Instruction, State};

#[derive(Debug, Default, Clone)]
pub struct Segment {
    pub initial_memory: Vec<QM31>,
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
    /// ## Arguments
    ///
    /// * `segment` - The segment to serialize
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized trace data for the segment.
    pub fn serialize_segment_trace(&self) -> Vec<u8> {
        self.trace
            .iter()
            .flat_map(|entry| [entry.fp.0, entry.pc.0])
            .flat_map(u32::to_le_bytes)
            .collect()
    }

    /// Writes the serialized memory trace to binary files, one per segment.
    ///
    /// This function creates a file for each segment with the naming pattern:
    /// `<base_path>_segment_<index>.<extension>`
    ///
    /// Each file starts with the program length, followed by the serialized memory trace
    /// for that segment.
    ///
    /// ## Arguments
    ///
    /// * `path` - The base file path for the binary memory trace files.
    ///
    /// Serializes the memory trace of a single segment into a binary format.
    ///
    /// The binary format consists of a sequence of memory entries, where each entry contains:
    /// - Address (4 bytes, little-endian u32)
    /// - Value (16 bytes, representing QM31 as 4 u32 values in little-endian)
    ///
    /// ## Arguments
    ///
    /// * `segment` - The segment whose memory trace should be serialized
    ///
    /// ## Returns
    ///
    /// A vector of bytes containing the serialized memory trace
    pub fn serialize_segment_memory_trace(&self) -> Vec<u8> {
        let memory_trace = self.memory_trace.borrow();
        memory_trace
            .iter()
            .flat_map(|entry| {
                let mut bytes = Vec::with_capacity(20);
                bytes.extend_from_slice(&entry.addr.0.to_le_bytes());
                // QM31 has two CM31 fields, each CM31 has two M31 fields
                bytes.extend_from_slice(&entry.value.0.0.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.0.1.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.1.0.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.1.1.0.to_le_bytes());
                bytes
            })
            .collect()
    }
}

/// Information about a function entrypoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntrypointInfo {
    /// The program counter (instruction index) where the function starts
    pub pc: usize,
    /// Names of the function arguments
    pub args: Vec<String>,
    /// Number of return values
    pub num_return_values: usize,
}

/// Metadata about the compiled program
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgramMetadata {
    /// Source file name if available
    pub source_file: Option<String>,

    /// Timestamp of compilation
    pub compiled_at: Option<String>,

    /// Compiler version
    pub compiler_version: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A compiled Cairo-M program with instructions and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    /// The program instructions
    pub instructions: Vec<Instruction>,
    /// Entrypoint names mapped to their information
    pub entrypoints: HashMap<String, EntrypointInfo>,
    /// Program metadata
    pub metadata: ProgramMetadata,
}

impl From<Vec<Instruction>> for Program {
    fn from(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            entrypoints: HashMap::new(),
            metadata: ProgramMetadata::default(),
        }
    }
}

impl Program {
    /// Create a new program
    pub const fn new(
        instructions: Vec<Instruction>,
        entrypoints: HashMap<String, EntrypointInfo>,
        metadata: ProgramMetadata,
    ) -> Self {
        Self {
            instructions,
            entrypoints,
            metadata,
        }
    }

    /// Get the full entrypoint information for a given function name
    pub fn get_entrypoint(&self, name: &str) -> Option<&EntrypointInfo> {
        self.entrypoints.get(name)
    }

    /// Get the total number of instructions
    pub const fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if the program is empty
    pub const fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
