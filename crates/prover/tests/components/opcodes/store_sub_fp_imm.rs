#[test]
fn test_store_sub_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = a - 15 (different destination)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreSubFpImm as u32,
                    0,  // off0
                    15, // off1 (immediate)
                    1,  // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(40),
                    value: M31::from(40),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(25), // 40 - 15
                }),
                None,
            ],
        },
        // Test case 2: a = a - 2 (in-place subtraction with immediate)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreSubFpImm as u32,
                    0, // off0 (a)
                    2, // off1 (immediate value)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50),
                    value: M31::from(50),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a write)
                    prev_clock: M31::from(2), // clock from read
                    prev_value: M31::from(50),
                    value: M31::from(48), // 50 - 2
                }),
                None,
            ],
        },
        // Test case 3: Subtraction with negative immediate (M31 field arithmetic)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreSubFpImm as u32,
                    1,               // off0
                    M31::from(-3).0, // off1 (immediate -3, which is like adding 3)
                    2,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(301),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(302),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(10) - M31::from(-3), // 10 - (-3) = 13 in M31
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_sub_fp_imm
    );
}
