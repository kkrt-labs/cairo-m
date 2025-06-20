use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::Opcode;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
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
/// * opcode - The opcode id of the instruction.
/// * operands - The arguments (offsets) of the instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: InstructionArgs,
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
        let opcode = Opcode::try_from(array[0])?;
        Ok(Self {
            opcode,
            operands: [array[1], array[2], array[3]],
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

        // Serialize as hex strings for JSON compatibility
        seq.serialize_element(&format!("0x{:x}", self.opcode.to_u32()))?;
        seq.serialize_element(&format!("0x{:x}", self.operands[0].0))?;
        seq.serialize_element(&format!("0x{:x}", self.operands[1].0))?;
        seq.serialize_element(&format!("0x{:x}", self.operands[2].0))?;
        seq.end()
    }
}

/// Deserialize instruction from JSON array
impl<'de> Deserialize<'de> for Instruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};

        struct InstructionVisitor;

        impl<'de> Visitor<'de> for InstructionVisitor {
            type Value = Instruction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("array of 4 hex strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let parse_hex = |s: &str| -> Result<u32, A::Error> {
                    u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(de::Error::custom)
                };

                let opcode_str: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let op0_str: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let op1_str: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let op2_str: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;

                let opcode_u32 = parse_hex(&opcode_str)?;
                let opcode = Opcode::try_from(opcode_u32).map_err(de::Error::custom)?;

                Ok(Instruction {
                    opcode,
                    operands: [
                        M31::from(parse_hex(&op0_str)?),
                        M31::from(parse_hex(&op1_str)?),
                        M31::from(parse_hex(&op2_str)?),
                    ],
                })
            }
        }

        deserializer.deserialize_seq(InstructionVisitor)
    }
}
