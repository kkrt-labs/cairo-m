#[test]
fn test_store_mul_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: c = a * 11 (different destination)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpImm as u32,
                    0,  // off0
                    11, // off1 (immediate multiplier)
                    1,  // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(55), // 5 * 11
                }),
                None,
            ],
        },
        // Test case 2: a = a * 4 (in-place multiplication with immediate)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpImm as u32,
                    0, // off0 (a)
                    4, // off1 (immediate value)
                    0, // off2 (a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(12),
                    value: M31::from(12),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a write)
                    prev_clock: M31::from(2), // clock from read
                    prev_value: M31::from(12),
                    value: M31::from(48), // 12 * 4
                }),
                None,
            ],
        },
        // Test case 3: Multiplication with larger immediate
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpImm as u32,
                    1,     // off0
                    10000, // off1 (immediate)
                    2,     // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(301),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(100),
                    value: M31::from(100),
                }),
                Some(DataAccess {
                    address: M31::from(302),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(100) * M31::from(10000), // Field multiplication
                }),
                None,
            ],
        },
    ];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_imm
    );
}
