#[test]
fn test_jnz_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: Condition is non-zero (will jump)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpFp as u32,
                    0, // off0 (condition)
                    1, // off1 (jump offset)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5), // Non-zero
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7), // Jump offset
                    value: M31::from(7),
                }),
                None,
            ],
        },
        // Test case 2: Condition is zero (won't jump)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(20),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpFp as u32,
                    0, // off0 (condition)
                    1, // off1 (jump offset)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0), // Zero condition
                    value: M31::from(0),
                }),
                Some(DataAccess {
                    address: M31::from(201),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_fp
    );
}
