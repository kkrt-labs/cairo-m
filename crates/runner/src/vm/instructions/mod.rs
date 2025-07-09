//! Instructions for the Cairo M VM.
//!
//! Cairo M instructions are currently of fixed size, encoded on 4 M31:
//! - One for the opcode.
//! - One for the first operand
//! - One for the second operand
//! - One for the destination
//! - Some instructions might use less than 3 arguments.
//!
//! A QM31 is made of 4 M31 (extension field of CM31, the extension field of M31).
//! This is why instructions can be represented as a QM31.

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Opcode, State};
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

pub type InstructionFn = fn(&mut Memory, State, &Instruction) -> Result<State, MemoryError>;

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
    // Try to convert M31 to Opcode enum
    let opcode = Opcode::try_from(op)?;

    let f = match opcode {
        Opcode::StoreAddFpFp => store_add_fp_fp,
        Opcode::StoreAddFpImm => store_add_fp_imm,
        Opcode::StoreSubFpFp => store_sub_fp_fp,
        Opcode::StoreSubFpImm => store_sub_fp_imm,
        Opcode::StoreDerefFp => store_deref_fp,
        Opcode::StoreDoubleDerefFp => store_double_deref_fp,
        Opcode::StoreImm => store_imm,
        Opcode::StoreMulFpFp => store_mul_fp_fp,
        Opcode::StoreMulFpImm => store_mul_fp_imm,
        Opcode::StoreDivFpFp => store_div_fp_fp,
        Opcode::StoreDivFpImm => store_div_fp_imm,
        Opcode::CallAbsImm => call_abs_imm,
        Opcode::Ret => ret,
        Opcode::JmpAbsImm => jmp_abs_imm,
        Opcode::JmpRelImm => jmp_rel_imm,
        Opcode::JnzFpImm => jnz_fp_imm,
    };
    Ok(f)
}

#[cfg(test)]
mod tests {
    use cairo_m_common::instruction::InstructionError;
    use cairo_m_common::{Instruction, Opcode};
    use stwo_prover::core::fields::m31::M31;
    use stwo_prover::core::fields::qm31::QM31;

    use super::opcode_to_instruction_fn;

    const LAST_VALID_OPCODE_ID: u32 = 15;

    #[test]
    fn test_instruction_from_qm31() {
        let instruction = QM31::from_m31_array([1, 2, 3, 4].map(Into::into));
        let instruction: Instruction = instruction.try_into().unwrap();
        assert_eq!(instruction.opcode, Opcode::StoreAddFpImm);
        assert_eq!(instruction.operands, [M31(2), M31(3), M31(4)]);
    }

    #[test]
    fn test_opcode_to_instruction_fn_invalid_opcode() {
        let invalid_opcode = M31(2_u32.pow(30));
        let result = opcode_to_instruction_fn(invalid_opcode);
        assert_eq!(result, Err(InstructionError::InvalidOpcode(invalid_opcode)));
    }

    #[test]
    fn test_opcode_to_instruction_fn_valid_opcodes() {
        for opcode_value in 0..=LAST_VALID_OPCODE_ID {
            let opcode = M31(opcode_value);
            let result = opcode_to_instruction_fn(opcode);
            assert!(result.is_ok(), "Opcode {opcode_value} should be valid");
        }
    }
}
