//! JUMP instructions for the Cairo M VM.

use cairo_m_common::Instruction;

use crate::memory::{Memory, MemoryError};
use crate::vm::State;

/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] + [fp + off1]
/// ```
pub fn jmp_abs_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;

    Ok(state.jump_abs(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] + imm
/// ```
pub fn jmp_abs_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? + imm;

    Ok(state.jump_abs(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0]
/// ```
pub fn jmp_abs_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, _, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)?;

    Ok(state.jump_abs(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp abs [[fp + off0] + off1]
/// ```
pub fn jmp_abs_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let deref_value = memory.get_data(state.fp + off0)?;
    let offset = memory.get_data(deref_value + off1)?;

    Ok(state.jump_abs(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp abs imm
/// ```
pub const fn jmp_abs_imm(
    _: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, _] = instruction.operands;

    Ok(state.jump_abs(imm))
}

/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] * [fp + off1]
/// ```
pub fn jmp_abs_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;

    Ok(state.jump_abs(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] * imm
/// ```
pub fn jmp_abs_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? * imm;

    Ok(state.jump_abs(offset))
}

pub fn jmp_rel_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;

    Ok(state.jump_rel(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] + imm
/// ```
pub fn jmp_rel_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? + imm;

    Ok(state.jump_rel(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0]
/// ```
pub fn jmp_rel_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, _, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)?;

    Ok(state.jump_rel(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp rel [[fp + off0] + off1]
/// ```
pub fn jmp_rel_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let deref_value = memory.get_data(state.fp + off0)?;
    let offset = memory.get_data(deref_value + off1)?;

    Ok(state.jump_rel(offset))
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

/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] * [fp + off1]
/// ```
pub fn jmp_rel_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;

    Ok(state.jump_rel(offset))
}

/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] * imm
/// ```
pub fn jmp_rel_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    let offset = memory.get_data(state.fp + off0)? * imm;

    Ok(state.jump_rel(offset))
}

#[cfg(test)]
#[path = "./jump_tests.rs"]
mod jump_tests;
