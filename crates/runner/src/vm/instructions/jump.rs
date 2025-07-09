//! JUMP instructions for the Cairo M VM.

use cairo_m_common::{Instruction, State};

use crate::memory::{Memory, MemoryError};
use crate::vm::state::VmState;

/// CASM equivalent:
/// ```casm
/// jmp abs imm
/// ```
pub fn jmp_abs_imm(
    _: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, _] = instruction.operands;

    Ok(state.jump_abs(imm))
}

/// CASM equivalent:
/// ```casm
/// jmp rel imm
/// ```
pub fn jmp_rel_imm(
    _: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, _] = instruction.operands;

    Ok(state.jump_rel(imm))
}

#[cfg(test)]
#[path = "./jump_tests.rs"]
mod jump_tests;
