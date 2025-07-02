#[test]
fn test_store_add_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = a + b (different operands, different destination)
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
                    1, // off0 (a)
                    2, // off1 (b)
                    3, // off2 (c)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(101), // fp + 1 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(103), // fp + 3 (c)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(15), // 10 + 5
                }),
            ],
        },
        // Test case 2: a = a + a (same operand twice, result overwrites operand)
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
                    0, // off0 (a)
                    0, // off1 (a)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a first read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a second read)
                    prev_clock: M31::from(2), // clock from first read
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a write)
                    prev_clock: M31::from(2), // clock from second read
                    prev_value: M31::from(7),
                    value: M31::from(14), // 7 + 7
                }),
            ],
        },
        // Test case 3: a = a + b (in-place addition with different operand)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    0, // off0 (a)
                    1, // off1 (b)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(300), // fp + 0 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(301), // fp + 1 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(8),
                    value: M31::from(8),
                }),
                Some(DataAccess {
                    address: M31::from(300),  // fp + 0 (a)
                    prev_clock: M31::from(3), // clock from first read
                    prev_value: M31::from(20),
                    value: M31::from(28), // 20 + 8
                }),
            ],
        },
        // Test case 4: with negative offsets (edge case)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(3),
                fp: M31::from(400),
            },
            clock: M31::from(4),
            instruction: InstructionAccess {
                prev_clock: M31::from(3),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    M31::from(-2).0, // off0
                    M31::from(-1).0, // off1
                    0,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(398), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(399), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(400), // fp + 0
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
