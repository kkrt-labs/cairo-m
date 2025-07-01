#[test]
fn test_jmp_rel_double_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelDoubleDerefFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(200),
                value: M31::from(200),
            }),
            Some(DataAccess {
                address: M31::from(201),
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Relative offset
                value: M31::from(15),
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_double_deref_fp
    );
}
