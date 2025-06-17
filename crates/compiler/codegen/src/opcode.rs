//TODO: sync with VM team to avoid duplication.

/// CASM opcodes with type-safe representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Opcode {
    // Arithmetic operations
    StoreAddFpFp = 0,  // [fp + off2] = [fp + off0] + [fp + off1]
    StoreAddFpImm = 1, // [fp + off2] = [fp + off0] + imm
    StoreSubFpFp = 2,  // [fp + off2] = [fp + off0] - [fp + off1]
    StoreSubFpImm = 3, // [fp + off2] = [fp + off0] - imm

    // Memory operations
    StoreDerefFp = 4,       // [fp + off2] = [fp + off0]
    StoreDoubleDerefFp = 5, // [fp + off2] = [[fp + off0] + off1]
    StoreImm = 6,           // [fp + off2] = imm

    // Multiplication/Division
    StoreMulFpFp = 7,   // [fp + off2] = [fp + off0] * [fp + off1]
    StoreMulFpImm = 8,  // [fp + off2] = [fp + off0] * imm
    StoreDivFpFp = 9,   // [fp + off2] = [fp + off0] / [fp + off1]
    StoreDivFpImm = 10, // [fp + off2] = [fp + off0] / imm

    // Call operations
    CallAbsFp = 11,  // call abs [fp + off0]
    CallAbsImm = 12, // call abs imm
    CallRelFp = 13,  // call rel [fp + off0]
    CallRelImm = 14, // call rel imm
    Ret = 15,        // ret

    // Jump operations
    JmpAbsAddFpFp = 16,       // jmp abs [fp + off0] + [fp + off1]
    JmpAbsAddFpImm = 17,      // jmp abs [fp + off0] + imm
    JmpAbsDerefFp = 18,       // jmp abs [fp + off0]
    JmpAbsDoubleDerefFp = 19, // jmp abs [[fp + off0] + off1]
    JmpAbsImm = 20,           // jmp abs imm
    JmpAbsMulFpFp = 21,       // jmp abs [fp + off0] * [fp + off1]
    JmpAbsMulFpImm = 22,      // jmp abs [fp + off0] * imm
    JmpRelAddFpFp = 23,       // jmp rel [fp + off0] + [fp + off1]
    JmpRelAddFpImm = 24,      // jmp rel [fp + off0] + imm
    JmpRelDerefFp = 25,       // jmp rel [fp + off0]
    JmpRelDoubleDerefFp = 26, // jmp rel [[fp + off0] + off1]
    JmpRelImm = 27,           // jmp rel imm
    JmpRelMulFpFp = 28,       // jmp rel [fp + off0] * [fp + off1]
    JmpRelMulFpImm = 29,      // jmp rel [fp + off0] * imm

    // Conditional jumps
    JnzFpFp = 30,  // jmp rel [fp + off1] if [fp + off0] != 0
    JnzFpImm = 31, // jmp rel imm if [fp + off0] != 0
}

impl From<Opcode> for u32 {
    fn from(opcode: Opcode) -> Self {
        opcode as Self
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
            11 => Some(Self::CallAbsFp),
            12 => Some(Self::CallAbsImm),
            13 => Some(Self::CallRelFp),
            14 => Some(Self::CallRelImm),
            15 => Some(Self::Ret),
            16 => Some(Self::JmpAbsAddFpFp),
            17 => Some(Self::JmpAbsAddFpImm),
            18 => Some(Self::JmpAbsDerefFp),
            19 => Some(Self::JmpAbsDoubleDerefFp),
            20 => Some(Self::JmpAbsImm),
            21 => Some(Self::JmpAbsMulFpFp),
            22 => Some(Self::JmpAbsMulFpImm),
            23 => Some(Self::JmpRelAddFpFp),
            24 => Some(Self::JmpRelAddFpImm),
            25 => Some(Self::JmpRelDerefFp),
            26 => Some(Self::JmpRelDoubleDerefFp),
            27 => Some(Self::JmpRelImm),
            28 => Some(Self::JmpRelMulFpFp),
            29 => Some(Self::JmpRelMulFpImm),
            30 => Some(Self::JnzFpFp),
            31 => Some(Self::JnzFpImm),
            _ => None,
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
    pub const STORE_DEREF_FP: u32 = Opcode::StoreDerefFp as u32;
    pub const STORE_DOUBLE_DEREF_FP: u32 = Opcode::StoreDoubleDerefFp as u32;
    pub const STORE_IMM: u32 = Opcode::StoreImm as u32;
    pub const STORE_MUL_FP_FP: u32 = Opcode::StoreMulFpFp as u32;
    pub const STORE_MUL_FP_IMM: u32 = Opcode::StoreMulFpImm as u32;
    pub const STORE_DIV_FP_FP: u32 = Opcode::StoreDivFpFp as u32;
    pub const STORE_DIV_FP_IMM: u32 = Opcode::StoreDivFpImm as u32;
    pub const CALL_ABS_FP: u32 = Opcode::CallAbsFp as u32;
    pub const CALL_ABS_IMM: u32 = Opcode::CallAbsImm as u32;
    pub const CALL_REL_FP: u32 = Opcode::CallRelFp as u32;
    pub const CALL_REL_IMM: u32 = Opcode::CallRelImm as u32;
    pub const RET: u32 = Opcode::Ret as u32;
    pub const JMP_ABS_ADD_FP_FP: u32 = Opcode::JmpAbsAddFpFp as u32;
    pub const JMP_ABS_ADD_FP_IMM: u32 = Opcode::JmpAbsAddFpImm as u32;
    pub const JMP_ABS_DEREF_FP: u32 = Opcode::JmpAbsDerefFp as u32;
    pub const JMP_ABS_DOUBLE_DEREF_FP: u32 = Opcode::JmpAbsDoubleDerefFp as u32;
    pub const JMP_ABS_IMM: u32 = Opcode::JmpAbsImm as u32;
    pub const JMP_ABS_MUL_FP_FP: u32 = Opcode::JmpAbsMulFpFp as u32;
    pub const JMP_ABS_MUL_FP_IMM: u32 = Opcode::JmpAbsMulFpImm as u32;
    pub const JMP_REL_ADD_FP_FP: u32 = Opcode::JmpRelAddFpFp as u32;
    pub const JMP_REL_ADD_FP_IMM: u32 = Opcode::JmpRelAddFpImm as u32;
    pub const JMP_REL_DEREF_FP: u32 = Opcode::JmpRelDerefFp as u32;
    pub const JMP_REL_DOUBLE_DEREF_FP: u32 = Opcode::JmpRelDoubleDerefFp as u32;
    pub const JMP_REL_IMM: u32 = Opcode::JmpRelImm as u32;
    pub const JMP_REL_MUL_FP_FP: u32 = Opcode::JmpRelMulFpFp as u32;
    pub const JMP_REL_MUL_FP_IMM: u32 = Opcode::JmpRelMulFpImm as u32;
    pub const JNZ_FP_FP: u32 = Opcode::JnzFpFp as u32;
    pub const JNZ_FP_IMM: u32 = Opcode::JnzFpImm as u32;
}
