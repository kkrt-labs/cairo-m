// Assert instructions for the Cairo M VM.

use cairo_m_common::{Instruction, InstructionError, State};

use super::InstructionExecutionError;
use crate::extract_as;
use crate::memory::Memory;
use crate::vm::state::VmState;

pub fn assert_eq_fp_fp(
    memory: &mut Memory,
    state: State,
    instruction: &Instruction,
) -> Result<State, InstructionExecutionError> {
    let (src0_off, src1_off) = extract_as!(instruction, AssertEqFpFp, (src0_off, src1_off));
    let value0 = memory.get_data(state.fp + src0_off)?;
    let value1 = memory.get_data(state.fp + src1_off)?;
    if value0 != value1 {
        return Err(InstructionExecutionError::Instruction(
            InstructionError::AssertionFailed(value0, value1),
        ));
    }
    Ok(state.advance_by(instruction.size_in_qm31s()))
}

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
    use super::*;
    use crate::memory::Memory;
    use cairo_m_common::State;
    use stwo_prover::core::fields::m31::M31;

    #[test]
    fn test_assert_eq_fp_fp() {
        let mut memory = Memory::from_iter([0, 1, 1].map(Into::into));
        let state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpFp {
            src0_off: M31::from(1),
            src1_off: M31::from(2),
        };
        let state = assert_eq_fp_fp(&mut memory, state, &instruction).unwrap();
        assert_eq!(state.pc, M31::from(2));

        let mut memory = Memory::from_iter([0, 1, 2].map(Into::into));
        let state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpFp {
            src0_off: M31::from(1),
            src1_off: M31::from(2),
        };
        let state = assert_eq_fp_fp(&mut memory, state, &instruction).unwrap_err();
        match state {
            InstructionExecutionError::Instruction(InstructionError::AssertionFailed(a, b)) => {
                assert_eq!(a, M31::from(1));
                assert_eq!(b, M31::from(2));
            }
            _ => panic!("Expected AssertionFailed error, got: {:?}", state),
        }
    }

    #[test]
    fn test_assert_eq_fp_imm() {
        let mut memory = Memory::from_iter([0, 1, 2].map(Into::into));
        let state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpImm {
            src_off: M31::from(1),
            imm: M31::from(1),
        };
        let state = assert_eq_fp_imm(&mut memory, state, &instruction).unwrap();
        assert_eq!(state.pc, M31::from(2));

        let mut memory = Memory::from_iter([0, 1, 3].map(Into::into));
        let state = State {
            pc: M31::from(1),
            fp: M31::from(0),
        };
        let instruction = Instruction::AssertEqFpImm {
            src_off: M31::from(1),
            imm: M31::from(2),
        };
        let state = assert_eq_fp_imm(&mut memory, state, &instruction).unwrap_err();
        match state {
            InstructionExecutionError::Instruction(InstructionError::AssertionFailed(a, b)) => {
                assert_eq!(a, M31::from(1));
                assert_eq!(b, M31::from(2));
            }
            _ => panic!("Expected AssertionFailed error, got: {:?}", state),
        }
    }
}
