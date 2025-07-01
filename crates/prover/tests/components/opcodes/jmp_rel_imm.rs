#[test]
fn test_jmp_rel_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Jump forward
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpRelImm as u32,
                    5, // off0 (relative jump +5)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
        // Test case 2: Jump backward
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(50),
                fp: M31::from(100),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpRelImm as u32,
                    M31::from(-10).0, // off0 (relative jump -10)
                    0,                // off1 (unused)
                    0,                // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_imm
    );
}
