use std::convert::TryFrom;

// Struct to hold constant characteristics of an opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpcodeInfo {
    pub memory_accesses: usize,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    StoreAddFpFp = 0x0,
    StoreAddFpImm = 0x01,
    StoreSubFpFp = 0x02,
    StoreSubFpImm = 0x03,
    StoreDerefFp = 0x04,
    StoreDoubleDerefFp = 0x05,
    StoreImm = 0x06,
    StoreMulFpFp = 0x07,
    StoreMulFpImm = 0x08,
    StoreDivFpFp = 0x09,
    StoreDivFpImm = 0x0A,
    CallAbsFp = 0x0B,
    CallAbsImm = 0x0C,
    CallRelFp = 0x0D,
    CallRelImm = 0x0E,
    Ret = 0x0F,
    JmpAbsAddFpFp = 0x10,
    JmpAbsAddFpImm = 0x11,
    JmpAbsDerefFp = 0x12,
    JmpAbsDoubleDerefFp = 0x13,
    JmpAbsImm = 0x14,
    JmpAbsMulFpFp = 0x15,
    JmpAbsMulFpImm = 0x16,
    JmpRelAddFpFp = 0x17,
    JmpRelAddFpImm = 0x18,
    JmpRelDerefFp = 0x19,
    JmpRelDoubleDerefFp = 0x1A,
    JmpRelImm = 0x1B,
    JmpRelMulFpFp = 0x1C,
    JmpRelMulFpImm = 0x1D,
    JnzFpFp = 0x1E,
    JnzFpImm = 0x1F,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidOpcodeError {
    pub id: u32,
}

impl InvalidOpcodeError {
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

impl std::fmt::Display for InvalidOpcodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid opcode ID: 0x{:02X}", self.id)
    }
}

impl std::error::Error for InvalidOpcodeError {}

// Implement TryFrom<u32> to convert opcode ID to Opcode
impl TryFrom<u32> for Opcode {
    type Error = InvalidOpcodeError;

    fn try_from(id: u32) -> Result<Self, Self::Error> {
        match id {
            0x00 => Ok(Self::StoreAddFpFp),
            0x01 => Ok(Self::StoreAddFpImm),
            0x02 => Ok(Self::StoreSubFpFp),
            0x03 => Ok(Self::StoreSubFpImm),
            0x04 => Ok(Self::StoreDerefFp),
            0x05 => Ok(Self::StoreDoubleDerefFp),
            0x06 => Ok(Self::StoreImm),
            0x07 => Ok(Self::StoreMulFpFp),
            0x08 => Ok(Self::StoreMulFpImm),
            0x09 => Ok(Self::StoreDivFpFp),
            0x0A => Ok(Self::StoreDivFpImm),
            0x0B => Ok(Self::CallAbsFp),
            0x0C => Ok(Self::CallAbsImm),
            0x0D => Ok(Self::CallRelFp),
            0x0E => Ok(Self::CallRelImm),
            0x0F => Ok(Self::Ret),
            0x10 => Ok(Self::JmpAbsAddFpFp),
            0x11 => Ok(Self::JmpAbsAddFpImm),
            0x12 => Ok(Self::JmpAbsDerefFp),
            0x13 => Ok(Self::JmpAbsDoubleDerefFp),
            0x14 => Ok(Self::JmpAbsImm),
            0x15 => Ok(Self::JmpAbsMulFpFp),
            0x16 => Ok(Self::JmpAbsMulFpImm),
            0x17 => Ok(Self::JmpRelAddFpFp),
            0x18 => Ok(Self::JmpRelAddFpImm),
            0x19 => Ok(Self::JmpRelDerefFp),
            0x1A => Ok(Self::JmpRelDoubleDerefFp),
            0x1B => Ok(Self::JmpRelImm),
            0x1C => Ok(Self::JmpRelMulFpFp),
            0x1D => Ok(Self::JmpRelMulFpImm),
            0x1E => Ok(Self::JnzFpFp),
            0x1F => Ok(Self::JnzFpImm),
            _ => Err(Self::Error::new(id)),
        }
    }
}

// Implement methods for Opcode
impl Opcode {
    // Get the constant characteristics for the opcode
    pub const fn info(self) -> OpcodeInfo {
        match self {
            Self::StoreAddFpFp => OpcodeInfo {
                memory_accesses: 3, // [fp + off2] = [fp + off0] + [fp + off1]
            },
            Self::StoreAddFpImm => OpcodeInfo {
                memory_accesses: 2, // [fp + off2] = [fp + off0] + imm
            },
            Self::StoreSubFpFp => OpcodeInfo {
                memory_accesses: 3, // [fp + off2] = [fp + off0] - [fp + off1]
            },
            Self::StoreSubFpImm => OpcodeInfo {
                memory_accesses: 2, // [fp + off2] = [fp + off0] - imm
            },
            Self::StoreDerefFp => OpcodeInfo {
                memory_accesses: 2, // [fp + off2] = [fp + off0]
            },
            Self::StoreDoubleDerefFp => OpcodeInfo {
                memory_accesses: 3, // [fp + off2] = [[fp + off0] + off1]
            },
            Self::StoreImm => OpcodeInfo {
                memory_accesses: 1, // [fp + off2] = imm
            },
            Self::StoreMulFpFp => OpcodeInfo {
                memory_accesses: 3, // [fp + off2] = [fp + off0] * [fp + off1]
            },
            Self::StoreMulFpImm => OpcodeInfo {
                memory_accesses: 2, // [fp + off2] = [fp + off0] * imm
            },
            Self::StoreDivFpFp => OpcodeInfo {
                memory_accesses: 3, // [fp + off2] = [fp + off0] / [fp + off1]
            },
            Self::StoreDivFpImm => OpcodeInfo {
                memory_accesses: 2, // [fp + off2] = [fp + off0] / imm
            },
            Self::CallAbsFp => OpcodeInfo {
                memory_accesses: 3, // call abs [fp + off0]
            },
            Self::CallAbsImm => OpcodeInfo {
                memory_accesses: 2, // call abs imm
            },
            Self::CallRelFp => OpcodeInfo {
                memory_accesses: 3, // call rel [fp + off0]
            },
            Self::CallRelImm => OpcodeInfo {
                memory_accesses: 2, // call rel imm
            },
            Self::Ret => OpcodeInfo {
                memory_accesses: 2, // ret
            },
            Self::JmpAbsAddFpFp => OpcodeInfo {
                memory_accesses: 2, // jmp abs [fp + off0] + [fp + off1]
            },
            Self::JmpAbsAddFpImm => OpcodeInfo {
                memory_accesses: 1, // jmp abs [fp + off0] + imm
            },
            Self::JmpAbsDerefFp => OpcodeInfo {
                memory_accesses: 1, // jmp abs [fp + off0]
            },
            Self::JmpAbsDoubleDerefFp => OpcodeInfo {
                memory_accesses: 2, // jmp abs [[fp + off0] + off1]
            },
            Self::JmpAbsImm => OpcodeInfo {
                memory_accesses: 0, // jmp abs imm
            },
            Self::JmpAbsMulFpFp => OpcodeInfo {
                memory_accesses: 2, // jmp abs [fp + off0] * [fp + off1]
            },
            Self::JmpAbsMulFpImm => OpcodeInfo {
                memory_accesses: 1, // jmp abs [fp + off0] * imm
            },
            Self::JmpRelAddFpFp => OpcodeInfo {
                memory_accesses: 2, // jmp rel [fp + off0] + [fp + off1]
            },
            Self::JmpRelAddFpImm => OpcodeInfo {
                memory_accesses: 1, // jmp rel [fp + off0] + imm
            },
            Self::JmpRelDerefFp => OpcodeInfo {
                memory_accesses: 1, // jmp rel [fp + off0]
            },
            Self::JmpRelDoubleDerefFp => OpcodeInfo {
                memory_accesses: 2, // jmp rel [[fp + off0] + off1]
            },
            Self::JmpRelImm => OpcodeInfo {
                memory_accesses: 0, // jmp rel imm
            },
            Self::JmpRelMulFpFp => OpcodeInfo {
                memory_accesses: 2, // jmp rel [fp + off0] * [fp + off1]
            },
            Self::JmpRelMulFpImm => OpcodeInfo {
                memory_accesses: 1, // jmp rel [fp + off0] * imm
            },
            Self::JnzFpFp => OpcodeInfo {
                memory_accesses: 2, // jmp rel [fp + off1] if [fp + off0] != 0
            },
            Self::JnzFpImm => OpcodeInfo {
                memory_accesses: 1, // jmp rel imm if [fp + off0] != 0
            },
        }
    }
}
