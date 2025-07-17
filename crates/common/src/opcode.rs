use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::instruction::InstructionError;

// Struct to hold constant characteristics of an opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpcodeInfo {
    pub memory_accesses: usize,
    pub operand_count: usize,
}

/// CASM opcodes with type-safe representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Opcode {
    // Arithmetic operations
    StoreAddFpFp,  // [fp + off2] = [fp + off0] + [fp + off1]
    StoreAddFpImm, // [fp + off2] = [fp + off0] + imm
    StoreSubFpFp,  // [fp + off2] = [fp + off0] - [fp + off1]
    StoreSubFpImm, // [fp + off2] = [fp + off0] - imm

    // Memory operations
    StoreDerefFp,       // [fp + off2] = [fp + off0]
    StoreDoubleDerefFp, // [fp + off2] = [[fp + off0] + off1]
    StoreImm,           // [fp + off2] = imm

    // Multiplication/Division
    StoreMulFpFp,  // [fp + off2] = [fp + off0] * [fp + off1]
    StoreMulFpImm, // [fp + off2] = [fp + off0] * imm
    StoreDivFpFp,  // [fp + off2] = [fp + off0] / [fp + off1]
    StoreDivFpImm, // [fp + off2] = [fp + off0] / imm

    // Call operations
    CallAbsImm, // call abs imm
    Ret,        // ret

    // Jump operations
    JmpAbsImm, // jmp abs imm
    JmpRelImm, // jmp rel imm

    // Conditional jumps
    JnzFpImm, // jmp rel imm if [fp + off0] != 0

    // U32 operations
    U32StoreAddFpImm, // u32([fp + off2], [fp + off2 + 1]) = u32([fp + off0], [fp + off0 + 1]) + u32(imm_limb_0, imm_limb_1)
}

impl From<Opcode> for u32 {
    fn from(opcode: Opcode) -> Self {
        opcode as Self
    }
}

impl From<Opcode> for M31 {
    fn from(opcode: Opcode) -> Self {
        Self::from(opcode as u32)
    }
}

impl TryFrom<u32> for Opcode {
    type Error = InstructionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_u32(value).ok_or_else(|| InstructionError::InvalidOpcode(M31::from(value)))
    }
}

impl TryFrom<M31> for Opcode {
    type Error = InstructionError;

    fn try_from(value: M31) -> Result<Self, Self::Error> {
        Self::from_u32(value.0).ok_or(InstructionError::InvalidOpcode(value))
    }
}

impl TryFrom<QM31> for Opcode {
    type Error = InstructionError;

    fn try_from(value: QM31) -> Result<Self, Self::Error> {
        let opcode_u32 = value.to_m31_array()[0].0;
        Self::from_u32(opcode_u32)
            .ok_or_else(|| InstructionError::InvalidOpcode(M31::from(opcode_u32)))
    }
}

impl Opcode {
    /// Convert opcode to its numeric value
    pub const fn to_u32(self) -> u32 {
        self as u32
    }

    /// Try to convert a u32 to an opcode
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::StoreAddFpFp),
            1 => Some(Self::StoreAddFpImm),
            2 => Some(Self::StoreSubFpFp),
            3 => Some(Self::StoreSubFpImm),
            4 => Some(Self::StoreDerefFp),
            5 => Some(Self::StoreDoubleDerefFp),
            6 => Some(Self::StoreImm),
            7 => Some(Self::StoreMulFpFp),
            8 => Some(Self::StoreMulFpImm),
            9 => Some(Self::StoreDivFpFp),
            10 => Some(Self::StoreDivFpImm),
            11 => Some(Self::CallAbsImm),
            12 => Some(Self::Ret),
            13 => Some(Self::JmpAbsImm),
            14 => Some(Self::JmpRelImm),
            15 => Some(Self::JnzFpImm),
            16 => Some(Self::U32StoreAddFpImm),
            _ => None,
        }
    }

    /// Get the constant characteristics for the opcode
    pub const fn info(self) -> OpcodeInfo {
        match self {
            Self::StoreAddFpFp => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 3,
            },
            Self::StoreAddFpImm => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::StoreSubFpFp => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 3,
            },
            Self::StoreSubFpImm => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::StoreDerefFp => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::StoreDoubleDerefFp => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 3,
            },
            Self::StoreImm => OpcodeInfo {
                memory_accesses: 1,
                operand_count: 3,
            },
            Self::StoreMulFpFp => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 3,
            },
            Self::StoreMulFpImm => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::StoreDivFpFp => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 3,
            },
            Self::StoreDivFpImm => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::CallAbsImm => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 3,
            },
            Self::Ret => OpcodeInfo {
                memory_accesses: 2,
                operand_count: 0,
            },
            Self::JmpAbsImm => OpcodeInfo {
                memory_accesses: 0,
                operand_count: 1,
            },
            Self::JmpRelImm => OpcodeInfo {
                memory_accesses: 0,
                operand_count: 1,
            },
            Self::JnzFpImm => OpcodeInfo {
                memory_accesses: 1,
                operand_count: 2,
            },
            Self::U32StoreAddFpImm => OpcodeInfo {
                memory_accesses: 3,
                operand_count: 4,
            },
        }
    }

    /// Get the name of the opcode as a string
    pub fn name(&self) -> String {
        format!("{self:?}")
    }

    /// Get the total size of the instruction in M31 fields (opcode + operands)
    ///
    /// This method is defined on Opcode rather than Instruction because:
    /// 1. Size is an intrinsic property of the opcode type, not the instruction instance
    /// 2. We need to know the expected size during parsing/decoding BEFORE creating an instruction
    /// 3. The VM needs to know how many QM31s to read from memory based solely on the opcode
    ///
    /// Example use case in parsing:
    /// ```ignore
    /// let opcode = Opcode::try_from(values[0])?;
    /// let expected_size = opcode.size_in_m31s();  // Need size before creating instruction!
    /// if values.len() != expected_size { /* error */ }
    /// ```
    pub const fn size_in_m31s(self) -> usize {
        1 + self.info().operand_count
    }

    /// Get the size of the instruction in QM31 fields (rounded up to nearest multiple of 4)
    ///
    /// This method is defined on Opcode for the same reasons as size_in_m31s().
    /// The VM uses this to determine how many memory cells to read for variable-sized instructions.
    ///
    /// For example:
    /// - Most instructions: size_in_qm31s() = 1 (fits in single QM31)
    /// - U32StoreAddFpImm: size_in_qm31s() = 2 (needs 5 M31s, spans 2 QM31s)
    pub const fn size_in_qm31s(self) -> u32 {
        self.size_in_m31s().div_ceil(4) as u32
    }
}

// Re-export as module for backward compatibility
pub mod opcodes {
    use super::Opcode;

    pub const STORE_ADD_FP_FP: u32 = Opcode::StoreAddFpFp as u32;
    pub const STORE_ADD_FP_IMM: u32 = Opcode::StoreAddFpImm as u32;
    pub const STORE_SUB_FP_FP: u32 = Opcode::StoreSubFpFp as u32;
    pub const STORE_SUB_FP_IMM: u32 = Opcode::StoreSubFpImm as u32;
    pub const STORE_DEREF_FP: u32 = Opcode::StoreDerefFp as u32;
    pub const STORE_DOUBLE_DEREF_FP: u32 = Opcode::StoreDoubleDerefFp as u32;
    pub const STORE_IMM: u32 = Opcode::StoreImm as u32;
    pub const STORE_MUL_FP_FP: u32 = Opcode::StoreMulFpFp as u32;
    pub const STORE_MUL_FP_IMM: u32 = Opcode::StoreMulFpImm as u32;
    pub const STORE_DIV_FP_FP: u32 = Opcode::StoreDivFpFp as u32;
    pub const STORE_DIV_FP_IMM: u32 = Opcode::StoreDivFpImm as u32;
    pub const CALL_ABS_IMM: u32 = Opcode::CallAbsImm as u32;
    pub const RET: u32 = Opcode::Ret as u32;
    pub const JMP_ABS_IMM: u32 = Opcode::JmpAbsImm as u32;
    pub const JMP_REL_IMM: u32 = Opcode::JmpRelImm as u32;
    pub const JNZ_FP_IMM: u32 = Opcode::JnzFpImm as u32;
    pub const U32_STORE_ADD_FP_IMM: u32 = Opcode::U32StoreAddFpImm as u32;
}
