//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.

use cairo_m_common::{Instruction, State};
use num_traits::One;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use super::InstructionExecutionError;
use crate::extract_as;
use crate::memory::{Memory, MemoryError, U32_LIMB_BITS, U32_LIMB_MASK};
use crate::vm::state::VmState;

/// Execute a binary op between two U32 operands `[fp + src0_off]` and `[fp + src1_off]`.
fn exec_u32_bin_op_fp_fp<F>(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
    src0_off: M31,
    src1_off: M31,
    dst_off: M31,
    op: F,
) -> Result<State, InstructionExecutionError>
where
    F: Fn(u32, u32) -> u32,
{
    let lhs = memory.get_u32(state.fp + src0_off)?;
    let rhs = memory.get_u32(state.fp + src1_off)?;

    let res = op(lhs, rhs);
    memory.insert_u32(state.fp + dst_off, res)?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// Execute a binary op between a U32 operand `[fp + src_off]` and a 32-bit immediate.
#[allow(clippy::too_many_arguments)]
fn exec_u32_bin_op_fp_imm<F>(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
    src_off: M31,
    imm_lo: M31,
    imm_hi: M31,
    dst_off: M31,
    op: F,
) -> Result<State, InstructionExecutionError>
where
    F: Fn(u32, u32) -> u32,
{
    if imm_hi.0 > U32_LIMB_MASK || imm_lo.0 > U32_LIMB_MASK {
        return Err(InstructionExecutionError::Memory(
            MemoryError::U32LimbOutOfRange {
                limb_lo: imm_lo.0,
                limb_hi: imm_hi.0,
            },
        ));
    }

    let imm_value: u32 = (imm_hi.0 << U32_LIMB_BITS) | imm_lo.0;
    let src_value = memory.get_u32(state.fp + src_off)?;

    let res = op(src_value, imm_value);
    memory.insert_u32(state.fp + dst_off, res)?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// Generates U32 `*_fp_fp` store operations.
macro_rules! impl_u32_store_bin_op_fp_fp {
    ($func_name:ident, $variant:ident, $body:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src0_off, src1_off, dst_off) =
                extract_as!(instruction, $variant, (src0_off, src1_off, dst_off));
            exec_u32_bin_op_fp_fp(
                memory,
                state,
                instruction,
                src0_off,
                src1_off,
                dst_off,
                $body,
            )
        }
    };
}

/// Generates U32 `*_fp_imm` store operations.
macro_rules! impl_u32_store_bin_op_fp_imm {
    ($func_name:ident, $variant:ident, $body:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src_off, imm_lo, imm_hi, dst_off) =
                extract_as!(instruction, $variant, (src_off, imm_lo, imm_hi, dst_off));
            exec_u32_bin_op_fp_imm(
                memory,
                state,
                instruction,
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
                $body,
            )
        }
    };
}

/// Generates the `*_fp_fp` binary-arithmetic store operations.
macro_rules! impl_store_bin_op_fp_fp {
    ($func_name:ident, $variant:ident, $op:tt) => {
        #[allow(clippy::suspicious_arithmetic_impl)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src0_off, src1_off, dst_off) =
                extract_as!(instruction, $variant, (src0_off, src1_off, dst_off));

            let value = memory.get_data(state.fp + src0_off)?
                $op memory.get_data(state.fp + src1_off)?;

            memory.insert(state.fp + dst_off, value.into())?;
            Ok(state.advance_by(instruction.size_in_qm31s()))
        }
    };
}

/// Generates the `*_fp_imm` binary-arithmetic store operations.
macro_rules! impl_store_bin_op_fp_imm {
    ($func_name:ident, $variant:ident, $op:tt) => {
        #[allow(clippy::suspicious_arithmetic_impl)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src_off, imm, dst_off) =
                extract_as!(instruction, $variant, (src_off, imm, dst_off));

            let value = memory.get_data(state.fp + src_off)? $op imm;

            memory.insert(state.fp + dst_off, value.into())?;
            Ok(state.advance_by(instruction.size_in_qm31s()))
        }
    };
}

// -------------------------------------------------------------------------------------------------
// Automatically-generated STORE FP-FP operations (scalar)
// -------------------------------------------------------------------------------------------------

impl_store_bin_op_fp_fp!(store_add_fp_fp, StoreAddFpFp, +);
impl_store_bin_op_fp_fp!(store_sub_fp_fp, StoreSubFpFp, -);
impl_store_bin_op_fp_fp!(store_mul_fp_fp, StoreMulFpFp, *);
impl_store_bin_op_fp_fp!(store_div_fp_fp, StoreDivFpFp, /);

// -------------------------------------------------------------------------------------------------
// Automatically-generated STORE FP-IMM operations (scalar)
// -------------------------------------------------------------------------------------------------

impl_store_bin_op_fp_imm!(store_add_fp_imm, StoreAddFpImm, +);
impl_store_bin_op_fp_imm!(store_sub_fp_imm, StoreSubFpImm, -);
impl_store_bin_op_fp_imm!(store_mul_fp_imm, StoreMulFpImm, *);
impl_store_bin_op_fp_imm!(store_div_fp_imm, StoreDivFpImm, /);

// -------------------------------------------------------------------------------------------------
// Singular / less-regular STORE instructions (scalar)
// -------------------------------------------------------------------------------------------------

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

// -------------------------------------------------------------------------------------------------
// U32-specific instructions
// -------------------------------------------------------------------------------------------------

// -- FP-FP variants ----------------------------------------------------------
impl_u32_store_bin_op_fp_fp!(u32_store_add_fp_fp, U32StoreAddFpFp, |a, b| a
    .wrapping_add(b));
impl_u32_store_bin_op_fp_fp!(u32_store_sub_fp_fp, U32StoreSubFpFp, |a, b| a
    .wrapping_sub(b));
impl_u32_store_bin_op_fp_fp!(u32_store_mul_fp_fp, U32StoreMulFpFp, |a, b| a
    .wrapping_mul(b));
impl_u32_store_bin_op_fp_fp!(u32_store_div_fp_fp, U32StoreDivFpFp, |a, b| a / b);

// -- FP-IMM variants ---------------------------------------------------------
impl_u32_store_bin_op_fp_imm!(u32_store_add_fp_imm, U32StoreAddFpImm, |a, b| a
    .wrapping_add(b));
impl_u32_store_bin_op_fp_imm!(u32_store_sub_fp_imm, U32StoreSubFpImm, |a, b| a
    .wrapping_sub(b));
impl_u32_store_bin_op_fp_imm!(u32_store_mul_fp_imm, U32StoreMulFpImm, |a, b| a
    .wrapping_mul(b));
impl_u32_store_bin_op_fp_imm!(u32_store_div_fp_imm, U32StoreDivFpImm, |a, b| a / b);

/// CASM equivalent:
/// ```casm
/// u32([fp + dst_off], [fp + dst_off + 1]) = imm
/// ```
pub fn u32_store_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (imm_lo, imm_hi, dst_off) =
        extract_as!(instruction, U32StoreImm, (imm_lo, imm_hi, dst_off));
    if imm_lo.0 > U32_LIMB_MASK || imm_hi.0 > U32_LIMB_MASK {
        return Err(InstructionExecutionError::Memory(
            MemoryError::U32LimbOutOfRange {
                limb_lo: imm_lo.0,
                limb_hi: imm_hi.0,
            },
        ));
    }
    memory.insert(state.fp + dst_off, imm_lo.into())?;
    memory.insert(state.fp + dst_off + M31::one(), imm_hi.into())?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

// -------------------------------------------------------------------------------------------------
// U32 Comparison operations
// -------------------------------------------------------------------------------------------------

/// Execute a comparison op between two U32 operands `[fp + src0_off]` and `[fp + src1_off]`.
fn exec_u32_cmp_op_fp_fp<F>(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
    src0_off: M31,
    src1_off: M31,
    dst_off: M31,
    op: F,
) -> Result<State, InstructionExecutionError>
where
    F: Fn(u32, u32) -> bool,
{
    let lhs = memory.get_u32(state.fp + src0_off)?;
    let rhs = memory.get_u32(state.fp + src1_off)?;

    let res = if op(lhs, rhs) {
        QM31::from(M31::one())
    } else {
        QM31::from(M31::from(0))
    };

    memory.insert(state.fp + dst_off, res)?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// Execute a comparison op between a U32 operand `[fp + src_off]` and a 32-bit immediate.
#[allow(clippy::too_many_arguments)]
fn exec_u32_cmp_op_fp_imm<F>(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
    src_off: M31,
    imm_lo: M31,
    imm_hi: M31,
    dst_off: M31,
    op: F,
) -> Result<State, InstructionExecutionError>
where
    F: Fn(u32, u32) -> bool,
{
    if imm_hi.0 > U32_LIMB_MASK || imm_lo.0 > U32_LIMB_MASK {
        return Err(InstructionExecutionError::Memory(
            MemoryError::U32LimbOutOfRange {
                limb_lo: imm_lo.0,
                limb_hi: imm_hi.0,
            },
        ));
    }

    let imm_value: u32 = (imm_hi.0 << U32_LIMB_BITS) | imm_lo.0;
    let src_value = memory.get_u32(state.fp + src_off)?;

    let res = if op(src_value, imm_value) {
        QM31::from(M31::one())
    } else {
        QM31::from(M31::from(0))
    };

    memory.insert(state.fp + dst_off, res)?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// Generates U32 comparison `*_fp_imm` operations.
macro_rules! impl_u32_cmp_op_fp_imm {
    ($func_name:ident, $variant:ident, $body:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src_off, imm_lo, imm_hi, dst_off) =
                extract_as!(instruction, $variant, (src_off, imm_lo, imm_hi, dst_off));
            exec_u32_cmp_op_fp_imm(
                memory,
                state,
                instruction,
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
                $body,
            )
        }
    };
}

/// Generates U32 comparison `*_fp_fp` operations.
macro_rules! impl_u32_cmp_op_fp_fp {
    ($func_name:ident, $variant:ident, $body:expr) => {
        #[allow(clippy::redundant_closure_call)]
        pub fn $func_name(
            memory: &mut Memory,
            state: State,
            instruction: &Instruction,
        ) -> Result<State, InstructionExecutionError> {
            let (src0_off, src1_off, dst_off) =
                extract_as!(instruction, $variant, (src0_off, src1_off, dst_off));
            exec_u32_cmp_op_fp_fp(
                memory,
                state,
                instruction,
                src0_off,
                src1_off,
                dst_off,
                $body,
            )
        }
    };
}

// -- U32 Comparison FP-FP variants ------------------------------------------
impl_u32_cmp_op_fp_fp!(u32_store_eq_fp_fp, U32StoreEqFpFp, |a, b| a == b);
impl_u32_cmp_op_fp_fp!(u32_store_neq_fp_fp, U32StoreNeqFpFp, |a, b| a != b);
impl_u32_cmp_op_fp_fp!(u32_store_gt_fp_fp, U32StoreGtFpFp, |a, b| a > b);
impl_u32_cmp_op_fp_fp!(u32_store_ge_fp_fp, U32StoreGeFpFp, |a, b| a >= b);
impl_u32_cmp_op_fp_fp!(u32_store_lt_fp_fp, U32StoreLtFpFp, |a, b| a < b);
impl_u32_cmp_op_fp_fp!(u32_store_le_fp_fp, U32StoreLeFpFp, |a, b| a <= b);

// -- U32 Comparison FP-IMM variants ------------------------------------------
impl_u32_cmp_op_fp_imm!(u32_store_eq_fp_imm, U32StoreEqFpImm, |a, b| a == b);
impl_u32_cmp_op_fp_imm!(u32_store_neq_fp_imm, U32StoreNeqFpImm, |a, b| a != b);
impl_u32_cmp_op_fp_imm!(u32_store_gt_fp_imm, U32StoreGtFpImm, |a, b| a > b);
impl_u32_cmp_op_fp_imm!(u32_store_ge_fp_imm, U32StoreGeFpImm, |a, b| a >= b);
impl_u32_cmp_op_fp_imm!(u32_store_lt_fp_imm, U32StoreLtFpImm, |a, b| a < b);
impl_u32_cmp_op_fp_imm!(u32_store_le_fp_imm, U32StoreLeFpImm, |a, b| a <= b);

#[cfg(test)]
#[path = "./store_tests.rs"]
mod store_tests;
