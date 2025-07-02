#[test]
fn test_store_div_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = a / 6 (different destination)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpImm as u32,
                    0, // off0
                    6, // off1 (immediate divisor)
                    1, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(42),
                    value: M31::from(42),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(42) * M31::from(6).inverse(), // 42 / 6 = 7 in M31
                }),
                None,
            ],
        },
        // Test case 2: d = d / 4 (in-place division with immediate)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpImm as u32,
                    3, // off0 (d)
                    4, // off1 (immediate value)
                    3, // off2 (d)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(203), // fp + 3 (d read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(203),  // fp + 3 (d write)
                    prev_clock: M31::from(2), // clock from read
                    prev_value: M31::from(20),
                    value: M31::from(20) * M31::from(4).inverse(), // 20 / 4 = 5 in M31
                }),
                None,
            ],
        },
        // Test case 3: Division with larger values
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDivFpImm as u32,
                    0,   // off0
                    100, // off1 (immediate divisor)
                    1,   // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(300),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50000),
                    value: M31::from(50000),
                }),
                Some(DataAccess {
                    address: M31::from(301),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(50000) * M31::from(100).inverse(), // 50000 / 100 = 500 in M31
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_imm
    );
}
