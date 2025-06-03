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
            CasmInstructionType::JmpLabel => write!(f, "jmp_label"),
            CasmInstructionType::JmpLabelIfNeq => write!(f, "jmp_label_if_neq"),
            CasmInstructionType::CallLabel => write!(f, "call_label"),
            CasmInstructionType::Ret => write!(f, "ret"),
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
            _ => write!(f, "{}", self.instruction_type),
        }
    }
}
