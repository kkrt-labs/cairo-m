#[test]
fn test_store_mul_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: Simple multiplication
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpFp as u32,
                    0, // off0
                    1, // off1
                    2, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(8),
                    value: M31::from(8),
                }),
                Some(DataAccess {
                    address: M31::from(102),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(56), // 7 * 8
                }),
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_fp
    );
}
