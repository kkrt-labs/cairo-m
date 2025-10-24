use stwo::core::fields::m31::M31;

use super::{InstructionExecutionError, *};

const JNZ_INITIAL_STATE: State = State {
    pc: M31(3),
    fp: M31(0),
};

#[test]
fn test_jnz_fp_imm_zero() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([0].map(Into::into));
    let instruction = Instruction::JnzFpImm {
        cond_off: M31(0),
        offset: M31(8),
    };

    let new_state = jnz_fp_imm(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

    let expected_state = State {
        pc: M31(4),
        fp: M31::zero(),
    };
    assert_eq!(new_state, expected_state);

    Ok(())
}

#[test]
fn test_jnz_fp_imm_not_zero() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::from_iter([7].map(Into::into));
    let instruction = Instruction::JnzFpImm {
        cond_off: M31(0),
        offset: M31(8),
    };

    let new_state = jnz_fp_imm(&mut memory, JNZ_INITIAL_STATE, &instruction)?;

    let expected_state = State {
        pc: M31(11),
        fp: M31::zero(),
    };
    assert_eq!(new_state, expected_state);

    Ok(())
}
