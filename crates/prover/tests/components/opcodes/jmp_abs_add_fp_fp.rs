#[test]
fn test_jmp_abs_add_fp_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsAddFpFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(30),
                value: M31::from(30),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(20),
                value: M31::from(20),
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_add_fp_fp
    );
}
