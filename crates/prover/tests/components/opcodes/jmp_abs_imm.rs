#[test]
fn test_jmp_abs_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Jump to address 50
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpAbsImm as u32,
                    50, // off0 (jump target)
                    0,  // off1 (unused)
                    0,  // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
        // Test case 2: Jump to address 0 (start of program)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(100),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpAbsImm as u32,
                    0, // off0 (jump to start)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_imm
    );
}
