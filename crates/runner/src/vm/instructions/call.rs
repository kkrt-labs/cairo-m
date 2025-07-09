//! CALL and RET instructions for the Cairo M VM.
//!
//! CALL instructions handle functions calls, creating new frames.
//! There are relative and absolute function calls.
//!
//! Call-related memory layout convention:
//! ```text
//! [lower addresses]
//! - Function arguments
//! - Return values
//! - Old FP
//! - Return PC
//! [higher addresses]
//! ```
//!
//! The first argument, `off0` is the offset between the current frame pointer and the next frame pointer minus 2.
//! In other words, `next_fp = fp + off0 + 2`.
//! The second argument, `off1` is the destination offset to compute the return address.
//!
//! The function arguments are assumed to be already stored in memory.
//! Considering a function call with N arguments and M return values,
//! the arguments are stored in memory at [fp + off0 - N - M, ..., fp + off0 - M - 1],
//! and the return values have dedicated cells at [fp + off0 - M, fp + off0 - 1].
//!
//! The function call is performed by:
//! - Storing FP in memory at fp + off0.
//! - Storing the return address in memory at fp + off0 + 1.
//! - Updating FP for the new frame: fp + off0 + 2.
//! - Updating PC to the function address based on off1.
//!
//! RET instructions returns control to the caller:
//! - Restore FP from memory, stored at fp - 2.
//! - Update PC to the return address, stored at fp - 1.

use cairo_m_common::{Instruction, State};
use num_traits::One;
use stwo_prover::core::fields::m31::M31;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::VmState;

/// Call instruction
/// PC update: `next_pc = imm`
///
/// CASM equivalent:
/// `call abs imm`
pub fn call_abs_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    memory.insert(state.fp + off0, state.fp.into())?;
    memory.insert(state.fp + off0 + M31::one(), (state.pc + M31::one()).into())?;

    Ok(state.call_abs(imm, off0 + M31(2)))
}

/// Return instruction
/// PC update: `next_pc = [fp - 1]`
/// FP update: `fp = [fp - 2]`
///
/// CASM equivalent:
/// `ret`
pub fn ret(memory: &mut Memory, state: State, _: &Instruction) -> Result<State, MemoryError> {
    let pc = memory.get_data(state.fp - M31::one())?;
    let fp = memory.get_data(state.fp - M31(2))?;

    Ok(state.ret(pc, fp))
}

#[cfg(test)]
#[path = "./call_tests.rs"]
mod call_tests;
