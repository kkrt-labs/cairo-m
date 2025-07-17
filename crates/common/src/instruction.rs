use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;

use crate::Opcode;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
    #[error("Invalid operand count: expected {expected}, actual {actual}")]
    InvalidOperandCount { expected: usize, actual: usize },
    #[error("Empty instruction")]
    EmptyInstruction,
    #[error("Instruction size mismatch for opcode. Expected {expected}, found {found}")]
    SizeMismatch { expected: usize, found: usize },
}

/// A Cairo M instruction is made of an opcode and variable number of operands.
/// * opcode - The opcode id of the instruction.
/// * operands - The arguments (offsets) of the instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<M31>,
}

impl TryFrom<Vec<M31>> for Instruction {
    type Error = InstructionError;

    fn try_from(mut values: Vec<M31>) -> Result<Self, Self::Error> {
        if values.is_empty() {
            return Err(InstructionError::EmptyInstruction);
        }

        let opcode = Opcode::try_from(values[0])?;
        let expected_size = opcode.size_in_m31s();

        if values.len() != expected_size {
            return Err(InstructionError::SizeMismatch {
                expected: expected_size,
                found: values.len(),
            });
        }

        values.remove(0); // Remove opcode, leaving just operands
        Ok(Self {
            opcode,
            operands: values,
        })
    }
}

impl From<&Instruction> for Vec<M31> {
    fn from(instruction: &Instruction) -> Self {
        let mut result = Self::with_capacity(instruction.size_in_m31s());
        result.push(instruction.opcode.into());
        result.extend(&instruction.operands);
        result
    }
}

impl From<Instruction> for Vec<M31> {
    fn from(instruction: Instruction) -> Self {
        Self::from(&instruction)
    }
}

impl Instruction {
    /// Create a new instruction with validation
    /// This validates that the operand count matches what the opcode expects
    pub fn new(opcode: Opcode, operands: Vec<M31>) -> Result<Self, InstructionError> {
        let info = opcode.info();
        if operands.len() != info.operand_count {
            return Err(InstructionError::InvalidOperandCount {
                expected: info.operand_count,
                actual: operands.len(),
            });
        }
        Ok(Self { opcode, operands })
    }

    /// Create a new instruction without validation
    pub const fn new_unchecked(opcode: Opcode, operands: Vec<M31>) -> Self {
        Self { opcode, operands }
    }

    /// Get operand by index
    pub fn operand(&self, index: usize) -> Option<M31> {
        self.operands.get(index).copied()
    }

    /// Get all operands as a slice
    pub fn operands(&self) -> &[M31] {
        &self.operands
    }

    /// Get the expected size in M31s for this instruction's opcode
    pub const fn size_in_m31s(&self) -> usize {
        self.opcode.size_in_m31s()
    }

    /// Get the expected size in QM31s for this instruction's opcode
    pub const fn size_in_qm31s(&self) -> u32 {
        self.opcode.size_in_qm31s()
    }
}

/// Serialize instruction as JSON array for compatibility
impl Serialize for Instruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.size_in_m31s()))?;

        seq.serialize_element(&format!("0x{:x}", self.opcode.to_u32()))?;
        for &operand in &self.operands {
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
        let hex_strings: Vec<String> = Deserialize::deserialize(deserializer)?;
        let parse_hex = |s: &str| -> Result<u32, D::Error> {
            u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(de::Error::custom)
        };

        // Convert hex strings to M31 values
        let m31_values: Vec<M31> = hex_strings
            .iter()
            .map(|s| parse_hex(s).map(M31::from))
            .collect::<Result<Vec<_>, _>>()
            .map_err(de::Error::custom)?;

        // Use TryFrom to handle all validation
        Self::try_from(m31_values).map_err(de::Error::custom)
    }
}
