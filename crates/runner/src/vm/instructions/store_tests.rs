use num_traits::Zero;
use proptest::{prelude::*, proptest};
use stwo_prover::core::fields::m31::M31;

use crate::memory::MemoryError;

use super::{InstructionExecutionError, *};

// ---------------------------------------------------------------------------
// Generic helpers (scalar instructions)
// ---------------------------------------------------------------------------

/// Signature of every store-style execution helper.
type ExecFn = fn(&mut Memory, State, &Instruction) -> Result<State, InstructionExecutionError>;

/// Run an instruction end-to-end and assert on *all* side-effects.
fn run_store_test(
    initial_mem: &[u32],
    state: State,
    instruction: Instruction,
    exec_fn: ExecFn,
    expected_mem: &[u32],
    expected_pc: u32,
    expected_fp: u32,
) -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter(initial_mem.iter().copied().map(Into::into));
    let new_state = exec_fn(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter(expected_mem.iter().copied().map(Into::into));
    assert_eq!(
        memory.data, expected_memory.data,
        "memory mismatch after executing {instruction:?}"
    );
    assert_eq!(new_state.pc, M31(expected_pc));
    assert_eq!(new_state.fp, M31(expected_fp));

    Ok(())
}

/// Same as [`run_store_test`] but starting from `State::default()`.
fn run_simple_store_test(
    initial_mem: &[u32],
    instruction: Instruction,
    exec_fn: ExecFn,
    expected_mem: &[u32],
    expected_pc: u32,
) -> Result<(), InstructionExecutionError> {
    run_store_test(
        initial_mem,
        State::default(),
        instruction,
        exec_fn,
        expected_mem,
        expected_pc,
        0,
    )
}

/// Run an FP-IMM-style U32 instruction and validate state + result.
fn run_u32_fp_imm_test(
    src_value: u32,
    instruction: Instruction,
    exec_fn: ExecFn,
    expected_res: u32,
    dst_off: u32,
    expected_pc: u32,
) -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);
    memory.insert_u32(initial_fp, src_value).unwrap();
    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };

    let new_state = exec_fn(&mut memory, state, &instruction)?;
    assert_eq!(
        memory.get_u32(initial_fp + M31(dst_off)).unwrap(),
        expected_res
    );
    assert_eq!(new_state.pc, M31(expected_pc));
    assert_eq!(new_state.fp, initial_fp);
    Ok(())
}

/// Run an FP-FP-style U32 instruction and validate state + result.
fn run_u32_fp_fp_test(
    src0: u32,
    src1: u32,
    instruction: Instruction,
    exec_fn: ExecFn,
    expected_res: u32,
    dst_off: u32,
    expected_pc: u32,
) -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);
    memory.insert_u32(initial_fp, src0).unwrap(); // fp+0 / fp+1
    memory.insert_u32(initial_fp + M31(2), src1).unwrap(); // fp+2 / fp+3
    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };

    let new_state = exec_fn(&mut memory, state, &instruction)?;
    assert_eq!(
        memory.get_u32(initial_fp + M31(dst_off)).unwrap(),
        expected_res
    );
    assert_eq!(new_state.pc, M31(expected_pc));
    assert_eq!(new_state.fp, initial_fp);
    Ok(())
}

// -----------------------------------------------------------------------------
// Scalar store_* instruction tests
// -----------------------------------------------------------------------------

proptest! {
    #[test]
    fn test_store_add_fp_fp(src0_val: u32, src1_val: u32) {
        let src0 = M31::from(src0_val);
        let src1 = M31::from(src1_val);
        let expected_res = src0 + src1;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src0_val;
        initial_mem[2] = src1_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreAddFpFp {
                src0_off: M31(1),
                src1_off: M31(2),
                dst_off: M31(3),
            },
            store_add_fp_fp,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_add_fp_imm(src_val: u32, imm_val: u32) {
        let src = M31::from(src_val);
        let imm = M31::from(imm_val);
        let expected_res = src + imm;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreAddFpImm {
                src_off: M31(1),
                imm,
                dst_off: M31(3),
            },
            store_add_fp_imm,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_sub_fp_fp(src0_val: u32, src1_val: u32) {
        let src0 = M31::from(src0_val);
        let src1 = M31::from(src1_val);
        let expected_res = src0 - src1;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src0_val;
        initial_mem[2] = src1_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreSubFpFp {
                src0_off: M31(1),
                src1_off: M31(2),
                dst_off: M31(3),
            },
            store_sub_fp_fp,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_sub_fp_imm(src_val: u32, imm_val: u32) {
        let src = M31::from(src_val);
        let imm = M31::from(imm_val);
        let expected_res = src - imm;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreSubFpImm {
                src_off: M31(1),
                imm,
                dst_off: M31(3),
            },
            store_sub_fp_imm,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_double_deref_fp(val_to_store: u32, dst_val: u32) {
        run_simple_store_test(
            &[0, 1, dst_val, val_to_store],
            Instruction::StoreDoubleDerefFp {
                base_off: M31(1),
                offset: M31(2),
                dst_off: M31(2),
            },
            store_double_deref_fp,
            &[0, 1, val_to_store, val_to_store],
            1,
        )
        .unwrap();
    }

    #[test]
    fn test_store_imm(imm_val: u32) {
        let imm = M31::from(imm_val);
        run_simple_store_test(
            &[0, 4],
            Instruction::StoreImm {
                imm,
                dst_off: M31(2),
            },
            store_imm,
            &[0, 4, imm.0],
            1,
        )
        .unwrap();
    }

    #[test]
    fn test_store_mul_fp_fp(src0_val: u32, src1_val: u32) {
        let src0 = M31::from(src0_val);
        let src1 = M31::from(src1_val);
        let expected_res = src0 * src1;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src0_val;
        initial_mem[2] = src1_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreMulFpFp {
                src0_off: M31(1),
                src1_off: M31(2),
                dst_off: M31(3),
            },
            store_mul_fp_fp,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_mul_fp_imm(src_val: u32, imm_val: u32) {
        let src = M31::from(src_val);
        let imm = M31::from(imm_val);
        let expected_res = src * imm;
        run_simple_store_test(
            &[0, src_val],
            Instruction::StoreMulFpImm {
                src_off: M31(1),
                imm,
                dst_off: M31(2),
            },
            store_mul_fp_imm,
            &[0, src_val, expected_res.0],
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_div_fp_fp(src0_val: u32, src1_val: u32) {
        let src1 = M31::from(src1_val);
        prop_assume!(src1 != M31::zero(), "division by zero");

        let src0 = M31::from(src0_val);
        let expected_res = src0 / src1;

        let mut initial_mem = vec![0; 4];
        initial_mem[1] = src0_val;
        initial_mem[2] = src1_val;
        let mut expected_mem = initial_mem.clone();
        expected_mem[3] = expected_res.0;

        run_simple_store_test(
            &initial_mem,
            Instruction::StoreDivFpFp {
                src0_off: M31(1),
                src1_off: M31(2),
                dst_off: M31(3),
            },
            store_div_fp_fp,
            &expected_mem,
            1,
        ).unwrap();
    }

    #[test]
    fn test_store_div_fp_imm(src_val: u32, imm_val: u32) {
        let imm = M31::from(imm_val);
        prop_assume!(imm != M31::zero(), "division by zero");

        let src = M31::from(src_val);
        let expected_res = src / imm;
        run_simple_store_test(
            &[0, src_val],
            Instruction::StoreDivFpImm {
                src_off: M31(1),
                imm,
                dst_off: M31(2),
            },
            store_div_fp_imm,
            &[0, src_val, expected_res.0],
            1,
        ).unwrap();
    }
}

#[test]
#[should_panic(expected = "0 has no inverse")]
fn test_store_div_fp_fp_by_zero() {
    let mut memory = Memory::from_iter([0u32, 4, 0].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let _ = store_div_fp_fp(&mut memory, state, &instruction);
}

#[test]
#[should_panic(expected = "0 has no inverse")]
fn test_store_div_fp_imm_by_zero() {
    let mut memory = Memory::from_iter([0u32, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpImm {
        src_off: M31(1),
        imm: M31(0),
        dst_off: M31(2),
    };

    let _ = store_div_fp_imm(&mut memory, state, &instruction);
}

// -----------------------------------------------------------------------------
// U32 FP-IMM instruction tests
// -----------------------------------------------------------------------------

proptest! {
    #[test]
    fn test_u32_store_add_fp_imm(src_value: u32, imm_val_hi in 0..=u16::MAX as u32, imm_val_lo in 0..=u16::MAX as u32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_add(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreAddFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_add_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_add_fp_imm_invalid_limbs(imm_val_lo: u32, imm_val_hi: u32) {
        prop_assume!(imm_val_lo > U32_LIMB_MASK || imm_val_hi > U32_LIMB_MASK);
        let err = run_u32_fp_imm_test(
            0,
            Instruction::U32StoreAddFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_add_fp_imm,
            0,
            2,
            2,
        );
        assert_eq!(err.unwrap_err(), InstructionExecutionError::Memory(MemoryError::U32LimbOutOfRange {
            limb_lo: imm_val_lo,
            limb_hi: imm_val_hi,
        }));
    }

    #[test]
    fn test_u32_store_sub_fp_imm(src_value: u32, imm_val_hi in 0..=u16::MAX as u32, imm_val_lo in 0..=u16::MAX as u32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_sub(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreSubFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_sub_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_sub_fp_imm_invalid_limbs(src_value: u32, imm_val_lo: u32, imm_val_hi: u32) {
        prop_assume!(imm_val_lo > U32_LIMB_MASK || imm_val_hi > U32_LIMB_MASK);
        let err = run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreSubFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_sub_fp_imm,
            0,
            2,
            2,
        );
        assert_eq!(err.unwrap_err(), InstructionExecutionError::Memory(MemoryError::U32LimbOutOfRange {
            limb_lo: imm_val_lo,
            limb_hi: imm_val_hi,
        }));
    }

    #[test]
    fn test_u32_store_mul_fp_imm(src_value: u32, imm_val_hi in 0..=u16::MAX as u32, imm_val_lo in 0..=u16::MAX as u32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_mul(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreMulFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_mul_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_mul_fp_imm_invalid_limbs(imm_val_lo: u32, imm_val_hi: u32) {
        prop_assume!(imm_val_lo > U32_LIMB_MASK || imm_val_hi > U32_LIMB_MASK);
        let err = run_u32_fp_imm_test(
            0,
            Instruction::U32StoreMulFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_mul_fp_imm,
            0,
            2,
            2,
        );
        assert_eq!(err.unwrap_err(), InstructionExecutionError::Memory(MemoryError::U32LimbOutOfRange {
            limb_lo: imm_val_lo,
            limb_hi: imm_val_hi,
        }));
    }

    #[test]
    fn test_u32_store_div_fp_imm(src_value: u32, imm_val_hi in 0..=u16::MAX as u32, imm_val_lo in 0..=u16::MAX as u32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        prop_assume!(imm_val != 0, "attempt to divide by zero");
        let expected_res = src_value / imm_val;
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreDivFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_div_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn test_u32_store_div_fp_imm_by_zero(src_value in 0..u16::MAX as u32) {
        let _ = run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreDivFpImm {
                src_off: M31(0),
                imm_lo: M31(0x0000),
                imm_hi: M31(0x0000),
                dst_off: M31(2),
            },
            u32_store_div_fp_imm,
            0,
            2,
            2,
        );
    }


    #[test]
    fn test_u32_store_div_fp_imm_invalid_limbs(imm_val_lo: u32, imm_val_hi: u32) {
        prop_assume!(imm_val_lo > U32_LIMB_MASK || imm_val_hi > U32_LIMB_MASK);
        let err = run_simple_store_test(
            &[0, 4],
            Instruction::U32StoreDivFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_div_fp_imm,
            &[0, 4, imm_val_lo, imm_val_hi],
            1,
        );
        assert_eq!(err.unwrap_err(), InstructionExecutionError::Memory(MemoryError::U32LimbOutOfRange {
            limb_lo: imm_val_lo,
            limb_hi: imm_val_hi,
        }));
    }
}

// -----------------------------------------------------------------------------
// U32 FP-FP instruction tests
// -----------------------------------------------------------------------------

proptest! {
    #[test]
    fn test_u32_store_add_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value.wrapping_add(src1_value);
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreAddFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_add_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_sub_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value.wrapping_sub(src1_value);
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreSubFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_sub_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_mul_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value.wrapping_mul(src1_value);
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreMulFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_mul_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_div_fp_fp(src0_value: u32, src1_value: u32) {
        prop_assume!(src1_value != 0, "attempt to divide by zero");
        let expected_res = src0_value / src1_value;
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreDivFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_div_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_imm(imm_val_lo in 0..=u16::MAX as u32, imm_val_hi in 0..=u16::MAX as u32) {
        run_simple_store_test(
            &[0, 4],
            Instruction::U32StoreImm {
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_imm,
            &[0, 4, imm_val_lo, imm_val_hi],
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_imm_invalid_limbs(imm_val_lo: u32, imm_val_hi: u32) {
        prop_assume!(imm_val_lo > U32_LIMB_MASK || imm_val_hi > U32_LIMB_MASK);
        let err = run_simple_store_test(
            &[0, 4],
            Instruction::U32StoreImm {
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_imm,
            &[0, 4, imm_val_lo, imm_val_hi],
            1,
        );
        assert_eq!(err.unwrap_err(), InstructionExecutionError::Memory(MemoryError::U32LimbOutOfRange {
            limb_lo: imm_val_lo,
            limb_hi: imm_val_hi,
        }));
    }
}

// -----------------------------------------------------------------------------
// U32 Bitwise FP-FP instruction tests
// -----------------------------------------------------------------------------

proptest! {
    #[test]
    fn test_u32_store_and_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value & src1_value;
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreAndFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_and_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_or_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value | src1_value;
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreOrFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_or_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_xor_fp_fp(src0_value: u32, src1_value: u32) {
        let expected_res = src0_value ^ src1_value;
        run_u32_fp_fp_test(
            src0_value,
            src1_value,
            Instruction::U32StoreXorFpFp {
                src0_off: M31(0),
                src1_off: M31(2),
                dst_off: M31(4),
            },
            u32_store_xor_fp_fp,
            expected_res,
            4,
            1,
        ).unwrap();
    }
}

// -----------------------------------------------------------------------------
// U32 Bitwise FP-IMM instruction tests
// -----------------------------------------------------------------------------

proptest! {
    #[test]
    fn test_u32_store_and_fp_imm(src_value: u32, imm_val_lo in 0..=u16::MAX as u32, imm_val_hi in 0..=u16::MAX as u32) {
        let imm_value: u32 = (imm_val_hi << U32_LIMB_BITS) | imm_val_lo;
        let expected_res = src_value & imm_value;
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreAndFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_and_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_or_fp_imm(src_value: u32, imm_val_lo in 0..=u16::MAX as u32, imm_val_hi in 0..=u16::MAX as u32) {
        let imm_value: u32 = (imm_val_hi << U32_LIMB_BITS) | imm_val_lo;
        let expected_res = src_value | imm_value;
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreOrFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_or_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_xor_fp_imm(src_value: u32, imm_val_lo in 0..=u16::MAX as u32, imm_val_hi in 0..=u16::MAX as u32) {
        let imm_value: u32 = (imm_val_hi << U32_LIMB_BITS) | imm_val_lo;
        let expected_res = src_value ^ imm_value;
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreXorFpImm {
                src_off: M31(0),
                imm_lo: M31(imm_val_lo),
                imm_hi: M31(imm_val_hi),
                dst_off: M31(2),
            },
            u32_store_xor_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }
}
