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
/// jmp rel [fp + off1] if [fp + off0] != 0
/// ```
pub fn jnz_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let condition = memory.get_data(state.fp + off0)?;
    // Target is out of if statement to have a constant number of memory accesses for each opcode(easier for prover)
    let target = memory.get_data(state.fp + off1)?;
    let new_state = if !condition.is_zero() {
        state.jump_rel(target)
    } else {
        state.advance()
    };

    Ok(new_state)
}

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
