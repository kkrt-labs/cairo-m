#[test]
fn test_store_double_deref_fp_constraints() {
    let execution_bundles = vec![
        // Test: [fp + 3] = [[fp + 0] + 1]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDoubleDerefFp as u32,
                    0, // off0 (first deref)
                    1, // off1 (offset for second deref)
                    3, // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(200), // Points to address 200
                    value: M31::from(200),
                }),
                Some(DataAccess {
                    address: M31::from(201), // [fp + 0] + 1 = 200 + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(88),
                    value: M31::from(88),
                }),
                Some(DataAccess {
                    address: M31::from(103), // fp + 3
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(88), // Value from double deref
                }),
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_double_deref_fp
    );
}
