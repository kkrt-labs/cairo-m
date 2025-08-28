use cairo_m_common::instruction::{ASSERT_EQ_FP_FP, ASSERT_EQ_FP_IMM};

use crate::{InstructionBuilder, Operand};

impl super::CasmBuilder {
    #[allow(dead_code)]
    pub(crate) fn assert_eq_fp_fp(&mut self, src0_off: i32, src1_off: i32, comment: String) {
        let instr = InstructionBuilder::new(ASSERT_EQ_FP_FP)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(src1_off))
            .with_comment(comment);
        self.emit_push(instr);
    }

    pub(crate) fn assert_eq_fp_imm(&mut self, src_off: i32, imm: i32, comment: String) {
        let instr = InstructionBuilder::new(ASSERT_EQ_FP_IMM)
            .with_operand(Operand::Literal(src_off))
            .with_operand(Operand::Literal(imm))
            .with_comment(comment);
        self.emit_push(instr);
    }
}
