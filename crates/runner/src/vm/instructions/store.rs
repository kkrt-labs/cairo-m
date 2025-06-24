//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.

use cairo_m_common::{Instruction, State};

use crate::memory::{Memory, MemoryError};
use crate::vm::state::VmState;

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] + [fp + off1]
/// ```
pub fn store_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] + imm
/// ```
pub fn store_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? + imm;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] - [fp + off1]
/// ```
pub fn store_sub_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? - memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] - imm
/// ```
pub fn store_sub_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? - imm;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0]
/// ```
pub fn store_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, _, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [[fp + off0] + off1]
/// ```
pub fn store_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.operands;
    let deref_value = memory.get_data(state.fp + off0)?;
    let value = memory.get_data(deref_value + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = imm
/// ```
pub fn store_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, off2] = instruction.operands;
    memory.insert(state.fp + off2, imm.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] * [fp + off1]
/// ```
pub fn store_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] * imm
/// ```
pub fn store_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? * imm;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] / [fp + off1]
/// ```
pub fn store_div_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? / memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] / imm
/// ```
pub fn store_div_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, off2] = instruction.operands;
    let value = memory.get_data(state.fp + off0)? / imm;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

#[cfg(test)]
#[path = "./store_tests.rs"]
mod store_tests;
