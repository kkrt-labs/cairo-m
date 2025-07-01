#[test]
fn test_store_add_fp_fp_constraints() {
    // Test case 1: Simple addition [fp + 4] = [fp + 1] + [fp + 2]
    let execution_bundles = vec![
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    1, // off0
                    2, // off1
                    4, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(104), // fp + 4
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(30), // 10 + 20
                }),
            ],
        },
        // Test case 2: Addition with negative offsets
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    M31::from(-2).0, // off0
                    M31::from(-1).0, // off1
                    0,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(198), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(200), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(12), // 5 + 7
                }),
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_fp
    );
}
