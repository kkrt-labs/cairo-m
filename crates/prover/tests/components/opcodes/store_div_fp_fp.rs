#[test]
fn test_store_div_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = d / b (different operands, different destination)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpFp as u32,
                    3, // off0 (d)
                    1, // off1 (b)
                    2, // off2 (c)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(103), // fp + 3 (d)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(101), // fp + 1 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2 (c)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(20) * M31::from(5).inverse(), // 20 / 5 = 4 in M31
                }),
            ],
        },
        // Test case 2: a = d / d (same operand twice, result overwrites different location)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpFp as u32,
                    3, // off0 (d)
                    3, // off1 (d)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(203), // fp + 3 (d first read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(42),
                    value: M31::from(42),
                }),
                Some(DataAccess {
                    address: M31::from(203),  // fp + 3 (d second read)
                    prev_clock: M31::from(2), // clock from first read
                    prev_value: M31::from(42),
                    value: M31::from(42),
                }),
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(1), // 42 / 42 = 1
                }),
            ],
        },
        // Test case 3: c = c / c (same operand thrice, in-place)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpFp as u32,
                    2, // off0 (c)
                    2, // off1 (c)
                    2, // off2 (c)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(302), // fp + 2 (c first read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(100),
                    value: M31::from(100),
                }),
                Some(DataAccess {
                    address: M31::from(302),  // fp + 2 (c second read)
                    prev_clock: M31::from(3), // clock from first read
                    prev_value: M31::from(100),
                    value: M31::from(100),
                }),
                Some(DataAccess {
                    address: M31::from(302),  // fp + 2 (c write)
                    prev_clock: M31::from(3), // clock from second read
                    prev_value: M31::from(100),
                    value: M31::from(1), // 100 / 100 = 1
                }),
            ],
        },
        // Test case 4: Division with specific values to test field arithmetic
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(3),
                fp: M31::from(400),
            },
            clock: M31::from(4),
            instruction: InstructionAccess {
                prev_clock: M31::from(3),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpFp as u32,
                    0, // off0 (dividend)
                    1, // off1 (divisor)
                    2, // off2 (quotient)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(400),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(35),
                    value: M31::from(35),
                }),
                Some(DataAccess {
                    address: M31::from(401),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(402),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(35) * M31::from(7).inverse(), // 35 / 7 = 5 in M31
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_fp
    );
}
