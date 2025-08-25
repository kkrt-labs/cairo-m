//! PRINT instructions for the Cairo M VM.
//!
//! PRINT instructions are no-op instructions that output values to stdout for debugging purposes.

use crate::vm::state::VmState;
use cairo_m_common::{Instruction, State};

use super::InstructionExecutionError;
use crate::extract_as;
use crate::memory::Memory;

/// Execute PrintM31 instruction.
/// Reads a value from [fp + offset] and prints it as an M31 field element.
/// This is a debugging instruction that doesn't modify the trace.
pub fn print_m31(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let offset = extract_as!(instruction, PrintM31, offset);
    let addr = state.fp + offset;
    let value = memory.get_data_no_trace(addr)?;
    println!("[PrintM31] [{}] = {}", addr, value);
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

/// Execute PrintU32 instruction.
/// Reads a U32 value from [fp + offset] and prints it as a 32-bit unsigned integer.
/// This is a debugging instruction that doesn't modify the trace.
pub fn print_u32(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let offset = extract_as!(instruction, PrintU32, offset);
    let addr = state.fp + offset;
    let value = memory.get_u32_no_trace(addr)?;
    println!("[PrintU32] [{}] = {}", addr, value);
    Ok(state.advance_by(instruction.size_in_qm31s()))
}
