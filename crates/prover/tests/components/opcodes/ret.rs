#[test]
fn test_ret_constraints() {
    let execution_bundles = vec![
        // Test case 1: Return from function
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(25),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::Ret as u32,
                    0, // off0 (unused)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(98), // fp - 2 (old fp location)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50), // Previous fp value
                    value: M31::from(50),
                }),
                Some(DataAccess {
                    address: M31::from(99), // fp - 1 (return address)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(15), // Return address
                    value: M31::from(15),
                }),
                None,
            ],
        },
        // Test case 2: Return from nested function
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(80),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::Ret as u32,
                    0, // off0 (unused)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(198), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(150), // Previous fp
                    value: M31::from(150),
                }),
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(60), // Return address
                    value: M31::from(60),
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(execution_bundles, cairo_m_prover::components::opcodes::ret);
}
