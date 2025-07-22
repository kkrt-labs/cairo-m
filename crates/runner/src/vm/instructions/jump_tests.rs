use cairo_m_common::Instruction;
use stwo_prover::core::fields::m31::M31;

use super::{InstructionExecutionError, *};

#[test]
fn test_jmp_abs_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let state = State::default();
    let instruction = Instruction::JmpAbsImm {
        target: M31::from(4),
    };

    let new_state = jmp_abs_imm(&mut memory, state, &instruction)?;

    assert_eq!(new_state.pc, M31(4));
    assert_eq!(new_state.fp, M31(0));
    Ok(())
}

#[test]
fn test_jmp_rel_imm() -> Result<(), InstructionExecutionError> {
    let mut memory = Memory::default();
    let state = State {
        pc: M31(3),
        fp: M31(0),
    };
    let instruction = Instruction::JmpRelImm {
        offset: M31::from(4),
    };

    let new_state = jmp_rel_imm(&mut memory, state, &instruction)?;

    assert_eq!(new_state.pc, M31(7)); // 3 + 4 = 7
    assert_eq!(new_state.fp, M31(0));
    Ok(())
}
