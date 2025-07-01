#[test]
fn test_store_mul_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreMulFpImm as u32,
                0,  // off0
                11, // off1 (immediate multiplier)
                1,  // off2
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(5),
                value: M31::from(5),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(55), // 5 * 11
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_imm
    );
}
