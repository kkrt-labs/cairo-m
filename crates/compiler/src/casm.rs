//! Cairo-M Assembly (CASM) Module
//!
//! This module defines the Cairo-M assembly language representation and encoding.
//! CASM is a low-level intermediate representation used between the compiler and the virtual machine.
//!
//! Instruction Format:
//! - Each instruction consists of an opcode and up to three arguments
//! - Arguments are encoded as signed offsets from 0x8000 to allow for both positive and negative
//!   frame pointer offsets
//! - Labels are used for control flow and function calls
//!
//! Instruction Categories:
//! - Memory Operations: Move values between frame pointer offsets or load immediate values
//! - Arithmetic: Add, subtract, and multiply operations with frame pointer offsets or immediates
//! - Control Flow: Absolute and relative jumps, conditional jumps, and function calls
//! - Labels: Mark locations in the code for jumps and calls

use std::fmt::{self, Debug, Display};

/// Represents the different types of Cairo-M assembly instructions.
///
/// Each variant corresponds to a specific operation that can be performed by the virtual machine.
/// The instruction types are organized into categories:
/// - Memory operations (MovFpFp, MovFpImm)
/// - Arithmetic operations (AddFpFp, AddFpImm, etc.)
/// - Control flow (JmpAbs, JmpRel, etc.)
/// - Function calls (CallRel, CallAbs)
/// - Labels and special instructions (Label, Ret)
#[derive(Clone)]
pub enum CasmInstructionType {
    MovFpFp,
    MovFpImm,
    AddFpFp,
    AddFpImm,
    SubFpFp,
    SubFpImm,
    MulFpFp,
    MulFpImm,
    JmpAbs,
    JmpRel,
    JmpAbsIfNeq,
    JmpRelIfNeq,
    CallRel,
    CallAbs,
    Label,
    JmpLabel,
    JmpLabelIfNeq,
    CallLabel,
    Ret,
}

impl CasmInstructionType {
    /// Returns the numeric opcode for the instruction type.
    ///
    /// The opcode is used in the binary encoding of instructions.
    /// Label instructions cannot be lowered to bytecode and will panic.
    pub fn get_opcode(&self) -> u32 {
        match self {
            CasmInstructionType::MovFpFp => 0,
            CasmInstructionType::MovFpImm => 1,
            CasmInstructionType::AddFpFp => 2,
            CasmInstructionType::AddFpImm => 3,
            CasmInstructionType::SubFpFp => 4,
            CasmInstructionType::SubFpImm => 5,
            CasmInstructionType::MulFpFp => 6,
            CasmInstructionType::MulFpImm => 7,
            CasmInstructionType::JmpAbs => 8,
            CasmInstructionType::JmpRel => 9,
            CasmInstructionType::JmpAbsIfNeq => 10,
            CasmInstructionType::JmpRelIfNeq => 11,
            CasmInstructionType::CallRel => 12,
            CasmInstructionType::CallAbs => 13,
            CasmInstructionType::Ret => 14,
            _ => panic!("Label instructions cannot be lowered to bytecode."),
        }
    }
}

impl Display for CasmInstructionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CasmInstructionType::MovFpFp => write!(f, "mov_fp_fp"),
            CasmInstructionType::MovFpImm => write!(f, "mov_fp_imm"),
            CasmInstructionType::AddFpFp => write!(f, "add_fp_fp"),
            CasmInstructionType::AddFpImm => write!(f, "add_fp_imm"),
            CasmInstructionType::SubFpFp => write!(f, "sub_fp_fp"),
            CasmInstructionType::SubFpImm => write!(f, "sub_fp_imm"),
            CasmInstructionType::MulFpFp => write!(f, "mul_fp_fp"),
            CasmInstructionType::MulFpImm => write!(f, "mul_fp_imm"),
            CasmInstructionType::JmpAbs => write!(f, "jmp_abs"),
            CasmInstructionType::JmpRel => write!(f, "jmp_rel"),
            CasmInstructionType::JmpAbsIfNeq => write!(f, "jmp_abs_if_neq"),
            CasmInstructionType::JmpRelIfNeq => write!(f, "jmp_rel_if_neq"),
            CasmInstructionType::CallRel => write!(f, "call_rel"),
            CasmInstructionType::CallAbs => write!(f, "call_abs"),
            CasmInstructionType::Label => write!(f, "label"),
            CasmInstructionType::Ret => write!(f, "ret"),
            CasmInstructionType::JmpLabel => write!(f, "jmp_label"),
            CasmInstructionType::JmpLabelIfNeq => write!(f, "jmp_label_if_neq"),
            CasmInstructionType::CallLabel => write!(f, "call_label"),
        }
    }
}

/// Represents a complete Cairo-M assembly instruction.
///
/// Each instruction consists of:
/// - An instruction type
/// - An optional label (for control flow and function calls)
/// - Up to three arguments (encoded as signed offsets from 0x8000)
#[derive(Clone)]
pub struct CasmInstruction {
    pub instruction_type: CasmInstructionType,
    pub label: Option<String>,
    pub arg0: i32,
    pub arg1: i32,
    pub arg2: i32,
}

impl CasmInstruction {
    /// Converts the instruction to its binary representation.
    ///
    /// Returns a tuple of four 32-bit words:
    /// - First word: Opcode
    /// - Remaining words: Arguments encoded as signed offsets from 0x8000
    pub fn to_bytes(&self) -> (u32, u32, u32, u32) {
        let opcode = self.instruction_type.get_opcode();
        let arg0 = (0x8000 + self.arg0) as u32;
        let arg1 = (0x8000 + self.arg1) as u32;
        let arg2 = (0x8000 + self.arg2) as u32;
        (opcode, arg0, arg1, arg2)
    }
}

impl Display for CasmInstruction {
    /// Formats the instruction as a human-readable string.
    ///
    /// The format varies by instruction type:
    /// - Memory operations: [fp + offset] = value
    /// - Arithmetic: [fp + dst] = [fp + src1] op [fp + src2]
    /// - Control flow: jmp/call target
    /// - Labels: label:
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.instruction_type {
            CasmInstructionType::Label => {
                if let Some(label) = &self.label {
                    write!(f, "{}:", label)
                } else {
                    write!(f, "{}", self.instruction_type)
                }
            }
            CasmInstructionType::MovFpFp => {
                write!(f, "[fp + {}] = [fp + {}]", self.arg0, self.arg1)
            }
            CasmInstructionType::MovFpImm => {
                write!(f, "[fp + {}] = {}", self.arg0, self.arg1)
            }
            CasmInstructionType::AddFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] + [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::AddFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] + {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::SubFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] - [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::SubFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] - {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::MulFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] * [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::MulFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] * {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::CallLabel => {
                if let Some(label) = &self.label {
                    write!(f, "call {};", label)
                } else {
                    write!(f, "call;")
                }
            }
            CasmInstructionType::Ret => {
                write!(f, "ret;")
            }
            CasmInstructionType::JmpLabel => {
                if let Some(label) = &self.label {
                    write!(f, "jmp {};", label)
                } else {
                    write!(f, "jmp;")
                }
            }
            CasmInstructionType::JmpLabelIfNeq => {
                if let Some(label) = &self.label {
                    write!(f, "jmp {} if [fp + {}] != 0;", label, self.arg1)
                } else {
                    write!(f, "jmp if [fp + {}] != 0;", self.arg1)
                }
            }
            CasmInstructionType::JmpAbs => {
                write!(f, "jmp {};", self.arg0)
            }
            CasmInstructionType::JmpRel => {
                write!(f, "jmp {};", self.arg0)
            }
            CasmInstructionType::JmpAbsIfNeq => {
                write!(f, "jmp {} if [fp + {}] != 0;", self.arg0, self.arg1)
            }
            CasmInstructionType::JmpRelIfNeq => {
                write!(f, "jmp {} if [fp + {}] != 0;", self.arg0, self.arg1)
            }
            CasmInstructionType::CallRel => {
                write!(f, "call {};", self.arg0)
            }
            CasmInstructionType::CallAbs => {
                write!(f, "call {};", self.arg0)
            }
        }
    }
}
