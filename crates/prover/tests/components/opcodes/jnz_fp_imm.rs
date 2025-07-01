#[test]
fn test_jnz_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Non-zero condition
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpImm as u32,
                    0,  // off0 (condition)
                    20, // off1 (immediate jump offset)
                    0,  // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(1), // Non-zero
                    value: M31::from(1),
                }),
                None,
                None,
            ],
        },
        // Test case 2: Zero condition
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(30),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpImm as u32,
                    0,  // off0 (condition)
                    15, // off1 (immediate jump offset)
                    0,  // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0), // Zero
                    value: M31::from(0),
                }),
                None,
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_imm
    );
}
