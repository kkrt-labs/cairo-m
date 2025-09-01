//! JUMP instructions for the Cairo M VM.

use cairo_m_common::{extract_as, Instruction, State};

use crate::memory::Memory;
use crate::vm::state::VmState;

use super::InstructionExecutionError;

/// CASM equivalent:
/// ```casm
/// jmp abs target
/// ```
pub fn jmp_abs_imm(
    _: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let target = extract_as!(instruction, JmpAbsImm, target);

    Ok(state.jump_abs(target))
}

/// CASM equivalent:
/// ```casm
/// jmp rel offset
/// ```
pub fn jmp_rel_imm(
    _: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let offset = extract_as!(instruction, JmpRelImm, offset);

    Ok(state.jump_rel(offset))
}

#[cfg(test)]
#[path = "./jump_tests.rs"]
mod jump_tests;
