#[test]
fn test_jnz_fp_imm_taken_constraints() {
    let execution_bundles = vec![ExecutionBundle {
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
                25, // off1 (immediate jump offset)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(42), // Non-zero - will jump
                value: M31::from(42),
            }),
            None,
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_imm_taken
    );
}
