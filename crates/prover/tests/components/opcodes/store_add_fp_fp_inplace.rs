#[test]
fn test_store_add_fp_fp_inplace_constraints() {
    // In-place operations modify the destination directly
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreAddFpFp as u32, // Note: opcode is same, but component handles in-place
                0,                           // off0
                1,                           // off1
                0,                           // off2 (same as off0 for in-place)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(10),
                value: M31::from(10),
            }),
            Some(DataAccess {
                address: M31::from(101), // fp + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(5),
                value: M31::from(5),
            }),
            Some(DataAccess {
                address: M31::from(100), // fp + 0 (in-place update)
                prev_clock: M31::from(1),
                prev_value: M31::from(10),
                value: M31::from(15), // 10 + 5
            }),
        ],
    }];
    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_fp_inplace
    );
}
