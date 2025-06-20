//! STORE instructions for the Cairo M VM.
//!
//! STORE instructions are used to store values in the memory.

use cairo_m_common::Instruction;

use crate::memory::{Memory, MemoryError};
use crate::vm::State;

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
mod tests {
    use cairo_m_common::Opcode;
    use num_traits::{One, Zero};
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    #[test]
    fn test_store_add_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreAddFpFp,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_add_fp_fp(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 7, 11].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_add_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
        let expected_memory = Memory::from_iter([0, 4, 7, 6].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreAddFpImm,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_add_fp_imm(&mut memory, state, &instruction)?;

        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_sub_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 7, 4].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreSubFpFp,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_sub_fp_fp(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 7, 4, 3].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_sub_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreSubFpImm,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_sub_fp_imm(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 7, 2].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDerefFp,
            [One::one(), Zero::zero(), M31::from(2)],
        );

        let new_state = store_deref_fp(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 4].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_double_deref_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 1, 7, 9].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDoubleDerefFp,
            [One::one(), M31::from(2), M31::from(2)],
        );

        let new_state = store_double_deref_fp(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 1, 9, 9].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4].map(Into::into));
        let state = State::default();
        let instruction =
            Instruction::new(Opcode::StoreImm, [One::one(), Zero::zero(), M31::from(2)]);

        let new_state = store_imm(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 1].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_mul_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreMulFpFp,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_mul_fp_fp(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 7, 28].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_mul_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreMulFpImm,
            [One::one(), M31::from(2), M31::from(2)],
        );

        let new_state = store_mul_fp_imm(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 8].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    fn test_store_div_fp_fp() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4, 7].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDivFpFp,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let new_state = store_div_fp_fp(&mut memory, state, &instruction)?;

        let expected_div = M31::from(4) / M31::from(7);
        let expected_memory = Memory::from_iter([0, 4, 7, expected_div.0].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    #[should_panic(expected = "0 has no inverse")]
    fn test_store_div_fp_fp_by_zero() {
        let mut memory = Memory::from_iter([0, 4, 0].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDivFpFp,
            [One::one(), M31::from(2), M31::from(3)],
        );

        let _ = store_div_fp_fp(&mut memory, state, &instruction);
    }

    #[test]
    fn test_store_div_fp_imm() -> Result<(), MemoryError> {
        let mut memory = Memory::from_iter([0, 4].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDivFpImm,
            [One::one(), M31::from(2), M31::from(2)],
        );

        let new_state = store_div_fp_imm(&mut memory, state, &instruction)?;

        let expected_memory = Memory::from_iter([0, 4, 2].map(Into::into));
        assert_eq!(memory.data, expected_memory.data);

        assert_eq!(new_state.fp, M31::zero());
        assert_eq!(new_state.pc, M31::one());

        Ok(())
    }

    #[test]
    #[should_panic(expected = "0 has no inverse")]
    fn test_store_div_fp_imm_by_zero() {
        let mut memory = Memory::from_iter([0, 4].map(Into::into));
        let state = State::default();
        let instruction = Instruction::new(
            Opcode::StoreDivFpImm,
            [One::one(), Zero::zero(), M31::from(2)],
        );

        let _ = store_div_fp_imm(&mut memory, state, &instruction);
    }
}
