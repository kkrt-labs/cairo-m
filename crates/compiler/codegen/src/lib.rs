//! # Cairo-M MIR to CASM Code Generation
//!
//! This crate translates MIR (Mid-level Intermediate Representation) to CASM
//! (Cairo Assembly) instructions. The core challenge is mapping MIR's flexible
//! register-based model onto CASM's rigid fp-relative memory model.
//!
//! ## Architecture
//!
//! The codegen process involves:
//! 1. **Stack Layout Calculation**: Determine fp-relative offsets for all values
//! 2. **Instruction Translation**: Convert MIR instructions to CASM
//! 3. **Control Flow**: Handle jumps, branches, and function calls
//! 4. **Label Resolution**: Two-pass approach for jump targets

#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

use cairo_m_common::{Instruction, Program};
use cairo_m_compiler_mir::{BasicBlockId, MirModule};

pub mod builder;
pub mod db;
pub mod generator;
pub mod layout;

// Re-export main components
pub use builder::CasmBuilder;
pub use db::{CodegenDb, compile_project as db_compile_project};
pub use generator::CodeGenerator;
pub use layout::FunctionLayout;

/// Main entry point for code generation
///
/// Converts a MIR module to a JSON representation of the compiled program
pub fn compile_module(module: &MirModule) -> Result<Program, CodegenError> {
    let mut generator = CodeGenerator::new();
    generator.generate_module(module)?;
    Ok(generator.compile())
}

/// Represents an operand that can be either a literal value or a label reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// A literal immediate value
    Literal(i32),
    /// A label reference that needs to be resolved
    Label(String),
}

impl Operand {
    /// Create a literal operand from an integer
    pub const fn literal(value: i32) -> Self {
        Self::Literal(value)
    }

    /// Create a label operand
    pub const fn label(name: String) -> Self {
        Self::Label(name)
    }
}

/// Represents an instruction being built during code generation.
///
/// This is an intermediate representation that may contain unresolved labels
/// and other builder state before being converted to the final Instruction type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionBuilder {
    /// The opcode number for this instruction
    pub opcode: u32,
    /// Vector of operands (can be literals or label references)
    pub operands: Vec<Operand>,
    /// Human-readable comment for debugging
    pub comment: Option<String>,
}

impl InstructionBuilder {
    /// Create a new CASM instruction
    pub const fn new(opcode: u32) -> Self {
        Self {
            opcode,
            operands: Vec::new(),
            comment: None,
        }
    }

    pub fn build(&self) -> Instruction {
        // For now, we'll panic if we can't convert operands to literals
        // This should only happen for unresolved labels, which should be resolved by this point
        let m31_operands: Vec<stwo_prover::core::fields::m31::M31> = self
            .operands
            .iter()
            .map(|op| match op {
                Operand::Literal(val) => stwo_prover::core::fields::m31::M31::from(*val),
                Operand::Label(label) => panic!("Unresolved label in build(): {}", label),
            })
            .collect();

        // Create the instruction from opcode and operands
        let mut values = vec![stwo_prover::core::fields::m31::M31::from(self.opcode)];
        values.extend(m31_operands);

        Instruction::try_from(values).unwrap_or_else(|e| {
            panic!(
                "Failed to build instruction: {:?}. Opcode: {}, Operands: {:?}",
                e, self.opcode, self.operands
            )
        })
    }

    /// Add an operand to the instruction
    pub fn with_operand(mut self, operand: Operand) -> Self {
        self.operands.push(operand);
        self
    }

    /// Set a comment
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Get the first operand
    pub fn op0(&self) -> Option<i32> {
        self.operands.first().and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Get the second operand
    pub fn op1(&self) -> Option<i32> {
        self.operands.get(1).and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Get the third operand
    pub fn op2(&self) -> Option<i32> {
        self.operands.get(2).and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Convert to CASM assembly string
    pub fn to_asm(&self) -> String {
        let mut parts = vec![self.opcode.to_string()];

        // Add operands to the string
        for operand in &self.operands {
            match operand {
                Operand::Literal(val) => parts.push(val.to_string()),
                Operand::Label(label) => parts.push(format!("@{}", label)),
            }
        }

        // Pad with underscores to reach exactly 4 parts for consistent formatting
        while parts.len() < 4 {
            parts.push("_".to_string());
        }

        let instruction = parts.join(" ");

        if let Some(comment) = &self.comment {
            format!("{instruction:<20} // {comment}")
        } else {
            instruction
        }
    }

    /// Convert asm instruction to a vector of hex strings
    /// Signed offsets are encoded as M31 before being converted to hex.
    pub fn to_hex(&self) -> Vec<String> {
        let mut hex_parts = vec![format!("{:#02x}", self.opcode)];

        for operand in &self.operands {
            match operand {
                Operand::Literal(val) => hex_parts.push(format!("{val:#02x}")),
                Operand::Label(label) => hex_parts.push(format!("@{}", label)),
            }
        }

        // Pad with zeros if needed
        while hex_parts.len() < 4 {
            hex_parts.push("0x00".to_string());
        }

        hex_parts
    }
}

/// Represents a label in the generated code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Unique name for this label
    pub name: String,
    /// Program counter address (filled during label resolution)
    pub address: Option<usize>,
}

impl Label {
    /// Create a new label
    pub const fn new(name: String) -> Self {
        Self {
            name,
            address: None,
        }
    }

    /// Create a label for a basic block
    pub fn for_block(function_name: &str, block_id: BasicBlockId) -> Self {
        Self::new(format!("{function_name}_{block_id:?}"))
    }

    /// Create a label for a function
    pub fn for_function(function_name: &str) -> Self {
        Self::new(function_name.to_string())
    }
}

/// Errors that can occur during code generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    /// Invalid MIR structure
    InvalidMir(String),
    /// Missing function or block
    MissingTarget(String),
    /// Unsupported instruction
    UnsupportedInstruction(String),
    /// Layout calculation error
    LayoutError(String),
    /// Unresolved label reference
    UnresolvedLabel(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMir(msg) => write!(f, "Invalid MIR: {msg}"),
            Self::MissingTarget(msg) => write!(f, "Missing target: {msg}"),
            Self::UnsupportedInstruction(msg) => write!(f, "Unsupported instruction: {msg}"),
            Self::LayoutError(msg) => write!(f, "Layout error: {msg}"),
            Self::UnresolvedLabel(msg) => write!(f, "Unresolved label: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Result type for codegen operations
pub type CodegenResult<T> = Result<T, CodegenError>;

impl std::fmt::Display for InstructionBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_asm())
    }
}
