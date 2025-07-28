use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::instruction::InstructionError;

// Struct to hold constant characteristics of an opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpcodeInfo {
    pub memory_accesses: usize,
}

/// CASM opcodes with type-safe representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Opcode {
    // Arithmetic operations: order matters for the prover, see store_fp_fp.rs
    StoreAddFpFp, // [fp + off2] = [fp + off0] + [fp + off1]
    StoreSubFpFp, // [fp + off2] = [fp + off0] - [fp + off1]
    StoreMulFpFp, // [fp + off2] = [fp + off0] * [fp + off1]
    StoreDivFpFp, // [fp + off2] = [fp + off0] / [fp + off1]

    // Arithmetic operations with immediate: order matters for the prover, see store_fp_imm.rs
    StoreAddFpImm, // [fp + off2] = [fp + off0] + imm
    StoreSubFpImm, // [fp + off2] = [fp + off0] - imm
    StoreMulFpImm, // [fp + off2] = [fp + off0] * imm
    StoreDivFpImm, // [fp + off2] = [fp + off0] / imm

    // Memory operations
    StoreDoubleDerefFp, // [fp + off2] = [[fp + off0] + off1]
    StoreImm,           // [fp + off2] = imm

    // Call operations
    CallAbsImm, // call abs imm
    Ret,        // ret

    // Jump operations: order matters for the prover, see jmp_imm.rs
    JmpAbsImm, // jmp abs imm
    JmpRelImm, // jmp rel imm

    // Conditional jumps
    JnzFpImm, // jmp rel imm if [fp + off0] != 0
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
            1 => Some(Self::StoreSubFpFp),
            2 => Some(Self::StoreMulFpFp),
            3 => Some(Self::StoreDivFpFp),
            4 => Some(Self::StoreAddFpImm),
            5 => Some(Self::StoreSubFpImm),
            6 => Some(Self::StoreMulFpImm),
            7 => Some(Self::StoreDivFpImm),
            8 => Some(Self::StoreDoubleDerefFp),
            9 => Some(Self::StoreImm),
            10 => Some(Self::CallAbsImm),
            11 => Some(Self::Ret),
            12 => Some(Self::JmpAbsImm),
            13 => Some(Self::JmpRelImm),
            14 => Some(Self::JnzFpImm),
            _ => None,
        }
    }

    /// Get the constant characteristics for the opcode
    pub const fn info(self) -> OpcodeInfo {
        match self {
            Self::StoreAddFpFp => OpcodeInfo { memory_accesses: 3 },
            Self::StoreAddFpImm => OpcodeInfo { memory_accesses: 2 },
            Self::StoreSubFpFp => OpcodeInfo { memory_accesses: 3 },
            Self::StoreSubFpImm => OpcodeInfo { memory_accesses: 2 },
            Self::StoreDoubleDerefFp => OpcodeInfo { memory_accesses: 3 },
            Self::StoreImm => OpcodeInfo { memory_accesses: 1 },
            Self::StoreMulFpFp => OpcodeInfo { memory_accesses: 3 },
            Self::StoreMulFpImm => OpcodeInfo { memory_accesses: 2 },
            Self::StoreDivFpFp => OpcodeInfo { memory_accesses: 3 },
            Self::StoreDivFpImm => OpcodeInfo { memory_accesses: 2 },
            Self::CallAbsImm => OpcodeInfo { memory_accesses: 2 },
            Self::Ret => OpcodeInfo { memory_accesses: 2 },
            Self::JmpAbsImm => OpcodeInfo { memory_accesses: 0 },
            Self::JmpRelImm => OpcodeInfo { memory_accesses: 0 },
            Self::JnzFpImm => OpcodeInfo { memory_accesses: 1 },
        }
    }

    /// Get the name of the opcode as a string
    pub fn name(&self) -> String {
        format!("{self:?}")
    }
}

// Re-export as module for backward compatibility
pub mod opcodes {
    use super::Opcode;

    pub const STORE_ADD_FP_FP: u32 = Opcode::StoreAddFpFp as u32;
    pub const STORE_ADD_FP_IMM: u32 = Opcode::StoreAddFpImm as u32;
    pub const STORE_SUB_FP_FP: u32 = Opcode::StoreSubFpFp as u32;
    pub const STORE_SUB_FP_IMM: u32 = Opcode::StoreSubFpImm as u32;
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
}
