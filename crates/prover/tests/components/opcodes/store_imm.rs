#[test]
fn test_store_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Store positive immediate
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreImm as u32,
                    42, // off0 (immediate value)
                    0,  // off1 (unused)
                    2,  // off2 (offset from fp)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(42),
                }),
                None,
                None,
            ],
        },
        // Test case 2: Store to negative offset
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreImm as u32,
                    100,             // off0 (immediate value)
                    0,               // off1 (unused)
                    M31::from(-1).0, // off2 (negative offset)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(100),
                }),
                None,
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_imm
    );
}
