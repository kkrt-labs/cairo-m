#[test]
fn test_call_rel_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(40),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallRelFp as u32,
                0, // off0 (offset to relative call value)
                4, // off1 (new fp offset)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Relative offset
                value: M31::from(15),
            }),
            Some(DataAccess {
                address: M31::from(104), // fp + 4
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(105), // fp + 4 + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(41), // PC + 1
            }),
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_rel_fp
    );
}
