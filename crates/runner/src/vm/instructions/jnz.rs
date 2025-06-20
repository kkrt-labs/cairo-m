//! JNZ instructions for the Cairo M VM.
//!
//! JNZ are conditional relative jumps.
//! The condition offset is the first instruction argument.
//! The destination offset when the condition is true is the second instruction argument.

use cairo_m_common::Instruction;
use num_traits::Zero;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::State;

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
mod tests {
    use cairo_m_common::{Instruction, Opcode};
    use num_traits::One;
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    const JNZ_INITIAL_STATE: State = State {
        pc: M31(3),
        fp: M31(0),
    };

    #[test]
    fn test_jnz_fp_fp_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 3].map(Into::into));
        let instruction =
            Instruction::new(Opcode::JnzFpFp, [Zero::zero(), One::one(), Zero::zero()]);

        let new_state = jnz_fp_fp(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(4),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_fp_not_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([7, 3].map(Into::into));
        let instruction =
            Instruction::new(Opcode::JnzFpFp, [Zero::zero(), One::one(), Zero::zero()]);

        let new_state = jnz_fp_fp(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_imm_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0].map(Into::into));
        let instruction =
            Instruction::new(Opcode::JnzFpImm, [Zero::zero(), M31::from(8), Zero::zero()]);

        let new_state = jnz_fp_imm(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(4),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jnz_fp_imm_not_zero() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([7].map(Into::into));
        let instruction =
            Instruction::new(Opcode::JnzFpImm, [Zero::zero(), M31::from(8), Zero::zero()]);

        let new_state = jnz_fp_imm(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(11),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }
}
