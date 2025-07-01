#[test]
fn test_jmp_abs_double_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsDoubleDerefFp as u32,
                0, // off0
                2, // off1 (offset for second deref)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(200), // First pointer
                value: M31::from(200),
            }),
            Some(DataAccess {
                address: M31::from(202), // 200 + 2
                prev_clock: M31::from(0),
                prev_value: M31::from(80), // Jump target
                value: M31::from(80),
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_double_deref_fp
    );
}
