//! JNZ instructions for the Cairo M VM.
//!
//! JNZ are conditional relative jumps.
//! The condition offset is the first instruction argument.
//! The destination offset when the condition is true is the second instruction argument.

use cairo_m_common::{Instruction, State};
use num_traits::Zero;

use super::InstructionExecutionError;
use crate::memory::Memory;
use crate::vm::state::VmState;
use cairo_m_common::extract_as;

/// CASM equivalent:
/// ```casm
/// jmp rel offset if [fp + cond_off] != 0
/// ```
pub fn jnz_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (cond_off, offset) = extract_as!(instruction, JnzFpImm, (cond_off, offset));
    let condition = memory.get_data(state.fp + cond_off)?;
    let new_state = if !condition.is_zero() {
        state.jump_rel(offset)
    } else {
        state.advance_by(instruction.size_in_qm31s())
    };

    Ok(new_state)
}

#[cfg(test)]
#[path = "./jnz_tests.rs"]
mod jnz_tests;
