use num_traits::{One, Zero};
use stwo_prover::core::fields::m31::M31;

use super::{InstructionExecutionError, *};
use crate::vm::test_utils::*;

#[test]
fn test_store_add_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreAddFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_add_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 11].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);

    assert_eq!(new_state.fp, M31::zero());
    assert_eq!(new_state.pc, M31::one());

    Ok(())
}

#[test]
fn test_store_add_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let expected_memory = Memory::from_iter([0, 4, 7, 6].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreAddFpImm {
        src_off: M31(1),
        imm: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_add_fp_imm(&mut memory, state, &instruction)?;

    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_sub_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 7, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreSubFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_sub_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 7, 4, 3].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_sub_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreSubFpImm {
        src_off: M31(1),
        imm: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_sub_fp_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 2].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_double_deref_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 1, 7, 9].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDoubleDerefFp {
        base_off: M31(1),
        offset: M31(2),
        dst_off: M31(2),
    };

    let new_state = store_double_deref_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 1, 9, 9].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreImm {
        imm: M31(1),
        dst_off: M31(2),
    };

    let new_state = store_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 1].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);

    assert_eq!(new_state.fp, M31::zero());
    assert_eq!(new_state.pc, M31::one());

    Ok(())
}

#[test]
fn test_store_mul_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreMulFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_mul_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 28].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_mul_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreMulFpImm {
        src_off: M31(1),
        imm: M31(2),
        dst_off: M31(2),
    };

    let new_state = store_mul_fp_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 8].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_div_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let new_state = store_div_fp_fp(&mut memory, state, &instruction)?;

    let expected_div = M31::from(4) / M31::from(7);
    let expected_memory = Memory::from_iter([0, 4, 7, expected_div.0].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
#[should_panic(expected = "0 has no inverse")]
fn test_store_div_fp_fp_by_zero() {
    let mut memory = Memory::from_iter([0, 4, 0].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpFp {
        src0_off: M31(1),
        src1_off: M31(2),
        dst_off: M31(3),
    };

    let _ = store_div_fp_fp(&mut memory, state, &instruction);
}

#[test]
fn test_store_div_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpImm {
        src_off: M31(1),
        imm: M31(2),
        dst_off: M31(2),
    };

    let new_state = store_div_fp_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 2].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
#[should_panic(expected = "0 has no inverse")]
fn test_store_div_fp_imm_by_zero() {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = Instruction::StoreDivFpImm {
        src_off: M31(1),
        imm: M31(0),
        dst_off: M31(2),
    };

    let _ = store_div_fp_imm(&mut memory, state, &instruction);
}

#[test]
fn test_u32_store_add_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    // Set up 32-bit value in memory stored as two limbs
    // Value: 0x12345678 stored as [0x5678, 0x1234] at [fp+0] and [fp+1]
    let initial_fp = M31(10); // Use non-zero FP to avoid confusion with addresses
    memory.insert(initial_fp, M31(0x5678).into())?;
    memory.insert(initial_fp + M31(1), M31(0x1234).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0x9876),
        imm_lo: M31(0xABCD),
        dst_off: M31(2),
    };

    let new_state = u32_store_add_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0x12345678 + 0x9876ABCD = 0xAAAB0245
    // Low limb: 0x0245, High limb: 0xAAAB
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0x0245));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0xAAAB));

    // Check state advancement (instruction size is 5 M31s = 2 QM31s)
    assert_eq!(new_state.pc, M31(2));
    assert_eq!(new_state.fp, initial_fp);

    Ok(())
}

#[test]
fn test_u32_store_add_fp_imm_overflow() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);
    // Set up maximum 32-bit value: 0xFFFFFFFF
    memory.insert(initial_fp, M31(U32_LIMB_MASK).into())?;
    memory.insert(initial_fp + M31(1), M31(U32_LIMB_MASK).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0),
        imm_lo: M31(1),
        dst_off: M31(2),
    };

    let new_state = u32_store_add_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0xFFFFFFFF + 0x00000001 = 0x00000000 (wrapping)
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0));

    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

#[test]
fn test_u32_store_add_fp_imm_partial_overflow() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);
    // Set up value that will overflow low limb: 0x0000FFFF
    memory.insert(initial_fp, M31(U32_LIMB_MASK).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0),
        imm_lo: M31(1),
        dst_off: M31(2),
    };

    let new_state = u32_store_add_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0x0000FFFF + 0x00000001 = 0x00010000
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(1));

    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

#[test]
fn test_u32_store_add_fp_imm_invalid_immediate_limbs() {
    let mut memory = Memory::default();
    memory.insert(M31::zero(), M31(0).into()).unwrap();
    memory.insert(M31::one(), M31(0).into()).unwrap();

    let state = State::default();
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0x10000), // Exceeds 16-bit limit
        imm_lo: M31(0x10000), // Exceeds 16-bit limit
        dst_off: M31(2),
    };

    assert!(matches!(
        u32_store_add_fp_imm(&mut memory, state, &instruction),
        Err(InstructionExecutionError::InvalidOperand(_))
    ));
}

#[test]
fn test_u32_store_add_fp_imm_invalid_source_limbs() {
    let mut memory = Memory::default();
    memory.insert(M31::zero(), M31(0x10000).into()).unwrap(); // Exceeds 16-bit limit
    memory.insert(M31::one(), M31(0x10000).into()).unwrap();

    let state = State::default();
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0),
        imm_lo: M31(0),
        dst_off: M31(2),
    };

    assert!(matches!(
        u32_store_add_fp_imm(&mut memory, state, &instruction),
        Err(InstructionExecutionError::InvalidOperand(_))
    ));
}

#[test]
fn test_u32_store_add_fp_imm_max_valid_values() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    memory.insert(M31::zero(), M31(0xFFFF).into())?;
    memory.insert(M31::one(), M31(0xFFFF).into())?;

    let state = State::default();
    let instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0),
        imm_hi: M31(0xFFFF),
        imm_lo: M31(0xFFFF),
        dst_off: M31(2),
    };

    let new_state = u32_store_add_fp_imm(&mut memory, state, &instruction)?;

    // 0xFFFFFFFF + 0xFFFFFFFF = 0xFFFFFFFE (with wrapping)
    assert_eq!(memory.get_data(M31(2))?, M31(0xFFFE));
    assert_eq!(memory.get_data(M31(3))?, M31(0xFFFF));
    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

// ==================== U32 FP-FP Operations Tests ====================

#[test]
fn test_u32_store_add_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x12345678
    memory.insert(initial_fp, M31(0x5678).into())?;
    memory.insert(initial_fp + M31(1), M31(0x1234).into())?;

    // Set up second 32-bit value: 0x9876ABCD
    memory.insert(initial_fp + M31(2), M31(0xABCD).into())?;
    memory.insert(initial_fp + M31(3), M31(0x9876).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreAddFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_add_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0x12345678 + 0x9876ABCD = 0xAAAB0245
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0x0245));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0xAAAB));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_sub_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x9876ABCD
    memory.insert(initial_fp, M31(0xABCD).into())?;
    memory.insert(initial_fp + M31(1), M31(0x9876).into())?;

    // Set up second 32-bit value: 0x12345678
    memory.insert(initial_fp + M31(2), M31(0x5678).into())?;
    memory.insert(initial_fp + M31(3), M31(0x1234).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreSubFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_sub_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0x9876ABCD - 0x12345678 = 0x86425555
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0x5555));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0x8642));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_sub_fp_fp_underflow() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x00000001
    memory.insert(initial_fp, M31(0x0001).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    // Set up second 32-bit value: 0x00000002
    memory.insert(initial_fp + M31(2), M31(0x0002).into())?;
    memory.insert(initial_fp + M31(3), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreSubFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_sub_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0x00000001 - 0x00000002 = 0xFFFFFFFF (wrapping)
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0xFFFF));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0xFFFF));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_mul_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x00001234
    memory.insert(initial_fp, M31(0x1234).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    // Set up second 32-bit value: 0x00005678
    memory.insert(initial_fp + M31(2), M31(0x5678).into())?;
    memory.insert(initial_fp + M31(3), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreMulFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_mul_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0x00001234 * 0x00005678 = 0x06260060
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0x0060));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0x0626));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_mul_fp_fp_overflow() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0xFFFFFFFF
    memory.insert(initial_fp, M31(0xFFFF).into())?;
    memory.insert(initial_fp + M31(1), M31(0xFFFF).into())?;

    // Set up second 32-bit value: 0x00000002
    memory.insert(initial_fp + M31(2), M31(0x0002).into())?;
    memory.insert(initial_fp + M31(3), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreMulFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_mul_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0xFFFFFFFF * 0x00000002 = 0xFFFFFFFE (with wrapping)
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0xFFFE));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0xFFFF));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_div_fp_fp() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x00006260
    memory.insert(initial_fp, M31(0x6260).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    // Set up second 32-bit value: 0x00000004
    memory.insert(initial_fp + M31(2), M31(0x0004).into())?;
    memory.insert(initial_fp + M31(3), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreDivFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_div_fp_fp(&mut memory, state, &instruction)?;

    // Expected: 0x00006260 / 0x00000004 = 0x00001898
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0x1898));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0x0000));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

#[test]
fn test_u32_store_div_fp_fp_by_zero() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up first 32-bit value: 0x12345678
    memory.insert(initial_fp, M31(0x5678).into())?;
    memory.insert(initial_fp + M31(1), M31(0x1234).into())?;

    // Set up second 32-bit value: 0x00000000
    memory.insert(initial_fp + M31(2), M31(0x0000).into())?;
    memory.insert(initial_fp + M31(3), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreDivFpFp {
        src0_off: M31(0),
        src1_off: M31(2),
        dst_off: M31(4),
    };

    let new_state = u32_store_div_fp_fp(&mut memory, state, &instruction)?;

    // Expected: division by zero returns 0xFFFFFFFF
    assert_eq!(memory.get_data(initial_fp + M31(4))?, M31(0xFFFF));
    assert_eq!(memory.get_data(initial_fp + M31(5))?, M31(0xFFFF));
    assert_eq!(new_state.pc, M31(1));

    Ok(())
}

// ==================== U32 FP-IMM Operations Tests ====================

#[test]
fn test_u32_store_sub_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up 32-bit value: 0x9876ABCD
    memory.insert(initial_fp, M31(0xABCD).into())?;
    memory.insert(initial_fp + M31(1), M31(0x9876).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreSubFpImm {
        src_off: M31(0),
        imm_hi: M31(0x1234),
        imm_lo: M31(0x5678),
        dst_off: M31(2),
    };

    let new_state = u32_store_sub_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0x9876ABCD - 0x12345678 = 0x86425555
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0x5555));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0x8642));
    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

#[test]
fn test_u32_store_mul_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up 32-bit value: 0x00001234
    memory.insert(initial_fp, M31(0x1234).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreMulFpImm {
        src_off: M31(0),
        imm_hi: M31(0x0000),
        imm_lo: M31(0x5678),
        dst_off: M31(2),
    };

    let new_state = u32_store_mul_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0x00001234 * 0x00005678 = 0x06260060
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0x0060));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0x0626));
    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

#[test]
fn test_u32_store_div_fp_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up 32-bit value: 0x00006260
    memory.insert(initial_fp, M31(0x6260).into())?;
    memory.insert(initial_fp + M31(1), M31(0x0000).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreDivFpImm {
        src_off: M31(0),
        imm_hi: M31(0x0000),
        imm_lo: M31(0x0004),
        dst_off: M31(2),
    };

    let new_state = u32_store_div_fp_imm(&mut memory, state, &instruction)?;

    // Expected: 0x00006260 / 0x00000004 = 0x00001898
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0x1898));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0x0000));
    assert_eq!(new_state.pc, M31(2));

    Ok(())
}

#[test]
fn test_u32_store_div_fp_imm_by_zero() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let initial_fp = M31(10);

    // Set up 32-bit value: 0x12345678
    memory.insert(initial_fp, M31(0x5678).into())?;
    memory.insert(initial_fp + M31(1), M31(0x1234).into())?;

    let state = State {
        pc: M31(0),
        fp: initial_fp,
    };
    let instruction = Instruction::U32StoreDivFpImm {
        src_off: M31(0),
        imm_hi: M31(0x0000),
        imm_lo: M31(0x0000),
        dst_off: M31(2),
    };

    let new_state = u32_store_div_fp_imm(&mut memory, state, &instruction)?;

    // Expected: division by zero returns 0xFFFFFFFF
    assert_eq!(memory.get_data(initial_fp + M31(2))?, M31(0xFFFF));
    assert_eq!(memory.get_data(initial_fp + M31(3))?, M31(0xFFFF));
    assert_eq!(new_state.pc, M31(2));

    Ok(())
}
