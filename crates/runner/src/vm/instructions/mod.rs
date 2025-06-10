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

use stwo_prover::core::fields::{m31::M31, qm31::QM31};

pub mod store;

/// The arguments of a Cairo M instruction.
/// It is represented as a fixed-size array of 3 M31 values.
/// * off0 - The first element of the array.
/// * off1 - The second element of the array.
/// * off2 - The third element of the array.
pub(crate) type InstructionArgs = [M31; 3];

/// A Cairo M instruction is made of an opcode and 3 arguments.
/// * op - The opcode id of the instruction.
/// * args - The arguments (offsets) of the instruction.
#[derive(Clone, Copy, Debug)]
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

#[cfg(test)]
mod tests {
    use num_traits::One;
    use stwo_prover::core::fields::{m31::M31, qm31::QM31};

    use crate::vm::instructions::Instruction;

    #[test]
    fn test_instruction_from_qm31() {
        let instruction = QM31::from_m31_array([1, 2, 3, 4].map(Into::into));
        let instruction = Instruction::from(instruction);
        assert_eq!(instruction.op, M31::one());
        assert_eq!(instruction.args, [M31::from(2), M31::from(3), M31::from(4)]);
    }

    #[test]
    fn test_instruction_from_array() {
        let instruction = Instruction::from([1, 2, 3, 4].map(Into::<M31>::into));
        assert_eq!(instruction.op, M31::one());
        assert_eq!(instruction.args, [M31::from(2), M31::from(3), M31::from(4)]);
    }
}
