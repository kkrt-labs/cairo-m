//! JUMP instructions for the Cairo M VM.

use crate::{
    memory::{Memory, MemoryError},
    vm::{instructions::Instruction, State},
};

/// Jump to the absolute address of the sum of the memory values at the offsets `fp + off0` and `fp + off1`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] + [fp + off1]
/// ```
pub fn jmp_abs_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;

    Ok(state.jump_abs(offset))
}

/// Jump to the absolute address of the sum of the memory value at the offset `fp + off0` and the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] + imm
/// ```
pub fn jmp_abs_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? + imm;

    Ok(state.jump_abs(offset))
}

/// Jump to the absolute address of the memory value at the offset `fp + off0`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0]
/// ```
pub fn jmp_abs_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, _, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)?;

    Ok(state.jump_abs(offset))
}

/// Jump to the absolute address of the memory value at the offset `[[fp + off0] + off1]`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [[fp + off0] + off1]
/// ```
pub fn jmp_abs_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let deref_value = memory.get_data(state.fp + off0)?;
    let offset = memory.get_data(deref_value + off1)?;

    Ok(state.jump_abs(offset))
}

/// Jump to the absolute address of the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs imm
/// ```
pub const fn jmp_abs_imm(
    _memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, _] = instruction.args;

    Ok(state.jump_abs(imm))
}

/// Jump to the absolute address of the product of the memory values at the offsets `fp + off0` and `fp + off1`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] * [fp + off1]
/// ```
pub fn jmp_abs_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;

    Ok(state.jump_abs(offset))
}

/// Jump to the absolute address of the product of the memory value at the offset `fp + off0` and the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp abs [fp + off0] * imm
/// ```
pub fn jmp_abs_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? * imm;

    Ok(state.jump_abs(offset))
}

/// Relative Jump - Add to PC the sum of the memory values at the offsets `fp + off0` and `fp + off1`.
pub fn jmp_rel_add_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? + memory.get_data(state.fp + off1)?;

    Ok(state.jump_rel(offset))
}

/// Relative Jump - Add to PC the sum of the memory value at the offset `fp + off0` and the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] + imm
/// ```
pub fn jmp_rel_add_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? + imm;

    Ok(state.jump_rel(offset))
}

/// Relative Jump - Add to PC the memory value at the offset `fp + off0`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0]
/// ```
pub fn jmp_rel_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, _, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)?;

    Ok(state.jump_rel(offset))
}

/// Relative Jump - Add to PC the memory value at the offset `[[fp + off0] + off1]`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel [[fp + off0] + off1]
/// ```
pub fn jmp_rel_double_deref_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let deref_value = memory.get_data(state.fp + off0)?;
    let offset = memory.get_data(deref_value + off1)?;

    Ok(state.jump_rel(offset))
}

/// Relative Jump - Add to PC the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel imm
/// ```
pub fn jmp_rel_imm(
    _memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [imm, _, _] = instruction.args;

    Ok(state.jump_rel(imm))
}

/// Relative Jump - Add to PC the product of the memory values at the offsets `fp + off0` and `fp + off1`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] * [fp + off1]
/// ```
pub fn jmp_rel_mul_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, off1, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? * memory.get_data(state.fp + off1)?;

    Ok(state.jump_rel(offset))
}

/// Relative Jump - Add to PC the product of the memory value at the offset `fp + off0` and the immediate value `imm`.
///
/// CASM equivalent:
/// ```casm
/// jmp rel [fp + off0] * imm
/// ```
pub fn jmp_rel_mul_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: Instruction,
) -> Result<State, MemoryError> {
    let [off0, imm, _] = instruction.args;
    let offset = memory.get_data(state.fp + off0)? * imm;

    Ok(state.jump_rel(offset))
}

#[cfg(test)]
mod tests {
    use num_traits::Zero;
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    #[test]
    fn test_jmp_abs_add_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([1, 2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([16, 0, 1, 0]);

        let new_state = jmp_abs_add_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(3),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_add_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([17, 0, 4, 0]);

        let new_state = jmp_abs_add_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([18, 0, 0, 0]);

        let new_state = jmp_abs_deref_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(2),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_double_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([19, 0, 1, 0]);

        let new_state = jmp_abs_double_deref_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(3),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::default();
        let state = State::default();
        let instruction = Instruction::from([20, 4, 0, 0]);

        let new_state = jmp_abs_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(4),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_mul_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([21, 0, 1, 0]);

        let new_state = jmp_abs_mul_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_abs_mul_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([22, 0, 4, 0]);

        let new_state = jmp_abs_mul_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(8),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_add_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([1, 2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([23, 0, 1, 0]);

        let new_state = jmp_rel_add_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(3),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_add_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([24, 0, 4, 0]);

        let new_state = jmp_rel_add_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([25, 0, 0, 0]);

        let new_state = jmp_rel_deref_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(2),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_double_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([26, 0, 1, 0]);

        let new_state = jmp_rel_double_deref_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(3),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::default();
        let state = State::default();
        let instruction = Instruction::from([27, 4, 0, 0]);

        let new_state = jmp_rel_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(4),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_mul_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2, 3].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([28, 0, 1, 0]);

        let new_state = jmp_rel_mul_fp_fp(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_mul_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::from([29, 0, 4, 0]);

        let new_state = jmp_rel_mul_fp_imm(&mut memory, state, instruction)?;

        let expected_state = State {
            pc: M31(8),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }
}
