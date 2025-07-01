#[test]
fn test_store_sub_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case: [fp + 2] = [fp + 0] - [fp + 1]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreSubFpFp as u32,
                    0, // off0
                    1, // off1
                    2, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50),
                    value: M31::from(50),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(102),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(30), // 50 - 20
                }),
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_sub_fp_fp
    );
}
