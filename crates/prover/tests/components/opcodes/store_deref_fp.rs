#[test]
fn test_store_deref_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: b = a (copy from one location to another)
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
                    0, // off0 (source: a)
                    0, // off1 (unused)
                    1, // off2 (destination: b)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0 (a)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(77),
                    value: M31::from(77),
                }),
                Some(DataAccess {
                    address: M31::from(101), // fp + 1 (b)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(77), // Copy value from a
                }),
                None,
            ],
        },
        // Test case 2: a = a (self-assignment)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDerefFp as u32,
                    0, // off0 (source: a)
                    0, // off1 (unused)
                    0, // off2 (destination: a)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200), // fp + 0 (a read)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(42),
                    value: M31::from(42),
                }),
                Some(DataAccess {
                    address: M31::from(200),  // fp + 0 (a write)
                    prev_clock: M31::from(2), // clock from read
                    prev_value: M31::from(42),
                    value: M31::from(42), // Same value (self-assignment)
                }),
                None,
            ],
        },
        // Test case 3: Copy with negative offsets
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(2),
                fp: M31::from(300),
            },
            clock: M31::from(3),
            instruction: InstructionAccess {
                prev_clock: M31::from(2),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDerefFp as u32,
                    M31::from(-2).0, // off0 (source)
                    0,               // off1 (unused)
                    0,               // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(298), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(123),
                    value: M31::from(123),
                }),
                Some(DataAccess {
                    address: M31::from(300), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(123), // Copy value
                }),
                None,
            ],
        },
        // Test case 4: Copy larger value
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(3),
                fp: M31::from(400),
            },
            clock: M31::from(4),
            instruction: InstructionAccess {
                prev_clock: M31::from(3),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDerefFp as u32,
                    1, // off0 (source)
                    0, // off1 (unused)
                    3, // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(401), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(999999),
                    value: M31::from(999999),
                }),
                Some(DataAccess {
                    address: M31::from(403), // fp + 3
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(999999), // Copy value
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
