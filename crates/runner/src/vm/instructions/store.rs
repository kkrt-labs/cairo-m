//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.

use cairo_m_common::{Instruction, State};
use num_traits::One;
use stwo_prover::core::fields::m31::M31;

use super::InstructionExecutionError;
use crate::extract_as;
use crate::memory::Memory;
use crate::vm::state::VmState;

/// Number of bits in a U32 limb (16 bits per limb for 32-bit values)
const U32_LIMB_BITS: u32 = 16;
/// Mask for a U32 limb (0xFFFF)
const U32_LIMB_MASK: u32 = (1 << U32_LIMB_BITS) - 1;

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src0_off] + [fp + src1_off]
/// ```
pub fn store_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src0_off, src1_off, dst_off) =
        extract_as!(instruction, StoreAddFpFp, (src0_off, src1_off, dst_off));
    let value = memory.get_data(state.fp + src0_off)? + memory.get_data(state.fp + src1_off)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src_off] + imm
/// ```
pub fn store_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm, dst_off) = extract_as!(instruction, StoreAddFpImm, (src_off, imm, dst_off));
    let value = memory.get_data(state.fp + src_off)? + imm;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src0_off] - [fp + src1_off]
/// ```
pub fn store_sub_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src0_off, src1_off, dst_off) =
        extract_as!(instruction, StoreSubFpFp, (src0_off, src1_off, dst_off));
    let value = memory.get_data(state.fp + src0_off)? - memory.get_data(state.fp + src1_off)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src_off] - imm
/// ```
pub fn store_sub_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm, dst_off) = extract_as!(instruction, StoreSubFpImm, (src_off, imm, dst_off));
    let value = memory.get_data(state.fp + src_off)? - imm;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [[fp + base_off] + offset]
/// ```
pub fn store_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (base_off, offset, dst_off) =
        extract_as!(instruction, StoreDoubleDerefFp, (base_off, offset, dst_off));
    let deref_value = memory.get_data(state.fp + base_off)?;
    let value = memory.get_data(deref_value + offset)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = imm
/// ```
pub fn store_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (imm, dst_off) = extract_as!(instruction, StoreImm, (imm, dst_off));
    memory.insert(state.fp + dst_off, imm.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src0_off] * [fp + src1_off]
/// ```
pub fn store_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src0_off, src1_off, dst_off) =
        extract_as!(instruction, StoreMulFpFp, (src0_off, src1_off, dst_off));
    let value = memory.get_data(state.fp + src0_off)? * memory.get_data(state.fp + src1_off)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src_off] * imm
/// ```
pub fn store_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm, dst_off) = extract_as!(instruction, StoreMulFpImm, (src_off, imm, dst_off));
    let value = memory.get_data(state.fp + src_off)? * imm;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src0_off] / [fp + src1_off]
/// ```
pub fn store_div_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src0_off, src1_off, dst_off) =
        extract_as!(instruction, StoreDivFpFp, (src0_off, src1_off, dst_off));
    let value = memory.get_data(state.fp + src0_off)? / memory.get_data(state.fp + src1_off)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src_off] / imm
/// ```
pub fn store_div_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm, dst_off) = extract_as!(instruction, StoreDivFpImm, (src_off, imm, dst_off));
    let value = memory.get_data(state.fp + src_off)? / imm;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// U32 store add fp imm instruction.
///
/// u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) + u32(imm_lo, imm_hi)
/// This instruction supports 32-bit values stored as two 16-bit M31 limbs
pub fn u32_store_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm_hi, imm_lo, dst_off) = extract_as!(
        instruction,
        U32StoreAddFpImm,
        (src_off, imm_hi, imm_lo, dst_off)
    );

    // Read 32-bit value from memory as two limbs
    let src_limb_0 = memory.get_data(state.fp + src_off)?;
    let src_limb_1 = memory.get_data(state.fp + src_off + M31::from(1))?;

    // Validate that source limbs are within 16-bit range
    if src_limb_0.0 > U32_LIMB_MASK || src_limb_1.0 > U32_LIMB_MASK {
        return Err(InstructionExecutionError::InvalidOperand(format!(
            "U32 source limbs exceed 16-bit range: limb_0={}, limb_1={}",
            src_limb_0.0, src_limb_1.0
        )));
    }
    let src_value = (src_limb_1.0 << U32_LIMB_BITS) | src_limb_0.0;

    // Construct 32-bit immediate from two limbs
    // Validate that immediate limbs are within 16-bit range
    if imm_lo.0 > U32_LIMB_MASK || imm_hi.0 > U32_LIMB_MASK {
        return Err(InstructionExecutionError::InvalidOperand(format!(
            "U32 immediate limbs exceed 16-bit range: imm_lo={}, imm_hi={}",
            imm_lo.0, imm_hi.0
        )));
    }
    let imm_value = (imm_hi.0 << U32_LIMB_BITS) | imm_lo.0;

    // Perform 32-bit addition with wrapping
    let result = src_value.wrapping_add(imm_value);

    // Store result as two 16-bit limbs
    let res_limb_0 = M31::from(result & U32_LIMB_MASK);
    let res_limb_1 = M31::from((result >> U32_LIMB_BITS) & U32_LIMB_MASK);

    memory.insert(state.fp + dst_off, res_limb_0.into())?;
    memory.insert(state.fp + dst_off + M31::one(), res_limb_1.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// TODO: Implement U32 store add fp fp instruction
pub fn u32_store_add_fp_fp(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_add_fp_fp not implemented")
}

/// TODO: Implement U32 store sub fp fp instruction
pub fn u32_store_sub_fp_fp(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_sub_fp_fp not implemented")
}

/// TODO: Implement U32 store mul fp fp instruction
pub fn u32_store_mul_fp_fp(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_mul_fp_fp not implemented")
}

/// TODO: Implement U32 store div fp fp instruction
pub fn u32_store_div_fp_fp(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_div_fp_fp not implemented")
}

/// TODO: Implement U32 store sub fp imm instruction
pub fn u32_store_sub_fp_imm(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_sub_fp_imm not implemented")
}

/// TODO: Implement U32 store mul fp imm instruction
pub fn u32_store_mul_fp_imm(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_mul_fp_imm not implemented")
}

/// TODO: Implement U32 store div fp imm instruction
pub fn u32_store_div_fp_imm(
    _memory: &mut Memory,
    _state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    todo!("u32_store_div_fp_imm not implemented")
}

#[cfg(test)]
#[path = "./store_tests.rs"]
mod store_tests;
