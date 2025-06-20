use cairo_m_common::Opcode;
use num_traits::{One, Zero};
use stwo_prover::core::fields::m31::M31;

use super::*;
use crate::vm::test_utils::*;

#[test]
fn test_store_add_fp_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreAddFpFp, 1, 2, 3);

    let new_state = store_add_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 11].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);

    assert_eq!(new_state.fp, M31::zero());
    assert_eq!(new_state.pc, M31::one());

    Ok(())
}

#[test]
fn test_store_add_fp_imm() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let expected_memory = Memory::from_iter([0, 4, 7, 6].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreAddFpImm, 1, 2, 3);

    let new_state = store_add_fp_imm(&mut memory, state, &instruction)?;

    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_sub_fp_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 7, 4].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreSubFpFp, 1, 2, 3);

    let new_state = store_sub_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 7, 4, 3].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_sub_fp_imm() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreSubFpImm, 1, 2, 3);

    let new_state = store_sub_fp_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 2].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_deref_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreDerefFp, 1, 0, 2);

    let new_state = store_deref_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 4].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_double_deref_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 1, 7, 9].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreDoubleDerefFp, 1, 2, 2);

    let new_state = store_double_deref_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 1, 9, 9].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_imm() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = store_imm!(1, 2);

    let new_state = store_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 1].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);

    assert_eq!(new_state.fp, M31::zero());
    assert_eq!(new_state.pc, M31::one());

    Ok(())
}

#[test]
fn test_store_mul_fp_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreMulFpFp, 1, 2, 3);

    let new_state = store_mul_fp_fp(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 7, 28].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_mul_fp_imm() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreMulFpImm, 1, 2, 2);

    let new_state = store_mul_fp_imm(&mut memory, state, &instruction)?;

    let expected_memory = Memory::from_iter([0, 4, 8].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_vm_state!(new_state, 1, 0);

    Ok(())
}

#[test]
fn test_store_div_fp_fp() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreDivFpFp, 1, 2, 3);

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
    let instruction = instr!(Opcode::StoreDivFpFp, 1, 2, 3);

    let _ = store_div_fp_fp(&mut memory, state, &instruction);
}

#[test]
fn test_store_div_fp_imm() -> Result<(), MemoryError> {
    let mut memory = Memory::from_iter([0, 4].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::StoreDivFpImm, 1, 2, 2);

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
    let instruction = instr!(Opcode::StoreDivFpImm, 1, 0, 2);

    let _ = store_div_fp_imm(&mut memory, state, &instruction);
}
