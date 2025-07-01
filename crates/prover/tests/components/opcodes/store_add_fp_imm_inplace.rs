#[test]
fn test_store_add_fp_imm_inplace_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreAddFpImm as u32,
                0,  // off0
                20, // off1 (immediate)
                0,  // off2 (same as off0 for in-place)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(30),
                value: M31::from(30),
            }),
            Some(DataAccess {
                address: M31::from(100), // fp + 0 (in-place update)
                prev_clock: M31::from(1),
                prev_value: M31::from(30),
                value: M31::from(50), // 30 + 20
            }),
            None,
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_imm_inplace
    );
}
