// Assert instructions for the Cairo M VM.

use cairo_m_common::{Instruction, InstructionError, State, extract_as};

use super::InstructionExecutionError;
use crate::memory::Memory;
use crate::vm::state::VmState;

/// CASM equivalent:
/// ```casm
/// assert [fp + src_off] == imm
/// ```
pub fn assert_eq_fp_imm(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src_off, imm) = extract_as!(instruction, AssertEqFpImm, (src_off, imm));
    let value = memory.get_data(state.fp + src_off)?;
    if value != imm {
        return Err(InstructionExecutionError::Instruction(
            InstructionError::AssertionFailed(value, imm),
        ));
    }
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

#[cfg(test)]
mod assert_tests {
    use cairo_m_common::State;
    use stwo_prover::core::fields::m31::M31;

    use super::*;
    use crate::memory::Memory;

    #[test]
    fn test_assert_eq_fp_imm_success() {
        let mut memory = Memory::from_iter([0, 1, 2].map(Into::into));
        let initial_state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpImm {
            src_off: M31::from(1),
            imm: M31::from(1),
        };
        let returned_state = assert_eq_fp_imm(&mut memory, initial_state, &instruction).unwrap();
        assert_eq!(returned_state.pc, M31::from(2));
    }

    #[test]
    fn test_assert_eq_fp_imm_failure() {
        let mut memory = Memory::from_iter([0, 1, 3].map(Into::into));
        let initial_state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpImm {
            src_off: M31::from(1),
            imm: M31::from(2),
        };
        let error = assert_eq_fp_imm(&mut memory, initial_state, &instruction).unwrap_err();
        match error {
            InstructionExecutionError::Instruction(InstructionError::AssertionFailed(a, b)) => {
                assert_eq!(a, M31::from(1));
                assert_eq!(b, M31::from(2));
            }
            _ => panic!("Expected AssertionFailed error, got: {:?}", error),
        }
    }
}
