#[test]
fn test_call_abs_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallAbsFp as u32,
                1, // off0 (offset to call target)
                3, // off1 (new fp offset)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(101), // fp + 1 (call target)
                prev_clock: M31::from(0),
                prev_value: M31::from(80),
                value: M31::from(80),
            }),
            Some(DataAccess {
                address: M31::from(103), // fp + 3 (new fp location)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(104), // fp + 3 + 1 (return address)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(21), // PC + 1
            }),
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_abs_fp
    );
}
