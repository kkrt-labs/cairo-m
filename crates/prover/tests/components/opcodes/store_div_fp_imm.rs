#[test]
fn test_store_div_fp_imm_constraints() {
    let dividend = M31::from(42);
    let divisor = M31::from(6);
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
                Opcode::StoreDivFpImm as u32,
                0,         // off0
                divisor.0, // off1 (immediate divisor)
                1,         // off2
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
                prev_value: M31::from(0),
                value: quotient, // 42 / 6 = 7 in M31
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_imm
    );
}
