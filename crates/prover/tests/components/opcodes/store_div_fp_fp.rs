#[test]
fn test_store_div_fp_fp_constraints() {
    // Division in M31 is multiplication by multiplicative inverse
    let divisor = M31::from(7);
    let dividend = M31::from(35);
    let quotient = dividend * divisor.inverse();
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreDivFpFp as u32,
                0, // off0 (dividend)
                1, // off1 (divisor)
                2, // off2 (quotient)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: dividend,
                value: dividend,
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: divisor,
                value: divisor,
            }),
            Some(DataAccess {
                address: M31::from(102),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: quotient, // 35 / 7 = 5 in M31
            }),
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_fp
    );
}
