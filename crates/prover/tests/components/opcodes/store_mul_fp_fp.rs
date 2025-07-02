#[test]
fn test_store_mul_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = a * b (different operands, different destination)
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
                    0, // off0 (a)
                    1, // off1 (b)
                    2, // off2 (c)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(101), // fp + 1 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(8),
                    value: M31::from(8),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2 (c)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(56), // 7 * 8
                }),
            ],
        },
        // Test case 2: b = b * b (same operand twice, result overwrites operand)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpFp as u32,
                    1, // off0 (b)
                    1, // off1 (b)
                    1, // off2 (b)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(201), // fp + 1 (b first read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(201),  // fp + 1 (b second read)
                    prev_clock: M31::from(2), // clock from first read
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(201),  // fp + 1 (b write)
                    prev_clock: M31::from(2), // clock from second read
                    prev_value: M31::from(5),
                    value: M31::from(25), // 5 * 5 = 25
                }),
            ],
        },
        // Test case 3: a = a * b (in-place multiplication with different operand)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpFp as u32,
                    0, // off0 (a)
                    1, // off1 (b)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(300), // fp + 0 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(3),
                    value: M31::from(3),
                }),
                Some(DataAccess {
                    address: M31::from(301), // fp + 1 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(4),
                    value: M31::from(4),
                }),
                Some(DataAccess {
                    address: M31::from(300),  // fp + 0 (a)
                    prev_clock: M31::from(3), // clock from first read
                    prev_value: M31::from(3),
                    value: M31::from(12), // 3 * 4
                }),
            ],
        },
        // Test case 4: larger values to test field arithmetic
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(3),
                fp: M31::from(400),
            },
            clock: M31::from(4),
            instruction: InstructionAccess {
                prev_clock: M31::from(3),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpFp as u32,
                    0, // off0
                    1, // off1
                    2, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(400),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(1000000),
                    value: M31::from(1000000),
                }),
                Some(DataAccess {
                    address: M31::from(401),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(2000),
                    value: M31::from(2000),
                }),
                Some(DataAccess {
                    address: M31::from(402),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(1000000) * M31::from(2000), // Field multiplication
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_fp
    );
}
