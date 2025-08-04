use num_traits::{One, Zero};
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

// ---------------------------------------------------------------------------
// U32-specific helpers
// ---------------------------------------------------------------------------

const LOW_MASK: u32 = 0xFFFF;

/// Split a `u32` into (`low`, `high`) 16-bit limbs.
fn split_u32(value: u32) -> (u32, u32) {
    (value & LOW_MASK, value >> 16)
}

/// Insert a 32-bit value into memory as two 16-bit limbs.
fn insert_u32(memory: &mut Memory, addr: M31, value: u32) {
    let (lo, hi) = split_u32(value);
    memory.insert(addr, M31(lo).into()).unwrap();
    memory.insert(addr + M31(1), M31(hi).into()).unwrap();
}

/// Assert that the memory at `addr` holds the 32-bit `expected` (low limb
/// first, high limb second).
fn assert_u32(memory: &Memory, addr: M31, expected: u32) {
    let (lo, hi) = split_u32(expected);
    assert_eq!(memory.get_data(addr).unwrap(), M31(lo));
    assert_eq!(memory.get_data(addr + M31(1)).unwrap(), M31(hi));
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
    insert_u32(&mut memory, initial_fp, src_value);
    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };

    let new_state = exec_fn(&mut memory, state, &instruction)?;
    assert_u32(&memory, initial_fp + M31(dst_off), expected_res);
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
    insert_u32(&mut memory, initial_fp, src0); // fp+0 / fp+1
    insert_u32(&mut memory, initial_fp + M31(2), src1); // fp+2 / fp+3
    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };

    let new_state = exec_fn(&mut memory, state, &instruction)?;
    assert_u32(&memory, initial_fp + M31(dst_off), expected_res);
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
    fn test_u32_store_add_fp_imm(src_value: u32, imm_val_hi in 0..=0xFFFFu32, imm_val_lo in 0..=0xFFFFu32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_add(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreAddFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_add_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_sub_fp_imm(src_value: u32, imm_val_hi in 0..=0xFFFFu32, imm_val_lo in 0..=0xFFFFu32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_sub(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreSubFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_sub_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_mul_fp_imm(src_value: u32, imm_val_hi in 0..=0xFFFFu32, imm_val_lo in 0..=0xFFFFu32) {
        let imm_val = (imm_val_hi << 16) | imm_val_lo;
        let expected_res = src_value.wrapping_mul(imm_val);
        run_u32_fp_imm_test(
            src_value,
            Instruction::U32StoreMulFpImm {
                src_off: M31(0),
                imm_hi: M31(imm_val_hi),
                imm_lo: M31(imm_val_lo),
                dst_off: M31(2),
            },
            u32_store_mul_fp_imm,
            expected_res,
            2,
            2,
        ).unwrap();
    }

    #[test]
    fn test_u32_store_div_fp_imm(src_value: u32, imm_val_hi in 0..=0xFFFFu32, imm_val_lo in 0..=0xFFFFu32) {
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
}

#[test]
fn test_u32_store_add_fp_imm_invalid_immediate_limbs() {
    // build memory with valid 0 value so the only failure comes from immediates
    let mut memory = Memory::default();
    insert_u32(&mut memory, M31::zero(), 0);

    let state = State::default();
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0x1_0000), // > 16-bit
        imm_lo: M31(0x1_0000), // > 16-bit
        dst_off: M31(2),
    };

    assert!(
        u32_store_add_fp_imm(&mut memory, state, &instruction)
            == Err(InstructionExecutionError::Memory(
                MemoryError::U32LimbOutOfRange {
                    limb_lo: 0x1_0000,
                    limb_hi: 0x1_0000,
                }
            ))
    );
}

#[test]
fn test_u32_store_add_fp_imm_invalid_source_limbs() {
    let mut memory = Memory::default();
    // Insert invalid (>16-bit) limbs into fp+0 / fp+1
    memory.insert(M31::zero(), M31(0x1_0000).into()).unwrap();
    memory.insert(M31::one(), M31(0x1_0000).into()).unwrap();

    let state = State::default();
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0),
        imm_lo: M31(0),
        dst_off: M31(2),
    };

    assert!(
        u32_store_add_fp_imm(&mut memory, state, &instruction)
            == Err(InstructionExecutionError::Memory(
                MemoryError::U32LimbOutOfRange {
                    limb_lo: 0x1_0000,
                    limb_hi: 0x1_0000,
                }
            ))
    );
}

#[test]
#[should_panic(expected = "attempt to divide by zero")]
fn test_u32_store_div_fp_imm_by_zero() {
    let src_value = 0x1234_5678;
    let _ = run_u32_fp_imm_test(
        src_value,
        Instruction::U32StoreDivFpImm {
            src_off: M31(0),
            imm_hi: M31(0x0000),
            imm_lo: M31(0x0000),
            dst_off: M31(2),
        },
        u32_store_div_fp_imm,
        0,
        2,
        2,
    );
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
}

#[test]
#[should_panic(expected = "attempt to divide by zero")]
fn test_u32_store_div_fp_fp_by_zero() {
    let src0_value = 0x1234_5678;
    let src1_value = 0x0000_0000;
    let _ = run_u32_fp_fp_test(
        src0_value,
        src1_value,
        Instruction::U32StoreDivFpFp {
            src0_off: M31(0),
            src1_off: M31(2),
            dst_off: M31(4),
        },
        u32_store_div_fp_fp,
        0,
        4,
        1,
    );
}
