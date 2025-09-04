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

use cairo_m_common::{Instruction, InstructionError};
use cairo_m_compiler_mir::BasicBlockId;
use thiserror::Error;

pub mod backend;
pub mod builder;
pub mod db;
pub mod generator;
pub mod layout;
pub mod mir_passes;
pub mod passes;

// Test support utilities (only compiled for tests)
#[cfg(test)]
pub mod test_support;

// Re-export main components
pub use backend::{compile_module, validate_for_casm};
pub use builder::CasmBuilder;
pub use db::{compile_project as db_compile_project, CodegenDb};
pub use generator::CodeGenerator;
pub use layout::FunctionLayout;

/// Represents an instruction being built during code generation.
///
/// This is an intermediate representation that may contain unresolved labels
/// and other builder state before being converted to the final Instruction type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionBuilder {
    /// The instruction being built. Jumps with labels are unresolved, and mutated during a label
    /// resolution pass, using the `label` field.
    inner: Instruction,
    /// Label to resolve to a target address. If present, the instruction is unresolved.
    label: Option<String>,
    /// Human-readable comment for debugging
    comment: Option<String>,
}

impl InstructionBuilder {
    /// Construct an instruction builder directly from an Instruction and optional comment.
    pub fn new(instruction: Instruction, comment: Option<String>) -> Self {
        let mut ib: Self = instruction.into();
        ib.comment = comment;
        ib
    }

    /// Finalize this builder into a concrete `Instruction`.
    ///
    /// Returns an error if a label is still unresolved. This avoids panicking
    /// and surfaces misuse as a proper codegen error.
    pub(crate) fn build(&self) -> CodegenResult<Instruction> {
        if let Some(label) = &self.label {
            return Err(CodegenError::UnresolvedLabel(format!(
                "Unresolved label in build(): {label}"
            )));
        }
        Ok(self.inner)
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Add a label that needs to be resolved to the instruction's  target field.
    pub(crate) fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Set a comment
    pub(crate) fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub(crate) const fn inner_instr(&self) -> &Instruction {
        &self.inner
    }

    pub(crate) const fn inner_instr_mut(&mut self) -> &mut Instruction {
        &mut self.inner
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
        let mut parts = vec![];
        for value in self.inner_instr().to_smallvec() {
            parts.push(value.to_string());
        }

        // Pad to reach a fixed part count for consistent formatting
        const MIN_PARTS: usize = 4;
        const PART_PLACEHOLDER: &str = "_";
        while parts.len() < MIN_PARTS {
            parts.push(PART_PLACEHOLDER.to_string());
        }

        let instruction = parts.join(" ");

        if let Some(comment) = &self.comment {
            write!(f, "{instruction:<20} // {comment}")
        } else {
            write!(f, "{instruction}")
        }
    }
}

impl From<Instruction> for InstructionBuilder {
    fn from(instr: Instruction) -> Self {
        // Extract opcode and numeric operands for debug/analysis convenience
        Self {
            label: None,
            comment: None,
            inner: instr,
        }
    }
}
