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
//! The first argument, `frame_off` is the offset between the current frame pointer and the next frame pointer minus 2.
//! In other words, `next_fp = fp + frame_off + 2`.
//! The second argument, `target` is the absolute address of the function to call.
//!
//! The function arguments are assumed to be already stored in memory.
//! Considering a function call with N arguments and M return values,
//! the arguments are stored in memory at [fp + frame_off - N - M, ..., fp + frame_off - M - 1],
//! and the return values have dedicated cells at [fp + frame_off - M, fp + frame_off - 1].
//!
//! The function call is performed by:
//! - Storing FP in memory at fp + frame_off.
//! - Storing the return address in memory at fp + frame_off + 1.
//! - Updating FP for the new frame: fp + frame_off + 2.
//! - Updating PC to the target address.
//!
//! RET instructions returns control to the caller:
//! - Restore FP from memory, stored at fp - 2.
//! - Update PC to the return address, stored at fp - 1.

use cairo_m_common::{Instruction, State};
use num_traits::One;
use stwo_prover::core::fields::m31::M31;

use super::InstructionExecutionError;
use crate::extract_as;
use crate::memory::Memory;
use crate::vm::state::VmState;

/// Call instruction
/// PC update: `next_pc = target`
///
/// CASM equivalent:
/// `call abs target`
pub fn call_abs_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (frame_off, target) = extract_as!(instruction, CallAbsImm, (frame_off, target));
    memory.insert(state.fp + frame_off, state.fp)?;
    memory.insert(
        state.fp + frame_off + M31::one(),
        state.pc + M31::from(instruction.size_in_m31s()),
    )?;

    Ok(state.call_abs(target, frame_off + M31(2)))
}

/// Return instruction
/// PC update: `next_pc = [fp - 1]`
/// FP update: `fp = [fp - 2]`
///
/// CASM equivalent:
/// `ret`
pub fn ret(
    memory: &mut Memory,
    state: State,
    _instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let pc = memory.get_felt(state.fp - M31::one())?;
    let fp = memory.get_felt(state.fp - M31(2))?;

    Ok(state.ret(pc, fp))
}

#[cfg(test)]
#[path = "./call_tests.rs"]
mod call_tests;
