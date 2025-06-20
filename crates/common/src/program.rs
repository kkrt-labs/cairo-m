use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Instruction;

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
    /// Entrypoint names mapped to instruction indices
    pub entrypoints: HashMap<String, usize>,
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
        entrypoints: HashMap<String, usize>,
        metadata: ProgramMetadata,
    ) -> Self {
        Self {
            instructions,
            entrypoints,
            metadata,
        }
    }

    /// Get the entry point address for a given function name
    pub fn get_entrypoint(&self, name: &str) -> Option<usize> {
        self.entrypoints.get(name).copied()
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
