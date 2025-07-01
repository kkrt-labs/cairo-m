#[test]
fn test_store_deref_fp_constraints() {
    let execution_bundles = vec![
        // Test: [fp + 2] = [fp + 0]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDerefFp as u32,
                    0, // off0 (source)
                    0, // off1 (unused)
                    2, // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(77),
                    value: M31::from(77),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(77), // Copy the value
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_deref_fp
    );
}
