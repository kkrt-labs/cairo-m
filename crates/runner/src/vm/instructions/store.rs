//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.

use cairo_m_common::{Instruction, State};
use num_traits::{One, Zero};
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
// StoreSubFpImm removed - compiled as StoreAddFpImm with negated immediate
impl_store_bin_op_fp_imm!(store_mul_fp_imm, StoreMulFpImm, *);
// StoreDivFpImm removed - compiled as StoreMulFpImm with inverse immediate

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [fp + src_off] < imm
/// ```
///
/// Store the result of a less-than comparison as a felt (0 or 1)
pub fn store_lower_than_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm, dst_off) =
        extract_as!(instruction, StoreLowerThanFpImm, (src_off, imm, dst_off));

    let src_value = memory.get_data(state.fp + src_off)?;
    let value = M31::from((src_value < imm) as u32);

    memory.insert(state.fp + dst_off, value.into())?;
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

// -------------------------------------------------------------------------------------------------
// Singular / less-regular STORE instructions (scalar)
// -------------------------------------------------------------------------------------------------

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [[fp + base_off] + imm]
/// ```
pub fn store_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (base_off, imm, dst_off) =
        extract_as!(instruction, StoreDoubleDerefFp, (base_off, imm, dst_off));
    let deref_value = memory.get_data(state.fp + base_off)?;
    let value = memory.get_data(deref_value + imm)?;
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [fp + dst_off] = [[fp + base_off] + [fp + offset_off]]
/// ```
pub fn store_double_deref_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (base_off, offset_off, dst_off) = extract_as!(
        instruction,
        StoreDoubleDerefFpFp,
        (base_off, offset_off, dst_off)
    );

    // Get the offset value from memory at [fp + offset_off]
    let offset_value = memory.get_data(state.fp + offset_off)?;

    // Get the base pointer from memory at [fp + base_off]
    let base_address = memory.get_data(state.fp + base_off)?;

    // Calculate the final address: base_address + offset_value
    let final_address = base_address + offset_value;

    // Read the value from the calculated address
    let value = memory.get_data(final_address)?;

    // Store the value at [fp + dst_off]
    memory.insert(state.fp + dst_off, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

// -------------------------------------------------------------------------------------------------
// Reverse Double Deref operations - Store TO computed addresses
// -------------------------------------------------------------------------------------------------

/// CASM equivalent:
/// ```casm
/// [[fp + base_off] + imm] = [fp + src_off]
/// ```
///
/// Stores the value at [fp + src_off] TO the address computed as [[fp + base_off] + imm].
/// This is the reverse of StoreDoubleDerefFp which reads FROM a computed address.
pub fn store_to_double_deref_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (base_off, imm, src_off) = extract_as!(
        instruction,
        StoreToDoubleDerefFpImm,
        (base_off, imm, src_off)
    );

    let value = memory.get_data(state.fp + src_off)?;
    let base_address = memory.get_data(state.fp + base_off)?;
    let target_address = base_address + imm;
    memory.insert(target_address, value.into())?;

    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// CASM equivalent:
/// ```casm
/// [[fp + base_off] + [fp + offset_off]] = [fp + src_off]
/// ```
///
/// Stores the value at [fp + src_off] TO the address computed as [[fp + base_off] + [fp + offset_off]].
/// This is the reverse of StoreDoubleDerefFpFp which reads FROM a computed address.
pub fn store_to_double_deref_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (base_off, offset_off, src_off) = extract_as!(
        instruction,
        StoreToDoubleDerefFpFp,
        (base_off, offset_off, src_off)
    );

    let value = memory.get_data(state.fp + src_off)?;
    let offset_value = memory.get_data(state.fp + offset_off)?;
    let base_addr = memory.get_data(state.fp + base_off)?;

    let target_address = base_addr + offset_value;
    memory.insert(target_address, value.into())?;

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
/// [fp + dst_off] = fp + imm
/// ```
pub fn store_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (imm, dst_off) = extract_as!(instruction, StoreFpImm, (imm, dst_off));
    let value = state.fp + imm;
    memory.insert(state.fp + dst_off, value.into())?;

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
// U32StoreSubFpImm removed - compiled as U32StoreAddFpImm with two's complement immediate
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
        QM31::from(M31::zero())
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
        QM31::from(M31::zero())
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

// -- U32 Comparison FP-FP variants (minimal set - others derived) -----------
impl_u32_cmp_op_fp_fp!(u32_store_eq_fp_fp, U32StoreEqFpFp, |a, b| a == b);
// U32StoreNeqFpFp removed - compiled as 1 - U32StoreEqFpFp
// U32StoreGtFpFp removed - compiled as U32StoreLtFpFp with swapped operands
// U32StoreGeFpFp removed - compiled as 1 - U32StoreLtFpFp
impl_u32_cmp_op_fp_fp!(u32_store_lt_fp_fp, U32StoreLtFpFp, |a, b| a < b);
// U32StoreLeFpFp removed - compiled as U32StoreGeFpFp with swapped operands

// -- U32 Comparison FP-IMM variants (minimal set - others derived) ----------
impl_u32_cmp_op_fp_imm!(u32_store_eq_fp_imm, U32StoreEqFpImm, |a, b| a == b);
// U32StoreNeqFpImm removed - compiled as 1 - U32StoreEqFpImm
// U32StoreGtFpImm removed - compiled as 1 - U32StoreLeFpImm
// U32StoreGeFpImm removed - compiled as 1 - U32StoreLtFpImm
impl_u32_cmp_op_fp_imm!(u32_store_lt_fp_imm, U32StoreLtFpImm, |a, b| a < b);
// U32StoreLeFpImm removed - compiled as U32StoreLtFpImm with bias (x <= c → x < c+1)

// -------------------------------------------------------------------------------------------------
// U32 Bitwise operations
// -------------------------------------------------------------------------------------------------

// -- U32 Bitwise FP-FP variants ------------------------------------------
impl_u32_store_bin_op_fp_fp!(u32_store_and_fp_fp, U32StoreAndFpFp, |a, b| a & b);
impl_u32_store_bin_op_fp_fp!(u32_store_or_fp_fp, U32StoreOrFpFp, |a, b| a | b);
impl_u32_store_bin_op_fp_fp!(u32_store_xor_fp_fp, U32StoreXorFpFp, |a, b| a ^ b);

// -- U32 Bitwise FP-IMM variants ------------------------------------------
impl_u32_store_bin_op_fp_imm!(u32_store_and_fp_imm, U32StoreAndFpImm, |a, b| a & b);
impl_u32_store_bin_op_fp_imm!(u32_store_or_fp_imm, U32StoreOrFpImm, |a, b| a | b);
impl_u32_store_bin_op_fp_imm!(u32_store_xor_fp_imm, U32StoreXorFpImm, |a, b| a ^ b);

#[cfg(test)]
#[path = "./store_tests.rs"]
mod store_tests;
