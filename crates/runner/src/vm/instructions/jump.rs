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
mod tests {
    use cairo_m_common::Opcode;
    use num_traits::Zero;
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    const JMP_REL_INITIAL_STATE: State = State {
        pc: M31(3),
        fp: M31(0),
    };

    #[test]
    fn test_jmp_abs_add_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([1, 2].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::JmpAbsAddFpFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_abs_add_fp_fp(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsAddFpImm,
            [Zero::zero(), M31::from(4), Zero::zero()],
        );

        let new_state = jmp_abs_add_fp_imm(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsDerefFp,
            [Zero::zero(), Zero::zero(), Zero::zero()],
        );

        let new_state = jmp_abs_deref_fp(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsDoubleDerefFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_abs_double_deref_fp(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsImm,
            [M31::from(4), Zero::zero(), Zero::zero()],
        );

        let new_state = jmp_abs_imm(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsMulFpFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_abs_mul_fp_fp(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpAbsMulFpImm,
            [Zero::zero(), M31::from(4), Zero::zero()],
        );

        let new_state = jmp_abs_mul_fp_imm(&mut memory, state, &instruction)?;

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
        let instruction = Instruction::new(
            Opcode::JmpRelAddFpFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_rel_add_fp_fp(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_add_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let instruction = Instruction::new(
            Opcode::JmpRelAddFpImm,
            [Zero::zero(), M31::from(4), Zero::zero()],
        );

        let new_state = jmp_rel_add_fp_imm(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(9),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let instruction = Instruction::new(
            Opcode::JmpRelDerefFp,
            [Zero::zero(), Zero::zero(), Zero::zero()],
        );

        let new_state = jmp_rel_deref_fp(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(5),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_double_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 3].map(Into::into));
        let instruction = Instruction::new(
            Opcode::JmpRelDoubleDerefFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_rel_double_deref_fp(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(6),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::default();
        let instruction = Instruction::new(
            Opcode::JmpRelImm,
            [M31::from(4), Zero::zero(), Zero::zero()],
        );

        let new_state = jmp_rel_imm(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(7),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_mul_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2, 3].map(Into::into));
        let instruction = Instruction::new(
            Opcode::JmpRelMulFpFp,
            [Zero::zero(), M31::from(1), Zero::zero()],
        );

        let new_state = jmp_rel_mul_fp_fp(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(9),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }

    #[test]
    fn test_jmp_rel_mul_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([2].map(Into::into));
        let instruction = Instruction::new(
            Opcode::JmpRelMulFpImm,
            [Zero::zero(), M31::from(4), Zero::zero()],
        );

        let new_state = jmp_rel_mul_fp_imm(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

        let expected_state = State {
            pc: M31(11),
            fp: M31::zero(),
        };
        assert_eq!(new_state, expected_state);

        Ok(())
    }
}
