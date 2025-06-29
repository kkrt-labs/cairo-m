#[cfg(test)]
mod tests {
    use cairo_m_common::{Instruction, Opcode};
    use stwo_prover::core::fields::m31::M31;

    use crate::vm::instructions::cmp::*;
    use crate::vm::{Memory, State};

    /// Macro to create a test for binary comparison operations (fp, fp -> fp)
    macro_rules! test_cmp_fp_fp {
        ($test_name:ident, $opcode:expr, $func:ident, $val1:expr, $val2:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let mut memory: Memory = Default::default();
                let state = State {
                    pc: M31::from(0),
                    fp: M31::from(10),
                };

                // Set up values at fp+0 and fp+1
                memory
                    .insert(state.fp + M31::from(0), $val1.into())
                    .unwrap();
                memory
                    .insert(state.fp + M31::from(1), $val2.into())
                    .unwrap();

                let instruction =
                    Instruction::new($opcode, [M31::from(0), M31::from(1), M31::from(2)]);
                let new_state = $func(&mut memory, state, &instruction).unwrap();

                // Check result
                assert_eq!(
                    memory.get_data(state.fp + M31::from(2)).unwrap(),
                    M31::from($expected)
                );
                assert_eq!(new_state.pc, M31::from(1));
            }
        };
    }

    /// Macro to create a test for immediate comparison operations (fp, imm -> fp)
    macro_rules! test_cmp_fp_imm {
        ($test_name:ident, $opcode:expr, $func:ident, $val1:expr, $imm:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let mut memory: Memory = Default::default();
                let state = State {
                    pc: M31::from(0),
                    fp: M31::from(10),
                };

                // Set up value at fp+0
                memory
                    .insert(state.fp + M31::from(0), M31::from($val1).into())
                    .unwrap();

                let instruction =
                    Instruction::new($opcode, [M31::from(0), M31::from($imm), M31::from(2)]);
                let new_state = $func(&mut memory, state, &instruction).unwrap();

                // Check result
                assert_eq!(
                    memory.get_data(state.fp + M31::from(2)).unwrap(),
                    M31::from($expected)
                );
                assert_eq!(new_state.pc, M31::from(1));
            }
        };
    }

    test_cmp_fp_fp!(
        test_cmp_eq_fp_fp_equal,
        Opcode::CmpEqFpFp,
        cmp_eq_fp_fp,
        42,
        42,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_eq_fp_fp_not_equal,
        Opcode::CmpEqFpFp,
        cmp_eq_fp_fp,
        42,
        43,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_neq_fp_fp,
        Opcode::CmpNeqFpFp,
        cmp_neq_fp_fp,
        42,
        43,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_neq_fp_fp_equal,
        Opcode::CmpNeqFpFp,
        cmp_neq_fp_fp,
        42,
        42,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_lt_fp_fp,
        Opcode::CmpLtFpFp,
        cmp_lt_fp_fp,
        42,
        43,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_lt_fp_fp_false,
        Opcode::CmpLtFpFp,
        cmp_lt_fp_fp,
        43,
        42,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_lt_fp_fp_equal,
        Opcode::CmpLtFpFp,
        cmp_lt_fp_fp,
        42,
        42,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_gt_fp_fp,
        Opcode::CmpGtFpFp,
        cmp_gt_fp_fp,
        43,
        42,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_gt_fp_fp_false,
        Opcode::CmpGtFpFp,
        cmp_gt_fp_fp,
        42,
        43,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_gt_fp_fp_equal,
        Opcode::CmpGtFpFp,
        cmp_gt_fp_fp,
        42,
        42,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_le_fp_fp_less,
        Opcode::CmpLeFpFp,
        cmp_le_fp_fp,
        42,
        43,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_le_fp_fp_equal,
        Opcode::CmpLeFpFp,
        cmp_le_fp_fp,
        42,
        42,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_le_fp_fp_false,
        Opcode::CmpLeFpFp,
        cmp_le_fp_fp,
        43,
        42,
        0
    );
    test_cmp_fp_fp!(
        test_cmp_ge_fp_fp_greater,
        Opcode::CmpGeFpFp,
        cmp_ge_fp_fp,
        43,
        42,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_ge_fp_fp_equal,
        Opcode::CmpGeFpFp,
        cmp_ge_fp_fp,
        42,
        42,
        1
    );
    test_cmp_fp_fp!(
        test_cmp_ge_fp_fp_false,
        Opcode::CmpGeFpFp,
        cmp_ge_fp_fp,
        42,
        43,
        0
    );

    test_cmp_fp_imm!(
        test_cmp_eq_fp_imm,
        Opcode::CmpEqFpImm,
        cmp_eq_fp_imm,
        42,
        42,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_eq_fp_imm_false,
        Opcode::CmpEqFpImm,
        cmp_eq_fp_imm,
        42,
        43,
        0
    );
    test_cmp_fp_imm!(
        test_cmp_neq_fp_imm,
        Opcode::CmpNeqFpImm,
        cmp_neq_fp_imm,
        42,
        43,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_neq_fp_imm_false,
        Opcode::CmpNeqFpImm,
        cmp_neq_fp_imm,
        42,
        42,
        0
    );
    test_cmp_fp_imm!(
        test_cmp_lt_fp_imm,
        Opcode::CmpLtFpImm,
        cmp_lt_fp_imm,
        42,
        43,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_lt_fp_imm_false,
        Opcode::CmpLtFpImm,
        cmp_lt_fp_imm,
        43,
        42,
        0
    );
    test_cmp_fp_imm!(
        test_cmp_gt_fp_imm,
        Opcode::CmpGtFpImm,
        cmp_gt_fp_imm,
        43,
        42,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_gt_fp_imm_false,
        Opcode::CmpGtFpImm,
        cmp_gt_fp_imm,
        42,
        43,
        0
    );
    test_cmp_fp_imm!(
        test_cmp_le_fp_imm,
        Opcode::CmpLeFpImm,
        cmp_le_fp_imm,
        42,
        42,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_le_fp_imm_less,
        Opcode::CmpLeFpImm,
        cmp_le_fp_imm,
        42,
        43,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_le_fp_imm_false,
        Opcode::CmpLeFpImm,
        cmp_le_fp_imm,
        43,
        42,
        0
    );
    test_cmp_fp_imm!(
        test_cmp_ge_fp_imm,
        Opcode::CmpGeFpImm,
        cmp_ge_fp_imm,
        42,
        42,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_ge_fp_imm_greater,
        Opcode::CmpGeFpImm,
        cmp_ge_fp_imm,
        43,
        42,
        1
    );
    test_cmp_fp_imm!(
        test_cmp_ge_fp_imm_false,
        Opcode::CmpGeFpImm,
        cmp_ge_fp_imm,
        42,
        43,
        0
    );
}
