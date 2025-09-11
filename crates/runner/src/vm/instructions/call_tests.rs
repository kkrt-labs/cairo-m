use cairo_m_common::State;

use super::*;
use crate::vm::state::VmState;

#[test]
fn test_call_abs_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State::default();
    let instruction = Instruction::CallAbsImm {
        frame_off: M31(3),
        target: M31(7),
    };

    let next_state = call_abs_imm(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
    let expected_state = State {
        pc: M31(7),
        fp: M31(5),
    };

    assert_eq!(memory.locals, expected_memory.locals);
    assert_eq!(memory.heap, expected_memory.heap);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_ret() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State {
        pc: M31(7),
        fp: M31(3),
    };
    let instruction = Instruction::Ret {};

    let next_state = ret(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let expected_state = State {
        pc: M31(12),
        fp: M31(11),
    };

    assert_eq!(memory.locals, expected_memory.locals);
    assert_eq!(memory.heap, expected_memory.heap);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_ret_call_abs_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let initial_state = State::default();
    let call_instruction = Instruction::CallAbsImm {
        frame_off: M31(3),
        target: M31(7),
    };
    let ret_instruction = Instruction::Ret {};

    let call_state = call_abs_imm(&mut memory, initial_state, &call_instruction).unwrap();
    let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));

    assert_eq!(memory.locals, expected_memory.locals);
    assert_eq!(memory.heap, expected_memory.heap);
    assert_eq!(ret_state, initial_state.advance_by(1));
}
