//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.
//! They are used to store the result of an operation or the value of a variable.

use crate::{
    memory::{Memory, MemoryError},
    vm::{instructions::Instruction, State},
};

/// Store the sum of the values at the offsets `fp + off0` and `fp + off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 0
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] + [fp + off1]
/// ```
pub fn store_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the sum of the value at the offset `fp + off0` and the immediate value `off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 1
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] + off1
/// ```
pub fn store_add_imm_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? + off1;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the subtraction of the values at the offsets `fp + off0` and `fp + off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 2
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] - [fp + off1]
/// ```
pub fn store_sub_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? - memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the subtraction of the value at the offset `fp + off0` and the immediate value `off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 3
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] - off1
/// ```
pub fn store_sub_imm_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? - off1;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the value at the offset `fp + off0` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 4
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0]
/// ```
pub fn store_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, _off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the double derefence of the value at the offset `fp + off0` and the offset `off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 5
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [[fp + off0] + off1]
/// ```
pub fn store_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let deref_value = memory.get_data(state.fp + off0)?;
    let value = memory.get_data(deref_value + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the immediate value `off0` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 6
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = off0
/// ```
pub fn store_imm(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, _off1, off2] = instruction.args;
    let value = off0;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the product of the values at the offsets `fp + off0` and `fp + off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 7
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] * [fp + off1]
/// ```
pub fn store_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the product of the value at the offset `fp + off0` and the immediate value `off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 8
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] * off1
/// ```
pub fn store_mul_imm_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the division of the values at the offsets `fp + off0` and `fp + off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 9
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] / [fp + off1]
/// ```
pub fn store_div_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? / memory.get_data(state.fp + off1)?;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}

/// Store the division of the value at the offset `fp + off0` and the immediate value `off1` in the memory at the offset `fp + off2`.
///
/// OPCODE ID: 10
///
/// CASM equivalent:
/// ```casm
/// [fp + off2] = [fp + off0] / off1
/// ```
pub fn store_div_imm_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, off2] = instruction.args;
    let value = memory.get_data(state.fp + off0)? / off1;
    memory.insert(state.fp + off2, value.into())?;

    Ok(state.advance())
}
