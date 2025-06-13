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

use crate::vm::{
    instructions::{call::*, jnz::*, jump::*, store::*},
    Memory, MemoryError, State,
};
use stwo_prover::core::fields::{m31::M31, qm31::QM31};
use thiserror::Error;

pub mod call;
pub mod jnz;
pub mod jump;
pub mod store;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
}

/// The arguments of a Cairo M instruction.
/// It is represented as a fixed-size array of 3 M31 values.
/// * off0 - The first element of the array.
/// * off1 - The second element of the array.
/// * off2 - The third element of the array.
pub(crate) type InstructionArgs = [M31; 3];

/// A Cairo M instruction is made of an opcode and 3 arguments.
/// * op - The opcode id of the instruction.
/// * args - The arguments (offsets) of the instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub op: M31,
    pub args: InstructionArgs,
}

impl From<QM31> for Instruction {
    fn from(instruction: QM31) -> Self {
        let [op, args @ ..] = instruction.to_m31_array();
        Self { op, args }
    }
}

impl<T: Into<M31>> From<[T; 4]> for Instruction {
    fn from(instruction: [T; 4]) -> Self {
        let [op, args @ ..] = instruction;
        Self {
            op: op.into(),
            args: args.map(Into::into),
        }
    }
}

type InstructionFn = fn(&mut Memory, State, Instruction) -> Result<State, MemoryError>;

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
/// TOOD: update the order to match specs when all instructions are implemented.
pub fn opcode_to_instruction_fn(op: M31) -> Result<InstructionFn, InstructionError> {
    let f = match op.0 {
        0 => store_add_fp_fp,          // [fp + off2] = [fp + off0] + [fp + off1]
        1 => store_add_fp_imm,         // [fp + off2] = [fp + off0] + imm
        2 => store_sub_fp_fp,          // [fp + off2] = [fp + off0] - [fp + off1]
        3 => store_sub_fp_imm,         // [fp + off2] = [fp + off0] - imm
        4 => store_deref_fp,           // [fp + off2] = [fp + off0]
        5 => store_double_deref_fp,    // [fp + off2] = [[fp + off0] + off1]
        6 => store_imm,                // [fp + off2] = imm
        7 => store_mul_fp_fp,          // [fp + off2] = [fp + off0] * [fp + off1]
        8 => store_mul_fp_imm,         // [fp + off2] = [fp + off0] * imm
        9 => store_div_fp_fp,          // [fp + off2] = [fp + off0] / [fp + off1]
        10 => store_div_fp_imm,        // [fp + off2] = [fp + off0] / imm
        11 => call_abs_fp,             // call abs [fp + off0]
        12 => call_abs_imm,            // call abs imm
        13 => call_rel_fp,             // call rel [fp + off0]
        14 => call_rel_imm,            // call rel imm
        15 => ret,                     // ret
        16 => jmp_abs_add_fp_fp,       // jmp abs [fp + off0] + [fp + off1]
        17 => jmp_abs_add_fp_imm,      // jmp abs [fp + off0] + imm
        18 => jmp_abs_deref_fp,        // jmp abs [fp + off0]
        19 => jmp_abs_double_deref_fp, // jmp abs [[fp + off0] + off1]
        20 => jmp_abs_imm,             // jmp abs imm
        21 => jmp_abs_mul_fp_fp,       // jmp abs [fp + off0] * [fp + off1]
        22 => jmp_abs_mul_fp_imm,      // jmp abs [fp + off0] * imm
        23 => jmp_rel_add_fp_fp,       // jmp rel [fp + off0] + [fp + off1]
        24 => jmp_rel_add_fp_imm,      // jmp rel [fp + off0] + imm
        25 => jmp_rel_deref_fp,        // jmp rel [fp + off0]
        26 => jmp_rel_double_deref_fp, // jmp rel [[fp + off0] + off1]
        27 => jmp_rel_imm,             // jmp rel imm
        28 => jmp_rel_mul_fp_fp,       // jmp rel [fp + off0] * [fp + off1]
        29 => jmp_rel_mul_fp_imm,      // jmp rel [fp + off0] * imm
        30 => jnz_fp_fp,               // jmp rel [fp + off1] if [fp + off0] != 0
        31 => jnz_fp_imm,              // jmp rel imm if [fp + off0] != 0
        _ => return Err(InstructionError::InvalidOpcode(op)),
    };
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::{opcode_to_instruction_fn, InstructionError};
    use num_traits::One;
    use stwo_prover::core::fields::{m31::M31, qm31::QM31};

    use crate::vm::instructions::Instruction;

    const LAST_VALID_OPCODE_ID: u32 = 31;

    #[test]
    fn test_instruction_from_qm31() {
        let instruction = QM31::from_m31_array([1, 2, 3, 4].map(Into::into));
        let instruction = Instruction::from(instruction);
        assert_eq!(instruction.op, M31::one());
        assert_eq!(instruction.args, [M31(2), M31(3), M31(4)]);
    }

    #[test]
    fn test_instruction_from_array() {
        let instruction = Instruction::from([1, 2, 3, 4].map(Into::<M31>::into));
        assert_eq!(instruction.op, M31::one());
        assert_eq!(instruction.args, [M31(2), M31(3), M31(4)]);
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
