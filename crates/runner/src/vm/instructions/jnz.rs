//! JNZ instructions for the Cairo M VM.
//!
//! JNZ are conditional relative jumps.
//! The condition offset is the first instruction argument.
//! The destination offset when the condition is true is the second instruction argument.

use num_traits::Zero;

use crate::{
    memory::{Memory, MemoryError},
    vm::{instructions::Instruction, state::State},
};

/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off1] if [fp + off0] != 0
/// ```
pub fn jnz_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let condition = memory.get_data(state.fp + off0)?;
    let new_state = if !condition.is_zero() {
        state.jump_rel(memory.get_data(state.fp + off1)?)
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
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.args;
    let condition = memory.get_data(state.fp + off0)?;
    let new_state = if !condition.is_zero() {
        state.jump_rel(imm)
    } else {
        state.advance()
    };

    Ok(new_state)
}

#[cfg(test)]
mod tests {
    use num_traits::One;
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    #[test]
    fn test_jnz_fp_fp_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([30, 0, 1, 0]);

        let new_state = jnz_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31::one(),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_fp_not_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([7, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([30, 0, 1, 0]);

        let new_state = jnz_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(3),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_imm_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([31, 0, 8, 0]);

        let new_state = jnz_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31::one(),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_imm_not_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([7].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([31, 0, 8, 0]);

        let new_state = jnz_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(8),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }
}
