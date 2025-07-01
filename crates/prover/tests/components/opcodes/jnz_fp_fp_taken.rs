#[test]
fn test_jnz_fp_fp_taken_constraints() {
    // This tests the taken branch component
    let execution_bundles = vec![ExecutionBundle {
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
                prev_value: M31::from(10), // Non-zero - will jump
                value: M31::from(10),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Jump offset
                value: M31::from(15),
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_fp_taken
    );
}
