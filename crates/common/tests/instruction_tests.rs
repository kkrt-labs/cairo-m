use cairo_m_common::{Instruction, InstructionError};
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
    assert_eq!(
        Instruction::StoreAddFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        0
    );
    assert_eq!(
        Instruction::StoreAddFpImm {
            src_off: M31::from(0),
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        1
    );
    assert_eq!(
        Instruction::StoreSubFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        2
    );
    assert_eq!(
        Instruction::StoreSubFpImm {
            src_off: M31::from(0),
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        3
    );
    assert_eq!(
        Instruction::StoreDoubleDerefFp {
            base_off: M31::from(0),
            offset: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        4
    );
    assert_eq!(
        Instruction::StoreImm {
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        5
    );
    assert_eq!(
        Instruction::StoreMulFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        6
    );
    assert_eq!(
        Instruction::StoreMulFpImm {
            src_off: M31::from(0),
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        7
    );
    assert_eq!(
        Instruction::StoreDivFpFp {
            src0_off: M31::from(0),
            src1_off: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        8
    );
    assert_eq!(
        Instruction::StoreDivFpImm {
            src_off: M31::from(0),
            imm: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        9
    );
    assert_eq!(
        Instruction::CallAbsImm {
            frame_off: M31::from(0),
            target: M31::from(0)
        }
        .opcode_value(),
        10
    );
    assert_eq!(Instruction::Ret {}.opcode_value(), 11);
    assert_eq!(
        Instruction::JmpAbsImm {
            target: M31::from(0)
        }
        .opcode_value(),
        12
    );
    assert_eq!(
        Instruction::JmpRelImm {
            offset: M31::from(0)
        }
        .opcode_value(),
        13
    );
    assert_eq!(
        Instruction::JnzFpImm {
            cond_off: M31::from(0),
            offset: M31::from(0)
        }
        .opcode_value(),
        14
    );
    assert_eq!(
        Instruction::U32StoreAddFpImm {
            src_off: M31::from(0),
            imm_hi: M31::from(0),
            imm_lo: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        15
    );
    assert_eq!(
        Instruction::U32StoreAddFpImm {
            src_off: M31::from(0),
            imm_hi: M31::from(0),
            imm_lo: M31::from(0),
            dst_off: M31::from(0)
        }
        .opcode_value(),
        15
    );
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
    assert_eq!(vec[0], M31::from(5)); // opcode
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
fn test_try_from_slice() {
    // Test Ret instruction
    let slice = &[M31::from(11)];
    let instruction = Instruction::try_from(slice.as_ref()).unwrap();
    assert!(matches!(instruction, Instruction::Ret {}));

    // Test StoreImm instruction
    let slice = &[M31::from(5), M31::from(42), M31::from(3)];
    let instruction = Instruction::try_from(slice.as_ref()).unwrap();
    match instruction {
        Instruction::StoreImm { imm, dst_off } => {
            assert_eq!(imm, M31::from(42));
            assert_eq!(dst_off, M31::from(3));
        }
        _ => panic!("Wrong instruction type"),
    }

    // Test StoreAddFpFp instruction
    let slice = &[M31::from(0), M31::from(1), M31::from(2), M31::from(3)];
    let instruction = Instruction::try_from(slice.as_ref()).unwrap();
    match instruction {
        Instruction::StoreAddFpFp {
            src0_off,
            src1_off,
            dst_off,
        } => {
            assert_eq!(src0_off, M31::from(1));
            assert_eq!(src1_off, M31::from(2));
            assert_eq!(dst_off, M31::from(3));
        }
        _ => panic!("Wrong instruction type"),
    }

    // Test U32StoreAddFpImm instruction
    let slice = &[
        M31::from(15),
        M31::from(1),
        M31::from(0x1234),
        M31::from(0x5678),
        M31::from(4),
    ];
    let instruction = Instruction::try_from(slice.as_ref()).unwrap();
    match instruction {
        Instruction::U32StoreAddFpImm {
            src_off,
            imm_hi,
            imm_lo,
            dst_off,
        } => {
            assert_eq!(src_off, M31::from(1));
            assert_eq!(imm_hi, M31::from(0x1234));
            assert_eq!(imm_lo, M31::from(0x5678));
            assert_eq!(dst_off, M31::from(4));
        }
        _ => panic!("Wrong instruction type"),
    }
}

#[test]
fn test_try_from_slice_errors() {
    // Test empty slice
    let slice: &[M31] = &[];
    let result = Instruction::try_from(slice);
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 1,
            found: 0
        })
    ));

    // Test invalid opcode
    let slice = &[M31::from(999)];
    let result = Instruction::try_from(slice.as_ref());
    assert!(matches!(result, Err(InstructionError::InvalidOpcode(_))));

    // Test size mismatch - too few operands
    let slice = &[M31::from(0), M31::from(1)]; // StoreAddFpFp needs 3 operands
    let result = Instruction::try_from(slice.as_ref());
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 3,
            found: 1
        })
    ));

    // Test size mismatch - too many operands
    let slice = &[M31::from(11), M31::from(1)]; // Ret needs 0 operands
    let result = Instruction::try_from(slice.as_ref());
    assert!(matches!(
        result,
        Err(InstructionError::SizeMismatch {
            expected: 0,
            found: 1
        })
    ));
}

#[test]
fn test_from_instruction_to_vec() {
    let instruction = Instruction::StoreAddFpFp {
        src0_off: M31::from(1),
        src1_off: M31::from(2),
        dst_off: M31::from(3),
    };

    // Test From<Instruction>
    let vec: Vec<M31> = instruction.into();
    assert_eq!(
        vec,
        vec![M31::from(0), M31::from(1), M31::from(2), M31::from(3)]
    );

    // Test From<&Instruction>
    let instruction = Instruction::StoreImm {
        imm: M31::from(42),
        dst_off: M31::from(3),
    };
    let vec: Vec<M31> = (&instruction).into();
    assert_eq!(vec, vec![M31::from(5), M31::from(42), M31::from(3)]);
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
    assert_eq!(m31_array[0], M31::from(15)); // opcode
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
    assert_eq!(json, r#"["0x5","0x1234567","0xcafe"]"#);

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
        // Test slice roundtrip
        let vec = instruction.to_m31_vec();
        let reconstructed = Instruction::try_from(vec.as_slice()).unwrap();
        assert_eq!(instruction, reconstructed);

        // Test JSON roundtrip
        let json = serde_json::to_string(&instruction).unwrap();
        let deserialized: Instruction = serde_json::from_str(&json).unwrap();
        assert_eq!(instruction, deserialized);
    }
}
