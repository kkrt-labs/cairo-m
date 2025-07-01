#[test]
fn test_jmp_abs_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsDerefFp as u32,
                1, // off0 (offset to deref)
                0, // off1 (unused)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(101), // fp + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(75), // Jump target
                value: M31::from(75),
            }),
            None,
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_deref_fp
    );
}
