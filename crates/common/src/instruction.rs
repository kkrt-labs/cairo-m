use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::Opcode;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
}

/// The operands of a Cairo M instruction.
/// It is represented as a fixed-size array of 3 M31 values.
/// * off0 - The first element of the array.
/// * off1 - The second element of the array.
/// * off2 - The third element of the array.
pub(crate) type InstructionOperands = [M31; 3];

/// A Cairo M instruction is made of an opcode and 3 arguments.
/// * opcode - The opcode id of the instruction.
/// * operands - The arguments (offsets) of the instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: InstructionOperands,
}

impl TryFrom<QM31> for Instruction {
    type Error = InstructionError;

    fn try_from(instruction: QM31) -> Result<Self, Self::Error> {
        let [op, args @ ..] = instruction.to_m31_array();
        Ok(Self {
            opcode: op.try_into()?,
            operands: args,
        })
    }
}

impl<T: Into<M31>> TryFrom<[T; 4]> for Instruction {
    type Error = InstructionError;

    fn try_from(instruction: [T; 4]) -> Result<Self, Self::Error> {
        let [op, args @ ..] = instruction;
        Ok(Self {
            opcode: Opcode::try_from(op.into())?,
            operands: args.map(Into::into),
        })
    }
}

impl From<&Instruction> for QM31 {
    fn from(instruction: &Instruction) -> Self {
        Self::from_m31_array(instruction.to_array())
    }
}

impl Instruction {
    /// Create a new instruction
    pub const fn new(opcode: Opcode, operands: [M31; 3]) -> Self {
        Self { opcode, operands }
    }

    /// Get the first operand
    pub const fn op0(&self) -> M31 {
        self.operands[0]
    }

    /// Get the second operand
    pub const fn op1(&self) -> M31 {
        self.operands[1]
    }

    /// Get the third operand
    pub const fn op2(&self) -> M31 {
        self.operands[2]
    }

    /// Convert to array representation [opcode, op0, op1, op2]
    pub fn to_array(&self) -> [M31; 4] {
        [
            M31::from(self.opcode),
            self.operands[0],
            self.operands[1],
            self.operands[2],
        ]
    }

    /// Create from array representation [opcode, op0, op1, op2]
    pub fn from_array(array: [M31; 4]) -> Result<Self, InstructionError> {
        let [opcode, operands @ ..] = array;
        Ok(Self {
            opcode: Opcode::try_from(opcode)?,
            operands,
        })
    }
}

/// Serialize instruction as JSON array for compatibility
impl Serialize for Instruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(4))?;

        seq.serialize_element(&format!("0x{:x}", self.opcode.to_u32()))?;
        for operand in self.operands {
            seq.serialize_element(&format!("0x{:x}", operand.0))?;
        }
        seq.end()
    }
}

/// Deserialize instruction from JSON array
impl<'de> Deserialize<'de> for Instruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;
        let hex_strings: [String; 4] = Deserialize::deserialize(deserializer)?;
        let parse_hex = |s: &str| -> Result<u32, D::Error> {
            u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(de::Error::custom)
        };
        let opcode_u32 = parse_hex(&hex_strings[0])?;
        let opcode = Opcode::try_from(opcode_u32).map_err(de::Error::custom)?;
        Ok(Self {
            opcode,
            operands: [
                M31::from(parse_hex(&hex_strings[1])?),
                M31::from(parse_hex(&hex_strings[2])?),
                M31::from(parse_hex(&hex_strings[3])?),
            ],
        })
    }
}
