//! JNZ instructions for the Cairo M VM.
//!
//! JNZ are conditional relative jumps.
//! The condition offset is the first instruction argument.
//! The destination offset when the condition is true is the second instruction argument.

use cairo_m_common::{Instruction, State};
use num_traits::Zero;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::VmState;

/// CASM equivalent:
/// ```casm
/// jmp rel imm if [fp + off0] != 0
/// ```
pub fn jnz_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    let condition = memory.get_data(state.fp + off0)?;
    let new_state = if !condition.is_zero() {
        state.jump_rel(imm)
    } else {
        state.advance()
    };

    Ok(new_state)
}

#[cfg(test)]
#[path = "./jnz_tests.rs"]
mod jnz_tests;
