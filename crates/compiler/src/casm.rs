use std::fmt::{self, Debug, Display};

#[derive(Clone)]
pub enum CasmInstructionType {
    MovFpFp,
    MovFpImm,
    AddFpFp,
    AddFpImm,
    SubFpFp,
    SubFpImm,
    MulFpFp,
    MulFpImm,
    JmpAbs,
    JmpRel,
    JmpAbsIfNeq,
    JmpRelIfNeq,
    CallRel,
    CallAbs,
    Label,
    JmpLabel,
    JmpLabelIfNeq,
    CallLabel,
    Ret,
}

impl CasmInstructionType {
    pub fn get_opcode(&self) -> u32 {
        match self {
            CasmInstructionType::MovFpFp => 0,
            CasmInstructionType::MovFpImm => 1,
            CasmInstructionType::AddFpFp => 2,
            CasmInstructionType::AddFpImm => 3,
            CasmInstructionType::SubFpFp => 4,
            CasmInstructionType::SubFpImm => 5,
            CasmInstructionType::MulFpFp => 6,
            CasmInstructionType::MulFpImm => 7,
            CasmInstructionType::JmpAbs => 8,
            CasmInstructionType::JmpRel => 9,
            CasmInstructionType::JmpAbsIfNeq => 10,
            CasmInstructionType::JmpRelIfNeq => 11,
            CasmInstructionType::CallRel => 12,
            CasmInstructionType::CallAbs => 13,
            CasmInstructionType::Label => 14,
            CasmInstructionType::Ret => 15,
            _ => panic!("Label instructions cannot be lowered to bytecode."),
        }
    }
}

impl Display for CasmInstructionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CasmInstructionType::MovFpFp => write!(f, "mov_fp_fp"),
            CasmInstructionType::MovFpImm => write!(f, "mov_fp_imm"),
            CasmInstructionType::AddFpFp => write!(f, "add_fp_fp"),
            CasmInstructionType::AddFpImm => write!(f, "add_fp_imm"),
            CasmInstructionType::SubFpFp => write!(f, "sub_fp_fp"),
            CasmInstructionType::SubFpImm => write!(f, "sub_fp_imm"),
            CasmInstructionType::MulFpFp => write!(f, "mul_fp_fp"),
            CasmInstructionType::MulFpImm => write!(f, "mul_fp_imm"),
            CasmInstructionType::JmpAbs => write!(f, "jmp_abs"),
            CasmInstructionType::JmpRel => write!(f, "jmp_rel"),
            CasmInstructionType::JmpAbsIfNeq => write!(f, "jmp_abs_if_neq"),
            CasmInstructionType::JmpRelIfNeq => write!(f, "jmp_rel_if_neq"),
            CasmInstructionType::CallRel => write!(f, "call_rel"),
            CasmInstructionType::CallAbs => write!(f, "call_abs"),
            CasmInstructionType::Label => write!(f, "label"),
            CasmInstructionType::Ret => write!(f, "ret"),
            CasmInstructionType::JmpLabel => write!(f, "jmp_label"),
            CasmInstructionType::JmpLabelIfNeq => write!(f, "jmp_label_if_neq"),
            CasmInstructionType::CallLabel => write!(f, "call_label"),
        }
    }
}

#[derive(Clone)]
pub struct CasmInstruction {
    pub instruction_type: CasmInstructionType,
    pub label: Option<String>,
    pub arg0: i32,
    pub arg1: i32,
    pub arg2: i32,
}

impl CasmInstruction {
    pub fn to_bytes(&self) -> (u32, u32, u32, u32) {
        let opcode = self.instruction_type.get_opcode();
        let arg0 = (0x8000 + self.arg0) as u32;
        let arg1 = (0x8000 + self.arg1) as u32;
        let arg2 = (0x8000 + self.arg2) as u32;
        (opcode, arg0, arg1, arg2)
    }
}

impl Display for CasmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.instruction_type {
            CasmInstructionType::Label => {
                if let Some(label) = &self.label {
                    write!(f, "{}:", label)
                } else {
                    write!(f, "{}", self.instruction_type)
                }
            }
            CasmInstructionType::MovFpFp => {
                write!(f, "[fp + {}] = [fp + {}]", self.arg0, self.arg1)
            }
            CasmInstructionType::MovFpImm => {
                write!(f, "[fp + {}] = {}", self.arg0, self.arg1)
            }
            CasmInstructionType::AddFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] + [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::AddFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] + {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::SubFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] - [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::SubFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] - {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::MulFpFp => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] * [fp + {}]",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::MulFpImm => {
                write!(
                    f,
                    "[fp + {}] = [fp + {}] * {}",
                    self.arg0, self.arg1, self.arg2
                )
            }
            CasmInstructionType::CallLabel => {
                if let Some(label) = &self.label {
                    write!(f, "call {};", label)
                } else {
                    write!(f, "call;")
                }
            }
            CasmInstructionType::Ret => {
                write!(f, "ret;")
            }
            CasmInstructionType::JmpLabel => {
                if let Some(label) = &self.label {
                    write!(f, "jmp {};", label)
                } else {
                    write!(f, "jmp;")
                }
            }
            CasmInstructionType::JmpLabelIfNeq => {
                if let Some(label) = &self.label {
                    write!(f, "jmp {} if [fp + {}] != 0;", label, self.arg1)
                } else {
                    write!(f, "jmp if [fp + {}] != 0;", self.arg1)
                }
            }
            CasmInstructionType::JmpAbs => {
                write!(f, "jmp {};", self.arg0)
            }
            CasmInstructionType::JmpRel => {
                write!(f, "jmp {};", self.arg0)
            }
            CasmInstructionType::JmpAbsIfNeq => {
                write!(f, "jmp {} if [fp + {}] != 0;", self.arg0, self.arg1)
            }
            CasmInstructionType::JmpRelIfNeq => {
                write!(f, "jmp {} if [fp + {}] != 0;", self.arg0, self.arg1)
            }
            CasmInstructionType::CallRel => {
                write!(f, "call {};", self.arg0)
            }
            CasmInstructionType::CallAbs => {
                write!(f, "call {};", self.arg0)
            }
        }
    }
}
