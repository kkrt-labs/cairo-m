use cairo_m_common::Instruction as CasmInstr;
use stwo_prover::core::fields::m31::M31;

use crate::InstructionBuilder;

impl super::CasmBuilder {
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
    use super::*;
    use crate::{CasmBuilder, FunctionLayout};

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
