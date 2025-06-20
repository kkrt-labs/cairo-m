//! CALL and RET instructions for the Cairo M VM.
//!
//! CALL instructions handle functions calls, creating new frames.
//! There are relative and absolute function calls.
//!
//! Call-related memory layout convention:
//! ```text
//! [lower addresses]
//! - Function arguments
//! - Return values
//! - Old FP
//! - Return PC
//! [higher addresses]
//! ```
//!
//! The first argument, `off0` is the offset between the current frame pointer and the next frame pointer minus 2.
//! In other words, `next_fp = fp + off0 + 2`.
//! The second argument, `off1` is the destination offset to compute the return address.
//!
//! The function arguments are assumed to be already stored in memory.
//! Considering a function call with N arguments and M return values,
//! the arguments are stored in memory at [fp + off0 - N - M, ..., fp + off0 - M - 1],
//! and the return values have dedicated cells at [fp + off0 - M, fp + off0 - 1].
//!
//! The function call is performed by:
//! - Storing FP in memory at fp + off0.
//! - Storing the return address in memory at fp + off0 + 1.
//! - Updating FP for the new frame: fp + off0 + 2.
//! - Updating PC to the function address based on off1.
//!
//! RET instructions returns control to the caller:
//! - Restore FP from memory, stored at fp - 2.
//! - Update PC to the return address, stored at fp - 1.

use cairo_m_common::Instruction;
use num_traits::One;
use stwo_prover::core::fields::m31::M31;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::State;

/// Call instruction
/// PC update: `next_pc = [fp + off1]`
///
/// CASM equivalent:
/// `call abs [fp + off1]`
pub fn call_abs_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    memory.insert(state.fp + off0, state.fp.into())?;
    memory.insert(state.fp + off0 + M31::one(), (state.pc + M31::one()).into())?;

    let next_pc = memory.get_data(state.fp + off1)?;

    Ok(state.call_abs(next_pc, off0 + M31(2)))
}

/// Call instruction
/// PC update: `next_pc = imm`
///
/// CASM equivalent:
/// `call abs imm`
pub fn call_abs_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    memory.insert(state.fp + off0, state.fp.into())?;
    memory.insert(state.fp + off0 + M31::one(), (state.pc + M31::one()).into())?;

    Ok(state.call_abs(imm, off0 + M31(2)))
}

/// Call instruction
/// PC update: `next_pc = pc + [fp + off1]`
///
/// CASM equivalent:
/// `call rel [fp + off1]`
pub fn call_rel_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.operands;
    memory.insert(state.fp + off0, state.fp.into())?;
    memory.insert(state.fp + off0 + M31::one(), (state.pc + M31::one()).into())?;

    let pc_offset = memory.get_data(state.fp + off1)?;

    Ok(state.call_rel(pc_offset, off0 + M31(2)))
}

/// Call instruction
/// PC update: `next_pc = pc + imm`
///
/// CASM equivalent:
/// `call rel imm`
pub fn call_rel_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.operands;
    memory.insert(state.fp + off0, state.fp.into())?;
    memory.insert(state.fp + off0 + M31::one(), (state.pc + M31::one()).into())?;

    Ok(state.call_rel(imm, off0 + M31(2)))
}

/// Return instruction
/// PC update: `next_pc = [fp - 1]`
/// FP update: `fp = [fp - 2]`
///
/// CASM equivalent:
/// `ret`
pub fn ret(memory: &mut Memory, state: State, _: &Instruction) -> Result<State, MemoryError> {
    let pc = memory.get_data(state.fp - M31::one())?;
    let fp = memory.get_data(state.fp - M31(2))?;

    Ok(state.ret(pc, fp))
}

#[cfg(test)]
mod tests {
    use cairo_m_common::Instruction;

    use super::*;

    #[test]
    fn test_call_abs_fp_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let state = State::default();
        let instruction = Instruction::try_from([11, 3, 0, 0]).unwrap();

        let next_state = call_abs_fp(&mut memory, state, &instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
        let expected_state = State {
            pc: M31(10),
            fp: M31(5),
        };

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(next_state, expected_state);
    }

    #[test]
    fn test_call_abs_imm_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let state = State::default();
        let instruction = Instruction::try_from([12, 3, 7, 0]).unwrap();

        let next_state = call_abs_imm(&mut memory, state, &instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
        let expected_state = State {
            pc: M31(7),
            fp: M31(5),
        };

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(next_state, expected_state);
    }

    #[test]
    fn test_call_rel_fp_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let state = State {
            pc: M31(4),
            fp: M31(0),
        };
        let instruction = Instruction::try_from([13, 3, 0, 0]).unwrap();

        let next_state = call_rel_fp(&mut memory, state, &instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));
        let expected_state = State {
            pc: M31(14),
            fp: M31(5),
        };

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(next_state, expected_state);
    }

    #[test]
    fn test_call_rel_imm_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let state = State {
            pc: M31(4),
            fp: M31(0),
        };
        let instruction = Instruction::try_from([14, 3, 7, 0]).unwrap();

        let next_state = call_rel_imm(&mut memory, state, &instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));
        let expected_state = State {
            pc: M31(11),
            fp: M31(5),
        };

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(next_state, expected_state);
    }

    #[test]
    fn test_ret() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let state = State {
            pc: M31(7),
            fp: M31(3),
        };
        let instruction = Instruction::try_from([15, 0, 0, 0]).unwrap();

        let next_state = ret(&mut memory, state, &instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let expected_state = State {
            pc: M31(12),
            fp: M31(11),
        };

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(next_state, expected_state);
    }

    #[test]
    fn test_ret_call_abs_fp_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let initial_state = State::default();
        let call_instruction = Instruction::try_from([11, 3, 0, 0]).unwrap();
        let ret_instruction = Instruction::try_from([15, 0, 0, 0]).unwrap();

        let call_state = call_abs_fp(&mut memory, initial_state, &call_instruction).unwrap();
        let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(ret_state, initial_state.advance());
    }

    #[test]
    fn test_ret_call_abs_imm_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let initial_state = State::default();
        let call_instruction = Instruction::try_from([12, 3, 7, 0]).unwrap();
        let ret_instruction = Instruction::try_from([15, 0, 0, 0]).unwrap();

        let call_state = call_abs_imm(&mut memory, initial_state, &call_instruction).unwrap();
        let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(ret_state, initial_state.advance());
    }

    #[test]
    fn test_ret_call_rel_fp_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let initial_state = State {
            pc: M31(4),
            fp: M31(0),
        };
        let call_instruction = Instruction::try_from([13, 3, 0, 0]).unwrap();
        let ret_instruction = Instruction::try_from([15, 0, 0, 0]).unwrap();

        let call_state = call_rel_fp(&mut memory, initial_state, &call_instruction).unwrap();
        let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(ret_state, initial_state.advance());
    }

    #[test]
    fn test_ret_call_rel_imm_2_args() {
        let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
        let initial_state = State {
            pc: M31(4),
            fp: M31(0),
        };
        let call_instruction = Instruction::try_from([14, 3, 7, 0]).unwrap();
        let ret_instruction = Instruction::try_from([15, 0, 0, 0]).unwrap();

        let call_state = call_rel_imm(&mut memory, initial_state, &call_instruction).unwrap();
        let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

        let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));

        assert_eq!(memory.data, expected_memory.data);
        assert_eq!(ret_state, initial_state.advance());
    }
}
