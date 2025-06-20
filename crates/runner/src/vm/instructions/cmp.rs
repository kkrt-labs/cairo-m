//! Comparison operations for the Cairo M VM.

use cairo_m_common::Instruction;
use stwo_prover::core::fields::m31::M31;

use crate::vm::{Memory, MemoryError, State};

/// Compare if two fp-offset values are equal and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpEqFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_eq_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0 == arg1 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value equals immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpEqFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_eq_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0 == imm { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if two fp-offset values are not equal and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpNeqFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_neq_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0 != arg1 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value not equals immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpNeqFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_neq_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0 != imm { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if first fp-offset value is less than second and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpLtFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_lt_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0.0 < arg1.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value is less than immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpLtFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_lt_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0.0 < imm.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if first fp-offset value is greater than second and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpGtFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_gt_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0.0 > arg1.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value is greater than immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpGtFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_gt_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0.0 > imm.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if first fp-offset value is less than or equal to second and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpLeFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_le_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0.0 <= arg1.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value is less than or equal to immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpLeFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_le_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0.0 <= imm.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if first fp-offset value is greater than or equal to second and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpGeFpFp instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_ge_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let arg1_offset = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let arg1 = memory.get_data(state.fp + arg1_offset)?;
    let result = if arg0.0 >= arg1.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

/// Compare if fp-offset value is greater than or equal to immediate and store result (1 or 0)
///
/// ## Arguments
///
/// * `memory` - The memory instance
/// * `state` - The current VM state
/// * `instruction` - The CmpGeFpImm instruction
///
/// ## Returns
///
/// The updated state with incremented PC
pub fn cmp_ge_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let arg0_offset = instruction.op0();
    let imm = instruction.op1();
    let dst_offset = instruction.op2();

    let arg0 = memory.get_data(state.fp + arg0_offset)?;
    let result = if arg0.0 >= imm.0 { M31(1) } else { M31(0) };

    memory.insert(state.fp + dst_offset, result.into())?;
    Ok(state.advance())
}

#[cfg(test)]
#[path = "./cmp_test.rs"]
mod cmp_test;
