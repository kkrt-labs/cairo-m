use cairo_m_common::instruction::{INSTRUCTION_MAX_SIZE, MAX_OPCODE};
use cairo_m_common::{Instruction, InstructionError};
use smallvec::{SmallVec, smallvec};
use stwo_prover::core::fields::m31::M31;

#[test]
fn test_instruction_sizes() {
    // Test size 1 instruction
    let ret = Instruction::Ret {};
    assert_eq!(ret.size_in_m31s(), 1);
    assert_eq!(ret.size_in_qm31s(), 1);

    // Test size 2 instructions
    let jmp_abs = Instruction::JmpAbsImm {
        target: M31::from(100),
    };
    assert_eq!(jmp_abs.size_in_m31s(), 2);
    assert_eq!(jmp_abs.size_in_qm31s(), 1);

    let jmp_rel = Instruction::JmpRelImm {
        offset: M31::from(50),
    };
    assert_eq!(jmp_rel.size_in_m31s(), 2);
    assert_eq!(jmp_rel.size_in_qm31s(), 1);

    // Test size 3 instructions
    let store_imm = Instruction::StoreImm {
        imm: M31::from(42),
        dst_off: M31::from(3),
    };
    assert_eq!(store_imm.size_in_m31s(), 3);
    assert_eq!(store_imm.size_in_qm31s(), 1);

    let jnz = Instruction::JnzFpImm {
        cond_off: M31::from(1),
        offset: M31::from(10),
    };
    assert_eq!(jnz.size_in_m31s(), 3);
    assert_eq!(jnz.size_in_qm31s(), 1);

    // Test size 4 instructions
    let add_fp_fp = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };
    assert_eq!(add_fp_fp.size_in_m31s(), 4);
    assert_eq!(add_fp_fp.size_in_qm31s(), 1);

    // Test size 5 instruction
    let u32_add = Instruction::U32StoreAddFpImm {
        src_off: M31::from(1),
        imm_hi: M31::from(0x1234),
        imm_lo: M31::from(0x5678),
        dst_off: M31::from(4),
    };
    assert_eq!(u32_add.size_in_m31s(), 5);
    assert_eq!(u32_add.size_in_qm31s(), 2);
}

#[test]
fn test_opcode_values() {
    // Test cases: (instruction_constructor, expected_opcode)
    let test_cases = [
        (
            Instruction::StoreAddFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            0,
        ),
        (
            Instruction::StoreSubFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            1,
        ),
        (
            Instruction::StoreMulFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            2,
        ),
        (
            Instruction::StoreDivFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            3,
        ),
        (
            Instruction::StoreAddFpImm {
                src_off: M31::from(0),
                imm: M31::from(0),
                dst_off: M31::from(0),
            },
            4,
        ),
        (
            Instruction::StoreSubFpImm {
                src_off: M31::from(0),
                imm: M31::from(0),
                dst_off: M31::from(0),
            },
            5,
        ),
        (
            Instruction::StoreMulFpImm {
                src_off: M31::from(0),
                imm: M31::from(0),
                dst_off: M31::from(0),
            },
            6,
        ),
        (
            Instruction::StoreDivFpImm {
                src_off: M31::from(0),
                imm: M31::from(0),
                dst_off: M31::from(0),
            },
            7,
        ),
        (
            Instruction::StoreDoubleDerefFp {
                base_off: M31::from(0),
                offset: M31::from(0),
                dst_off: M31::from(0),
            },
            8,
        ),
        (
            Instruction::StoreImm {
                imm: M31::from(0),
                dst_off: M31::from(0),
            },
            9,
        ),
        (
            Instruction::CallAbsImm {
                frame_off: M31::from(0),
                target: M31::from(0),
            },
            10,
        ),
        (Instruction::Ret {}, 11),
        (
            Instruction::JmpAbsImm {
                target: M31::from(0),
            },
            12,
        ),
        (
            Instruction::JmpRelImm {
                offset: M31::from(0),
            },
            13,
        ),
        (
            Instruction::JnzFpImm {
                cond_off: M31::from(0),
                offset: M31::from(0),
            },
            14,
        ),
        (
            Instruction::U32StoreAddFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            15,
        ),
        (
            Instruction::U32StoreSubFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            16,
        ),
        (
            Instruction::U32StoreMulFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            17,
        ),
        (
            Instruction::U32StoreDivFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(0),
                dst_off: M31::from(0),
            },
            18,
        ),
        (
            Instruction::U32StoreAddFpImm {
                src_off: M31::from(0),
                imm_hi: M31::from(0),
                imm_lo: M31::from(0),
                dst_off: M31::from(0),
            },
            19,
        ),
        (
            Instruction::U32StoreSubFpImm {
                src_off: M31::from(0),
                imm_hi: M31::from(0),
                imm_lo: M31::from(0),
                dst_off: M31::from(0),
            },
            20,
        ),
        (
            Instruction::U32StoreMulFpImm {
                src_off: M31::from(0),
                imm_hi: M31::from(0),
                imm_lo: M31::from(0),
                dst_off: M31::from(0),
            },
            21,
        ),
        (
            Instruction::U32StoreDivFpImm {
                src_off: M31::from(0),
                imm_hi: M31::from(0),
                imm_lo: M31::from(0),
                dst_off: M31::from(0),
            },
            22,
        ),
    ];

    for (instruction, expected_opcode) in test_cases {
        assert_eq!(instruction.opcode_value(), expected_opcode);
    }
}

#[test]
fn test_memory_accesses() {
    assert_eq!(
        Instruction::StoreAddFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .memory_accesses(),
        3
    );
    assert_eq!(
        Instruction::StoreAddFpImm {
            src_off: M31::from(0),
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .memory_accesses(),
        2
    );
    assert_eq!(
        Instruction::StoreImm {
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .memory_accesses(),
        1
    );
    assert_eq!(
        Instruction::JmpAbsImm {
            target: M31::from(0)
        }
        .memory_accesses(),
        0
    );
    assert_eq!(Instruction::Ret {}.memory_accesses(), 2);
}

#[test]
fn test_instruction_names() {
    assert_eq!(
        Instruction::StoreAddFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .name(),
        "StoreAddFpFp"
    );
    assert_eq!(Instruction::Ret {}.name(), "Ret");
    assert_eq!(
        Instruction::U32StoreAddFpImm {
            src_off: M31::from(0),
            imm_hi: M31::from(0),
            imm_lo: M31::from(0),
            dst_off: M31::from(0)
        }
        .name(),
        "U32StoreAddFpImm"
    );
}

#[test]
fn test_to_m31_vec() {
    let ret = Instruction::Ret {};
    let vec = ret.to_m31_vec();
    assert_eq!(vec.len(), 1);
    assert_eq!(vec[0], M31::from(11));

    let store_imm = Instruction::StoreImm {
        imm: M31::from(42),
        dst_off: M31::from(3),
    };
    let vec = store_imm.to_m31_vec();
    assert_eq!(vec.len(), 3);
    assert_eq!(vec[0], M31::from(9)); // opcode
    assert_eq!(vec[1], M31::from(42)); // imm
    assert_eq!(vec[2], M31::from(3)); // dst_off

    let add_fp_fp = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };
    let vec = add_fp_fp.to_m31_vec();
    assert_eq!(vec.len(), 4);
    assert_eq!(vec[0], M31::from(0)); // opcode
    assert_eq!(vec[1], M31::from(1)); // src0_off
    assert_eq!(vec[2], M31::from(2)); // src1_off
    assert_eq!(vec[3], M31::from(3)); // dst_off
}

#[test]
fn test_operands() {
    let ret = Instruction::Ret {};
    assert_eq!(ret.operands(), vec![]);

    let store_imm = Instruction::StoreImm {
        imm: M31::from(42),
        dst_off: M31::from(3),
    };
    assert_eq!(store_imm.operands(), vec![M31::from(42), M31::from(3)]);

    let add_fp_fp = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };
    assert_eq!(
        add_fp_fp.operands(),
        vec![M31::from(1), M31::from(2), M31::from(3)]
    );
}

#[test]
fn test_try_from_smallvec() {
    // Test cases: (smallvec_values, expected_instruction, description)
    let test_cases: Vec<(SmallVec<[M31; INSTRUCTION_MAX_SIZE]>, Instruction, &str)> = vec![
        // Basic instructions
        (
            smallvec![M31::from(11)],
            Instruction::Ret {},
            "Ret instruction",
        ),
        (
            smallvec![M31::from(9), M31::from(42), M31::from(3)],
            Instruction::StoreImm {
                imm: M31::from(42),
                dst_off: M31::from(3),
            },
            "StoreImm instruction",
        ),
        // Felt arithmetic operations
        (
            smallvec![M31::from(0), M31::from(1), M31::from(2), M31::from(3)],
            Instruction::StoreAddFpFp {
                src0_off: M31::from(1),
                src1_off: M31::from(2),
                dst_off: M31::from(3),
            },
            "StoreAddFpFp instruction",
        ),
        (
            smallvec![M31::from(1), M31::from(4), M31::from(5), M31::from(6)],
            Instruction::StoreSubFpFp {
                src0_off: M31::from(4),
                src1_off: M31::from(5),
                dst_off: M31::from(6),
            },
            "StoreSubFpFp instruction",
        ),
        (
            smallvec![M31::from(2), M31::from(7), M31::from(8), M31::from(9)],
            Instruction::StoreMulFpFp {
                src0_off: M31::from(7),
                src1_off: M31::from(8),
                dst_off: M31::from(9),
            },
            "StoreMulFpFp instruction",
        ),
        (
            smallvec![M31::from(3), M31::from(10), M31::from(11), M31::from(12)],
            Instruction::StoreDivFpFp {
                src0_off: M31::from(10),
                src1_off: M31::from(11),
                dst_off: M31::from(12),
            },
            "StoreDivFpFp instruction",
        ),
        // Felt arithmetic with immediate
        (
            smallvec![M31::from(4), M31::from(1), M31::from(100), M31::from(3)],
            Instruction::StoreAddFpImm {
                src_off: M31::from(1),
                imm: M31::from(100),
                dst_off: M31::from(3),
            },
            "StoreAddFpImm instruction",
        ),
        (
            smallvec![M31::from(5), M31::from(2), M31::from(200), M31::from(4)],
            Instruction::StoreSubFpImm {
                src_off: M31::from(2),
                imm: M31::from(200),
                dst_off: M31::from(4),
            },
            "StoreSubFpImm instruction",
        ),
        (
            smallvec![M31::from(6), M31::from(3), M31::from(300), M31::from(5)],
            Instruction::StoreMulFpImm {
                src_off: M31::from(3),
                imm: M31::from(300),
                dst_off: M31::from(5),
            },
            "StoreMulFpImm instruction",
        ),
        (
            smallvec![M31::from(7), M31::from(4), M31::from(400), M31::from(6)],
            Instruction::StoreDivFpImm {
                src_off: M31::from(4),
                imm: M31::from(400),
                dst_off: M31::from(6),
            },
            "StoreDivFpImm instruction",
        ),
        // U32 arithmetic operations
        (
            smallvec![M31::from(15), M31::from(1), M31::from(2), M31::from(3)],
            Instruction::U32StoreAddFpFp {
                src0_off: M31::from(1),
                src1_off: M31::from(2),
                dst_off: M31::from(3),
            },
            "U32StoreAddFpFp instruction",
        ),
        (
            smallvec![M31::from(16), M31::from(4), M31::from(5), M31::from(6)],
            Instruction::U32StoreSubFpFp {
                src0_off: M31::from(4),
                src1_off: M31::from(5),
                dst_off: M31::from(6),
            },
            "U32StoreSubFpFp instruction",
        ),
        (
            smallvec![M31::from(17), M31::from(7), M31::from(8), M31::from(9)],
            Instruction::U32StoreMulFpFp {
                src0_off: M31::from(7),
                src1_off: M31::from(8),
                dst_off: M31::from(9),
            },
            "U32StoreMulFpFp instruction",
        ),
        (
            smallvec![M31::from(18), M31::from(10), M31::from(11), M31::from(12)],
            Instruction::U32StoreDivFpFp {
                src0_off: M31::from(10),
                src1_off: M31::from(11),
                dst_off: M31::from(12),
            },
            "U32StoreDivFpFp instruction",
        ),
        // U32 arithmetic with immediate
        (
            smallvec![
                M31::from(19),
                M31::from(1),
                M31::from(0x1234),
                M31::from(0x5678),
                M31::from(4)
            ],
            Instruction::U32StoreAddFpImm {
                src_off: M31::from(1),
                imm_hi: M31::from(0x1234),
                imm_lo: M31::from(0x5678),
                dst_off: M31::from(4),
            },
            "U32StoreAddFpImm instruction",
        ),
        (
            smallvec![
                M31::from(20),
                M31::from(2),
                M31::from(0xabcd),
                M31::from(0xef01),
                M31::from(5)
            ],
            Instruction::U32StoreSubFpImm {
                src_off: M31::from(2),
                imm_hi: M31::from(0xabcd),
                imm_lo: M31::from(0xef01),
                dst_off: M31::from(5),
            },
            "U32StoreSubFpImm instruction",
        ),
        (
            smallvec![
                M31::from(21),
                M31::from(3),
                M31::from(0x2345),
                M31::from(0x6789),
                M31::from(6)
            ],
            Instruction::U32StoreMulFpImm {
                src_off: M31::from(3),
                imm_hi: M31::from(0x2345),
                imm_lo: M31::from(0x6789),
                dst_off: M31::from(6),
            },
            "U32StoreMulFpImm instruction",
        ),
        (
            smallvec![
                M31::from(22),
                M31::from(4),
                M31::from(0xcdef),
                M31::from(0x0123),
                M31::from(7)
            ],
            Instruction::U32StoreDivFpImm {
                src_off: M31::from(4),
                imm_hi: M31::from(0xcdef),
                imm_lo: M31::from(0x0123),
                dst_off: M31::from(7),
            },
            "U32StoreDivFpImm instruction",
        ),
        // Memory and control flow operations
        (
            smallvec![M31::from(8), M31::from(9), M31::from(10), M31::from(11)],
            Instruction::StoreDoubleDerefFp {
                base_off: M31::from(9),
                offset: M31::from(10),
                dst_off: M31::from(11),
            },
            "StoreDoubleDerefFp instruction",
        ),
        (
            smallvec![M31::from(10), M31::from(19), M31::from(1000)],
            Instruction::CallAbsImm {
                frame_off: M31::from(19),
                target: M31::from(1000),
            },
            "CallAbsImm instruction",
        ),
        (
            smallvec![M31::from(12), M31::from(2000)],
            Instruction::JmpAbsImm {
                target: M31::from(2000),
            },
            "JmpAbsImm instruction",
        ),
        (
            smallvec![M31::from(13), M31::from(50)],
            Instruction::JmpRelImm {
                offset: M31::from(50),
            },
            "JmpRelImm instruction",
        ),
        (
            smallvec![M31::from(14), M31::from(21), M31::from(60)],
            Instruction::JnzFpImm {
                cond_off: M31::from(21),
                offset: M31::from(60),
            },
            "JnzFpImm instruction",
        ),
    ];

    assert_eq!(test_cases.len(), MAX_OPCODE as usize + 1);

    for (values, expected_instruction, description) in test_cases {
        let instruction = Instruction::try_from(values)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {:?}", description, e));
        assert_eq!(
            instruction, expected_instruction,
            "Mismatch for {}",
            description
        );
    }
}

#[test]
fn test_try_from_smallvec_errors() {
    // Test empty smallvec
    let values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = SmallVec::new();
    let result = Instruction::try_from(values);
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 1,
            found: 0
        })
    ));

    // Test invalid opcode
    let values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = smallvec![M31::from(999)];
    let result = Instruction::try_from(values);
    assert!(matches!(result, Err(InstructionError::InvalidOpcode(_))));

    // Test size mismatch - too few operands
    let values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = smallvec![M31::from(0), M31::from(1)]; // StoreAddFpFp needs 3 operands
    let result = Instruction::try_from(values);
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 3,
            found: 1
        })
    ));

    // Test size mismatch - too many operands
    let values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = smallvec![M31::from(11), M31::from(1)]; // Ret needs 0 operands
    let result = Instruction::try_from(values);
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 0,
            found: 1
        })
    ));
}

#[test]
fn test_from_instruction_to_smallvec() {
    let instruction = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };

    // Test From<Instruction>
    let smallvec: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = instruction.into();
    assert_eq!(
        smallvec.as_slice(),
        &[M31::from(0), M31::from(1), M31::from(2), M31::from(3)]
    );

    // Test From<&Instruction>
    let instruction = Instruction::StoreImm {
        imm: M31::from(42),
        dst_off: M31::from(3),
    };
    let smallvec: SmallVec<[M31; INSTRUCTION_MAX_SIZE]> = (&instruction).into();
    assert_eq!(
        smallvec.as_slice(),
        &[M31::from(9), M31::from(42), M31::from(3)]
    );
}

#[test]
fn test_to_qm31_vec() {
    // Test size 1 instruction (Ret) - needs padding
    let ret = Instruction::Ret {};
    let qm31_vec = ret.to_qm31_vec();
    assert_eq!(qm31_vec.len(), 1);
    let m31_array = qm31_vec[0].to_m31_array();
    assert_eq!(m31_array[0], M31::from(11));
    assert_eq!(m31_array[1], M31::from(0)); // padded
    assert_eq!(m31_array[2], M31::from(0)); // padded
    assert_eq!(m31_array[3], M31::from(0)); // padded

    // Test size 4 instruction (StoreAddFpFp) - fits exactly
    let add_fp_fp = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };
    let qm31_vec = add_fp_fp.to_qm31_vec();
    assert_eq!(qm31_vec.len(), 1);
    let m31_array = qm31_vec[0].to_m31_array();
    assert_eq!(m31_array[0], M31::from(0)); // opcode
    assert_eq!(m31_array[1], M31::from(1)); // src0_off
    assert_eq!(m31_array[2], M31::from(2)); // src1_off
    assert_eq!(m31_array[3], M31::from(3)); // dst_off

    // Test size 5 instruction (U32StoreAddFpImm) - needs 2 QM31s
    let u32_add = Instruction::U32StoreAddFpImm {
        src_off: M31::from(1),
        imm_hi: M31::from(0x1234),
        imm_lo: M31::from(0x5678),
        dst_off: M31::from(4),
    };
    let qm31_vec = u32_add.to_qm31_vec();
    assert_eq!(qm31_vec.len(), 2);

    let m31_array = qm31_vec[0].to_m31_array();
    assert_eq!(m31_array[0], M31::from(19)); // opcode
    assert_eq!(m31_array[1], M31::from(1)); // src_off
    assert_eq!(m31_array[2], M31::from(0x1234)); // imm_hi
    assert_eq!(m31_array[3], M31::from(0x5678)); // imm_lo

    let m31_array = qm31_vec[1].to_m31_array();
    assert_eq!(m31_array[0], M31::from(4)); // dst_off
    assert_eq!(m31_array[1], M31::from(0)); // padded
    assert_eq!(m31_array[2], M31::from(0)); // padded
    assert_eq!(m31_array[3], M31::from(0)); // padded
}

#[test]
fn test_serialization() {
    use serde_json;

    let instruction = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&instruction).unwrap();
    assert_eq!(json, r#"["0x0","0x1","0x2","0x3"]"#);

    // Deserialize from JSON
    let deserialized: Instruction = serde_json::from_str(&json).unwrap();
    assert_eq!(instruction, deserialized);

    // Test with larger values (but within M31 bounds)
    let instruction = Instruction::StoreImm {
        imm: M31::from(0x1234567),
        dst_off: M31::from(0xcafe),
    };
    let json = serde_json::to_string(&instruction).unwrap();
    assert_eq!(json, r#"["0x9","0x1234567","0xcafe"]"#);

    let deserialized: Instruction = serde_json::from_str(&json).unwrap();
    assert_eq!(instruction, deserialized);
}

#[test]
fn test_roundtrip_all_instructions() {
    // Test roundtrip for all instruction types
    let instructions = vec![
        Instruction::StoreAddFpFp {
            src0_off: M31::from(1),
            src1_off: M31::from(2),
            dst_off: M31::from(3),
        },
        Instruction::StoreAddFpImm {
            src_off: M31::from(1),
            imm: M31::from(100),
            dst_off: M31::from(3),
        },
        Instruction::StoreSubFpFp {
            src0_off: M31::from(4),
            src1_off: M31::from(5),
            dst_off: M31::from(6),
        },
        Instruction::StoreSubFpImm {
            src_off: M31::from(4),
            imm: M31::from(200),
            dst_off: M31::from(6),
        },
        Instruction::StoreDoubleDerefFp {
            base_off: M31::from(9),
            offset: M31::from(10),
            dst_off: M31::from(11),
        },
        Instruction::StoreImm {
            imm: M31::from(300),
            dst_off: M31::from(12),
        },
        Instruction::StoreMulFpFp {
            src0_off: M31::from(13),
            src1_off: M31::from(14),
            dst_off: M31::from(15),
        },
        Instruction::StoreMulFpImm {
            src_off: M31::from(13),
            imm: M31::from(400),
            dst_off: M31::from(15),
        },
        Instruction::StoreDivFpFp {
            src0_off: M31::from(16),
            src1_off: M31::from(17),
            dst_off: M31::from(18),
        },
        Instruction::StoreDivFpImm {
            src_off: M31::from(16),
            imm: M31::from(500),
            dst_off: M31::from(18),
        },
        Instruction::CallAbsImm {
            frame_off: M31::from(19),
            target: M31::from(1000),
        },
        Instruction::Ret {},
        Instruction::JmpAbsImm {
            target: M31::from(2000),
        },
        Instruction::JmpRelImm {
            offset: M31::from(50),
        },
        Instruction::JnzFpImm {
            cond_off: M31::from(21),
            offset: M31::from(60),
        },
        Instruction::U32StoreAddFpImm {
            src_off: M31::from(22),
            imm_hi: M31::from(0x1234),
            imm_lo: M31::from(0x5678),
            dst_off: M31::from(23),
        },
    ];

    for instruction in instructions {
        // Test smallvec roundtrip
        let vec = instruction.to_m31_vec();
        let smallvec = SmallVec::from_vec(vec);
        let reconstructed = Instruction::try_from(smallvec).unwrap();
        assert_eq!(instruction, reconstructed);

        // Test JSON roundtrip
        let json = serde_json::to_string(&instruction).unwrap();
        let deserialized: Instruction = serde_json::from_str(&json).unwrap();
        assert_eq!(instruction, deserialized);
    }
}
