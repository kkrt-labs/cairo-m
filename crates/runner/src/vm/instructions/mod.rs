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

use cairo_m_common::instruction::*;
use cairo_m_common::{Instruction, State};
use stwo_prover::core::fields::m31::M31;

use crate::vm::instructions::call::*;
use crate::vm::instructions::jnz::*;
use crate::vm::instructions::jump::*;
use crate::vm::instructions::store::*;
use crate::vm::{Memory, MemoryError};

pub mod call;
pub mod jnz;
pub mod jump;
pub mod store;

/// Extracts fields from a specific instruction variant or returns an InvalidOpcode error.
///
/// This macro simplifies instruction decoding by handling the boilerplate of matching
/// and error handling. It automatically dereferences the extracted fields.
///
/// # Panics
/// The macro generates a `return` statement, so it must be used inside a function
/// that returns a `Result<_, InstructionExecutionError>`.
///
/// # Usage
///
/// ## Extracting multiple fields into a tuple:
/// ```ignore
/// let (cond_off, offset) = extract_as!(instruction, JnzFpImm, (cond_off, offset));
/// ```
/// expands to:
/// ```ignore
/// let (cond_off, offset) = match instruction {
///     Instruction::JnzFpImm { cond_off, offset } => (*cond_off, *offset),
///     _ => return Err(InstructionExecutionError::InvalidInstructionType),
/// };
/// ```
///
/// ## Extracting a single field:
/// ```ignore
/// let target = extract_as!(instruction, JmpAbsImm, target);
/// ```
/// expands to:
/// ```ignore
/// let target = match instruction {
///     Instruction::JmpAbsImm { target } => *target,
///     _ => return Err(InstructionExecutionError::InvalidInstructionType),
/// };
/// ```
#[macro_export]
macro_rules! extract_as {
    // Case 1: Extracting multiple fields into a tuple.
    // e.g., extract_as!(instruction, JnzFpImm, (cond_off, offset))
    ($instruction:expr, $variant:ident, ($($field:ident),+)) => {
        match $instruction {
            cairo_m_common::Instruction::$variant { $($field),+ } => {
                // Creates a tuple of the dereferenced fields: (*cond_off, *offset)
                ($(*$field),+)
            },
            _ => {
                return Err($crate::vm::instructions::InstructionExecutionError::InvalidInstructionType);
            }
        }
    };

    // Case 2: Extracting a single field.
    // e.g., extract_as!(instruction, JmpAbsImm, target)
    ($instruction:expr, $variant:ident, $field:ident) => {
        match $instruction {
            cairo_m_common::Instruction::$variant { $field } => {
                // Dereferences the single field: *target
                *$field
            },
            _ => {
                return Err($crate::vm::instructions::InstructionExecutionError::InvalidInstructionType);
            }
        }
    };

    // Case 3: Validating instruction variant with no fields (like Ret).
    // e.g., extract_as!(instruction, Ret)
    ($instruction:expr, $variant:ident) => {
        match $instruction {
            cairo_m_common::Instruction::$variant { .. } => {
                // No fields to extract, just validates the variant
            },
            _ => {
                return Err($crate::vm::instructions::InstructionExecutionError::InvalidInstructionType);
            }
        }
    };
}

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

/// Maps an opcode to its corresponding instruction handler function.
///
/// ## Arguments
///
/// * `op` - The opcode ID as an [`M31`] field element.
///
/// ## Returns
///
/// Returns an [`InstructionFn`] - a function pointer that can execute the instruction
/// when called with memory, state, and instruction arguments.
///
/// ## Errors
///
/// Returns [`InstructionError::InvalidOpcode`] if the provided opcode does not
/// correspond to any implemented instruction.
pub fn opcode_to_instruction_fn(op: M31) -> Result<InstructionFn, InstructionError> {
    let f = match op.0 {
        STORE_ADD_FP_FP => store_add_fp_fp,
        STORE_ADD_FP_IMM => store_add_fp_imm,
        STORE_SUB_FP_FP => store_sub_fp_fp,
        STORE_SUB_FP_IMM => store_sub_fp_imm,
        STORE_DOUBLE_DEREF_FP => store_double_deref_fp,
        STORE_DOUBLE_DEREF_FP_FP => store_double_deref_fp_fp,
        STORE_IMM => store_imm,
        STORE_FP_IMM => store_fp_imm,
        STORE_MUL_FP_FP => store_mul_fp_fp,
        STORE_MUL_FP_IMM => store_mul_fp_imm,
        STORE_DIV_FP_FP => store_div_fp_fp,
        STORE_DIV_FP_IMM => store_div_fp_imm,
        CALL_ABS_IMM => call_abs_imm,
        RET => ret,
        JMP_ABS_IMM => jmp_abs_imm,
        JMP_REL_IMM => jmp_rel_imm,
        JNZ_FP_IMM => jnz_fp_imm,
        U32_STORE_ADD_FP_FP => u32_store_add_fp_fp,
        U32_STORE_SUB_FP_FP => u32_store_sub_fp_fp,
        U32_STORE_MUL_FP_FP => u32_store_mul_fp_fp,
        U32_STORE_DIV_FP_FP => u32_store_div_fp_fp,
        U32_STORE_ADD_FP_IMM => u32_store_add_fp_imm,
        U32_STORE_SUB_FP_IMM => u32_store_sub_fp_imm,
        U32_STORE_MUL_FP_IMM => u32_store_mul_fp_imm,
        U32_STORE_DIV_FP_IMM => u32_store_div_fp_imm,
        U32_STORE_IMM => u32_store_imm,
        U32_STORE_EQ_FP_FP => u32_store_eq_fp_fp,
        U32_STORE_NEQ_FP_FP => u32_store_neq_fp_fp,
        U32_STORE_GT_FP_FP => u32_store_gt_fp_fp,
        U32_STORE_GE_FP_FP => u32_store_ge_fp_fp,
        U32_STORE_LT_FP_FP => u32_store_lt_fp_fp,
        U32_STORE_LE_FP_FP => u32_store_le_fp_fp,
        U32_STORE_EQ_FP_IMM => u32_store_eq_fp_imm,
        U32_STORE_NEQ_FP_IMM => u32_store_neq_fp_imm,
        U32_STORE_GT_FP_IMM => u32_store_gt_fp_imm,
        U32_STORE_GE_FP_IMM => u32_store_ge_fp_imm,
        U32_STORE_LT_FP_IMM => u32_store_lt_fp_imm,
        U32_STORE_LE_FP_IMM => u32_store_le_fp_imm,
        U32_STORE_AND_FP_FP => u32_store_and_fp_fp,
        U32_STORE_OR_FP_FP => u32_store_or_fp_fp,
        U32_STORE_XOR_FP_FP => u32_store_xor_fp_fp,
        U32_STORE_AND_FP_IMM => u32_store_and_fp_imm,
        U32_STORE_OR_FP_IMM => u32_store_or_fp_imm,
        U32_STORE_XOR_FP_IMM => u32_store_xor_fp_imm,
        STORE_TO_DOUBLE_DEREF_FP_IMM => store_to_double_deref_fp_imm,
        STORE_TO_DOUBLE_DEREF_FP_FP => store_to_double_deref_fp_fp,
        _ => return Err(InstructionError::InvalidOpcode(op)),
    };
    Ok(f)
}

#[cfg(test)]
mod tests {
    use cairo_m_common::instruction::{InstructionError, MAX_OPCODE};
    use stwo_prover::core::fields::m31::M31;

    use super::opcode_to_instruction_fn;

    #[test]
    fn test_opcode_to_instruction_fn_invalid_opcode() {
        let invalid_opcode = M31(2_u32.pow(30));
        let result = opcode_to_instruction_fn(invalid_opcode);
        assert_eq!(result, Err(InstructionError::InvalidOpcode(invalid_opcode)));
    }

    #[test]
    fn test_opcode_to_instruction_fn_valid_opcodes() {
        for opcode_value in 0..=MAX_OPCODE {
            let opcode = M31(opcode_value);
            let result = opcode_to_instruction_fn(opcode);
            assert!(result.is_ok(), "Opcode {opcode_value} should be valid");
        }
    }
}
