use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Instruction;

/// One parameter or return value in the ABI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbiSlot {
    /// Name of the parameter or return value (empty if the compiler had no debug name)
    pub name: String,
    /// Number of memory slots this value occupies (1 for felt/bool/ptr, 2 for u32, etc.)
    pub slots: usize,
}

/// Information about a function entrypoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntrypointInfo {
    /// The program counter (instruction index) where the function starts
    pub pc: usize,
    /// Information about each parameter
    pub params: Vec<AbiSlot>,
    /// Information about each return value
    pub returns: Vec<AbiSlot>,
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
