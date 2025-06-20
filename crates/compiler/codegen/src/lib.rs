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

use cairo_m_common::{Instruction, Opcode, Program};
use cairo_m_compiler_mir::{BasicBlockId, MirModule};

pub mod builder;
pub mod db;
pub mod generator;
pub mod layout;

// Re-export main components
pub use builder::CasmBuilder;
pub use db::{codegen_errors, codegen_mir_module, compile_module as db_compile_module, CodegenDb};
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
    /// First offset operand (fp-relative)
    pub off0: Option<i32>,
    /// Second offset operand (fp-relative)
    pub off1: Option<i32>,
    /// Third offset operand (fp-relative)
    pub off2: Option<i32>,
    /// Operand that can be either a literal or label reference
    pub operand: Option<Operand>,
    /// Human-readable comment for debugging
    pub comment: Option<String>,
}

impl InstructionBuilder {
    /// Create a new CASM instruction
    pub const fn new(opcode: u32) -> Self {
        Self {
            opcode,
            off0: None,
            off1: None,
            off2: None,
            operand: None,
            comment: None,
        }
    }

    pub fn build(&self) -> Instruction {
        Instruction {
            opcode: Opcode::from_u32(self.opcode)
                .unwrap_or_else(|| panic!("Invalid opcode: {}", self.opcode)),
            operands: [
                self.op0().unwrap_or(0).into(),
                self.op1().unwrap_or(0).into(),
                self.op2().unwrap_or(0).into(),
            ],
        }
    }

    /// Set the first offset
    pub const fn with_off0(mut self, off0: i32) -> Self {
        self.off0 = Some(off0);
        self
    }

    /// Set the second offset
    pub const fn with_off1(mut self, off1: i32) -> Self {
        self.off1 = Some(off1);
        self
    }

    /// Set the third offset
    pub const fn with_off2(mut self, off2: i32) -> Self {
        self.off2 = Some(off2);
        self
    }

    /// Set the operand (replaces with_imm)
    pub fn with_operand(mut self, operand: Operand) -> Self {
        self.operand = Some(operand);
        self
    }

    /// Set the immediate value (convenience method)
    pub fn with_imm(mut self, imm: i32) -> Self {
        self.operand = Some(Operand::Literal(imm));
        self
    }

    /// Set a label operand (convenience method)
    pub fn with_label(mut self, label: String) -> Self {
        self.operand = Some(Operand::Label(label));
        self
    }

    /// Set a comment
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Get the immediate value if this instruction has a literal operand
    pub const fn imm(&self) -> Option<i32> {
        match &self.operand {
            Some(Operand::Literal(value)) => Some(*value),
            _ => None,
        }
    }

    /// Get the first operand
    pub const fn op0(&self) -> Option<i32> {
        if let Some(off0) = self.off0 {
            Some(off0)
        } else {
            self.imm()
        }
    }

    /// Get the second operand
    pub const fn op1(&self) -> Option<i32> {
        if let Some(off1) = self.off1 {
            Some(off1)
        } else if self.off0.is_some() {
            self.imm()
        } else {
            None
        }
    }

    /// Get the third operand
    pub const fn op2(&self) -> Option<i32> {
        if let Some(off2) = self.off2 {
            Some(off2)
        } else if self.off0.is_some() && self.off1.is_some() {
            self.imm()
        } else {
            None
        }
    }

    /// Get the third operand
    /// Convert to CASM assembly string
    pub fn to_asm(&self) -> String {
        let mut parts = vec![self.opcode.to_string()];

        if let Some(op0) = self.op0() {
            parts.push(op0.to_string());
        } else {
            parts.push("_".to_string());
        }
        if let Some(op1) = self.op1() {
            parts.push(op1.to_string());
        } else {
            parts.push("_".to_string());
        }
        if let Some(op2) = self.op2() {
            parts.push(op2.to_string());
        } else {
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
        let opcode = format!("{:#02x}", self.opcode);
        let op0 = self
            .op0()
            .map(|off| format!("{off:#02x}"))
            .unwrap_or_else(|| "0x00".to_string());
        let op1 = self
            .op1()
            .map(|off| format!("{off:#02x}"))
            .unwrap_or_else(|| "0x00".to_string());
        let op2 = self
            .op2()
            .map(|off| format!("{off:#02x}"))
            .unwrap_or_else(|| "0x00".to_string());
        vec![opcode, op0, op1, op2]
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
