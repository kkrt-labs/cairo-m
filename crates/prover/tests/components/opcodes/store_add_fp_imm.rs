#[test]
fn test_store_add_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Add positive immediate
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpImm as u32,
                    1,  // off0 (source)
                    42, // off1 (immediate value)
                    3,  // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(103), // fp + 3
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(52), // 10 + 42
                }),
                None,
            ],
        },
        // Test case 2: Add negative immediate (M31 field arithmetic)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(100),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpImm as u32,
                    0,               // off0
                    M31::from(-5).0, // off1 (immediate -5)
                    1,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(15), // 20 + (-5) = 15
                }),
                None,
            ],
        },
        // Test case 3: a = a + 3 (in-place addition with immediate)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(200),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpImm as u32,
                    0, // off0 (a)
                    3, // off1 (immediate value)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a write)
                    prev_clock: M31::from(3), // clock from read
                    prev_value: M31::from(10),
                    value: M31::from(13), // 10 + 3
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_imm
    );
}
