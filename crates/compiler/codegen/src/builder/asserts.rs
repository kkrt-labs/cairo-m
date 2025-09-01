use crate::InstructionBuilder;
use cairo_m_common::Instruction as CasmInstr;
use stwo_prover::core::fields::m31::M31;

impl super::CasmBuilder {
    #[allow(dead_code)]
    pub(crate) fn assert_eq_fp_fp(&mut self, src0_off: i32, src1_off: i32, comment: String) {
        let instr = InstructionBuilder::from(CasmInstr::AssertEqFpFp {
            src0_off: M31::from(src0_off),
            src1_off: M31::from(src1_off),
        })
        .with_comment(comment);
        self.emit_push(instr);
    }

    pub(crate) fn assert_eq_fp_imm(&mut self, src_off: i32, imm: i32, comment: String) {
        let instr = InstructionBuilder::from(CasmInstr::AssertEqFpImm {
            src_off: M31::from(src_off),
            imm: M31::from(imm),
        })
        .with_comment(comment);
        self.emit_push(instr);
    }
}

#[cfg(test)]
mod tests {
    use crate::{CasmBuilder, FunctionLayout};

    use super::*;

    #[test]
    fn test_assert_eq_fp_fp() {
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.assert_eq_fp_fp(0, 1, "test".to_string());
        assert_eq!(builder.instructions.len(), 1);
        assert_eq!(
            builder.instructions[0].inner_instr(),
            &CasmInstr::AssertEqFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(1),
            }
        );
    }

    #[test]
    fn test_assert_eq_fp_imm() {
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.assert_eq_fp_imm(0, 1, "test".to_string());
        assert_eq!(builder.instructions.len(), 1);
        assert_eq!(
            builder.instructions[0].inner_instr(),
            &CasmInstr::AssertEqFpImm {
                src_off: M31::from(0),
                imm: M31::from(1),
            }
        );
    }
}
