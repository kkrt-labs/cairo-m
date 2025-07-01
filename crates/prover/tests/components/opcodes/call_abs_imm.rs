#[test]
fn test_call_abs_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallAbsImm as u32,
                50, // off0 (call target)
                2,  // off1 (new fp offset)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(102), // fp + 2 (new fp location)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(103), // fp + 2 + 1 (return address)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(11), // PC + 1
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_abs_imm
    );
}
