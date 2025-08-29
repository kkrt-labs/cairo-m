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

use cairo_m_common::instruction::INSTRUCTION_MAX_SIZE;
use cairo_m_common::{Instruction, InstructionError};
use cairo_m_compiler_mir::BasicBlockId;
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use thiserror::Error;

pub mod backend;
pub mod builder;
pub mod db;
pub mod generator;
pub mod layout;
pub mod mir_passes;

// Test support utilities (only compiled for tests)
#[cfg(test)]
pub mod test_support;

// Re-export main components
pub use backend::{compile_module, validate_for_casm};
pub use builder::CasmBuilder;
pub use db::{compile_project as db_compile_project, CodegenDb};
pub use generator::CodeGenerator;
pub use layout::FunctionLayout;

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
    /// The opcode number for this instruction. Always matches the inner instruction kind.
    pub opcode: u32,
    /// Vector of operands (can be literals or label references). For typed instructions,
    /// this is populated from the original integer operands for debug/analysis convenience.
    pub operands: Vec<Operand>,
    /// Human-readable comment for debugging
    pub comment: Option<String>,
    /// If present, a fully-typed instruction from `cairo_m_common`.
    /// When set, this takes precedence during `build()`.
    typed: Option<Instruction>,
}

impl InstructionBuilder {
    /// Construct a pending instruction from opcode, operands and optional comment.
    pub const fn new_with(opcode: u32, operands: Vec<Operand>, comment: Option<String>) -> Self {
        Self {
            opcode,
            operands,
            comment,
            typed: None,
        }
    }

    /// Construct a ready instruction directly from a typed Instruction and optional comment.
    pub fn from_instr(instruction: Instruction, comment: Option<String>) -> Self {
        let mut ib: Self = instruction.into();
        ib.comment = comment;
        ib
    }
    /// Create a new CASM instruction
    pub const fn new(opcode: u32) -> Self {
        Self {
            opcode,
            operands: Vec::new(),
            comment: None,
            typed: None,
        }
    }

    pub(crate) fn build(&self) -> Instruction {
        if let Some(instr) = &self.typed {
            return *instr;
        }

        // Fallback: build from opcode + literal operands (legacy path for label-bearing instrs)
        let mut values = SmallVec::<[M31; INSTRUCTION_MAX_SIZE]>::new();
        values.push(M31::from(self.opcode));

        for op in &self.operands {
            match op {
                Operand::Literal(val) => values.push(M31::from(*val)),
                Operand::Label(label) => panic!("Unresolved label in build(): {}", label),
            }
        }

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
    pub(crate) fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub(crate) const fn get_typed_instruction(&self) -> Option<&Instruction> {
        self.typed.as_ref()
    }

    /// Get the first operand
    pub(crate) fn op0(&self) -> Option<i32> {
        self.operands.first().and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Get the second operand
    pub(crate) fn op1(&self) -> Option<i32> {
        self.operands.get(1).and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Get the third operand
    pub(crate) fn op2(&self) -> Option<i32> {
        self.operands.get(2).and_then(|op| match op {
            Operand::Literal(value) => Some(*value),
            _ => None,
        })
    }

    /// Convert to CASM assembly string
    pub(crate) fn to_asm(&self) -> String {
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
    pub(crate) fn for_block(function_name: &str, block_id: BasicBlockId) -> Self {
        Self::new(format!("{function_name}_{block_id:?}"))
    }

    /// Create a label for a function
    pub(crate) fn for_function(function_name: &str) -> Self {
        Self::new(function_name.to_string())
    }
}

/// Errors that can occur during code generation
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodegenError {
    /// Invalid MIR structure
    #[error("Invalid MIR: {0}")]
    InvalidMir(String),
    /// Missing function or block
    #[error("Missing target: {0}")]
    MissingTarget(String),
    /// Unsupported instruction
    #[error("Unsupported instruction: {0}")]
    UnsupportedInstruction(String),
    /// Layout calculation error
    #[error("Layout error: {0}")]
    LayoutError(String),
    /// Unresolved label reference
    #[error("Unresolved label: {0}")]
    UnresolvedLabel(String),
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
    /// Instruction error
    #[error("Instruction error: {0}")]
    Instruction(#[from] InstructionError),
}

/// Result type for codegen operations
pub type CodegenResult<T> = Result<T, CodegenError>;

impl std::fmt::Display for InstructionBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_asm())
    }
}

impl From<Instruction> for InstructionBuilder {
    fn from(instr: Instruction) -> Self {
        // Extract opcode and numeric operands for debug/analysis convenience
        let opcode = instr.opcode_value();
        let operands = instr
            .operands()
            .into_iter()
            .map(|m| Operand::Literal(m.0 as i32))
            .collect();
        Self {
            opcode,
            operands,
            comment: None,
            typed: Some(instr),
        }
    }
}
