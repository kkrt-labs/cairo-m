//! Instructions for the Cairo M VM.
//!
//! Cairo M instructions use variable-size encoding based on their type:
//! - The first M31 is always the opcode
//! - The remaining M31 elements are instruction-specific operands
//! - Instructions range from 1 M31 (Ret) to 5 M31 (U32StoreAddFpImm)
//!
//! Instructions are stored in memory as QM31 values (4 M31 elements each).
//! When an instruction doesn't fill a complete QM31, it's padded with zeros.
//!
//! ## Instruction Format
//!
//! Each instruction variant is defined with named fields in the Instruction enum.
//! For example:
//! - `StoreImm { imm: M31, dst_off: M31 }` - 3 M31 total (opcode + 2 operands)
//! - `StoreAddFpFp { src0_off: M31, src1_off: M31, dst_off: M31 }` - 4 M31 total
//! - `Ret {}` - 1 M31 total (just the opcode)

use assert::assert_eq_fp_imm;
use cairo_m_common::instruction::*;
use cairo_m_common::{Instruction, State};

use crate::vm::instructions::call::*;
use crate::vm::instructions::jnz::*;
use crate::vm::instructions::jump::*;
use crate::vm::instructions::print::*;
use crate::vm::instructions::store::*;
use crate::vm::{Memory, MemoryError};

pub mod assert;
pub mod call;
pub mod jnz;
pub mod jump;
pub mod print;
pub mod store;

/// Error type for instruction execution
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstructionExecutionError {
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
    #[error("Instruction error: {0}")]
    Instruction(#[from] InstructionError),
    #[error("Invalid operand: {0}")]
    InvalidOperand(String),
    #[error("Invalid instruction type for handler")]
    InvalidInstructionType,
}

pub type InstructionFn =
    fn(&mut Memory, State, &Instruction) -> Result<State, InstructionExecutionError>;

/// Maps an instruction to its corresponding instruction handler function.
///
/// ## Arguments
///
/// * `instruction` - The instruction to map to a function.
///
/// ## Returns
///
/// Returns an [`InstructionFn`] - a function pointer that can execute the instruction
/// when called with memory, state, and instruction arguments.
///
/// ## Errors
///
/// Returns [`InstructionError::InvalidOpcode`] if the provided instruction does not
/// correspond to any implemented handler.
pub fn instruction_to_fn(instruction: Instruction) -> Result<InstructionFn, InstructionError> {
    let f = match instruction {
        Instruction::StoreAddFpFp { .. } => store_add_fp_fp,
        Instruction::StoreAddFpImm { .. } => store_add_fp_imm,
        Instruction::StoreSubFpFp { .. } => store_sub_fp_fp,
        Instruction::StoreDoubleDerefFp { .. } => store_double_deref_fp,
        Instruction::StoreDoubleDerefFpFp { .. } => store_double_deref_fp_fp,
        Instruction::StoreImm { .. } => store_imm,
        Instruction::StoreFramePointer { .. } => store_fp_imm,
        Instruction::StoreMulFpFp { .. } => store_mul_fp_fp,
        Instruction::StoreMulFpImm { .. } => store_mul_fp_imm,
        Instruction::StoreDivFpFp { .. } => store_div_fp_fp,
        Instruction::CallAbsImm { .. } => call_abs_imm,
        Instruction::Ret { .. } => ret,
        Instruction::JmpAbsImm { .. } => jmp_abs_imm,
        Instruction::JmpRelImm { .. } => jmp_rel_imm,
        Instruction::JnzFpImm { .. } => jnz_fp_imm,
        Instruction::U32StoreAddFpFp { .. } => u32_store_add_fp_fp,
        Instruction::U32StoreSubFpFp { .. } => u32_store_sub_fp_fp,
        Instruction::U32StoreMulFpFp { .. } => u32_store_mul_fp_fp,
        Instruction::U32StoreDivRemFpFp { .. } => u32_store_div_rem_fp_fp,
        Instruction::U32StoreAddFpImm { .. } => u32_store_add_fp_imm,
        Instruction::U32StoreMulFpImm { .. } => u32_store_mul_fp_imm,
        Instruction::U32StoreDivRemFpImm { .. } => u32_store_div_rem_fp_imm,
        Instruction::U32StoreImm { .. } => u32_store_imm,
        Instruction::U32StoreEqFpFp { .. } => u32_store_eq_fp_fp,
        Instruction::U32StoreLtFpFp { .. } => u32_store_lt_fp_fp,
        Instruction::U32StoreEqFpImm { .. } => u32_store_eq_fp_imm,
        Instruction::U32StoreLtFpImm { .. } => u32_store_lt_fp_imm,
        Instruction::U32StoreAndFpFp { .. } => u32_store_and_fp_fp,
        Instruction::U32StoreOrFpFp { .. } => u32_store_or_fp_fp,
        Instruction::U32StoreXorFpFp { .. } => u32_store_xor_fp_fp,
        Instruction::U32StoreAndFpImm { .. } => u32_store_and_fp_imm,
        Instruction::U32StoreOrFpImm { .. } => u32_store_or_fp_imm,
        Instruction::U32StoreXorFpImm { .. } => u32_store_xor_fp_imm,
        Instruction::StoreToDoubleDerefFpImm { .. } => store_to_double_deref_fp_imm,
        Instruction::StoreToDoubleDerefFpFp { .. } => store_to_double_deref_fp_fp,
        Instruction::PrintM31 { .. } => print_m31,
        Instruction::PrintU32 { .. } => print_u32,
        Instruction::StoreLeFpImm { .. } => store_le_fp_imm,
        Instruction::AssertEqFpImm { .. } => assert_eq_fp_imm,
    };
    Ok(f)
}

#[cfg(test)]
mod print_tests;
