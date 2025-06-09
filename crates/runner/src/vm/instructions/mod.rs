use stwo_prover::core::fields::{m31::M31, qm31::QM31};

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
            args: args.map(|x| x.into()),
        }
    }
}
