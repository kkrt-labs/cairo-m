use cairo_m_common::{Opcode, State};

use super::*;
use crate::vm::state::VmState;
use crate::vm::test_utils::*;

#[test]
fn test_call_abs_fp_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::CallAbsFp, 3, 0, 0);

    let next_state = call_abs_fp(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
    let expected_state = State {
        pc: M31(10),
        fp: M31(5),
    };

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_call_abs_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State::default();
    let instruction = instr!(Opcode::CallAbsImm, 3, 7, 0);

    let next_state = call_abs_imm(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
    let expected_state = State {
        pc: M31(7),
        fp: M31(5),
    };

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_call_rel_fp_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State {
        pc: M31(4),
        fp: M31(0),
    };
    let instruction = instr!(Opcode::CallRelFp, 3, 0, 0);

    let next_state = call_rel_fp(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));
    let expected_state = State {
        pc: M31(14),
        fp: M31(5),
    };

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_call_rel_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State {
        pc: M31(4),
        fp: M31(0),
    };
    let instruction = instr!(Opcode::CallRelImm, 3, 7, 0);

    let next_state = call_rel_imm(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));
    let expected_state = State {
        pc: M31(11),
        fp: M31(5),
    };

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_ret() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let state = State {
        pc: M31(7),
        fp: M31(3),
    };
    let instruction = instr!(Opcode::Ret, 0, 0, 0);

    let next_state = ret(&mut memory, state, &instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let expected_state = State {
        pc: M31(12),
        fp: M31(11),
    };

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(next_state, expected_state);
}

#[test]
fn test_ret_call_abs_fp_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let initial_state = State::default();
    let call_instruction = instr!(Opcode::CallAbsFp, 3, 0, 0);
    let ret_instruction = instr!(Opcode::Ret, 0, 0, 0);

    let call_state = call_abs_fp(&mut memory, initial_state, &call_instruction).unwrap();
    let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));
    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(ret_state, initial_state.advance());
}

#[test]
fn test_ret_call_abs_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let initial_state = State::default();
    let call_instruction = instr!(Opcode::CallAbsImm, 3, 7, 0);
    let ret_instruction = instr!(Opcode::Ret, 0, 0, 0);

    let call_state = call_abs_imm(&mut memory, initial_state, &call_instruction).unwrap();
    let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 1].map(Into::into));

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(ret_state, initial_state.advance());
}

#[test]
fn test_ret_call_rel_fp_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let initial_state = State {
        pc: M31(4),
        fp: M31(0),
    };
    let call_instruction = instr!(Opcode::CallRelFp, 3, 0, 0);
    let ret_instruction = instr!(Opcode::Ret, 0, 0, 0);

    let call_state = call_rel_fp(&mut memory, initial_state, &call_instruction).unwrap();
    let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(ret_state, initial_state.advance());
}

#[test]
fn test_ret_call_rel_imm_2_args() {
    let mut memory = Memory::from_iter([10, 11, 12].map(Into::into));
    let initial_state = State {
        pc: M31(4),
        fp: M31(0),
    };
    let call_instruction = instr!(Opcode::CallRelImm, 3, 7, 0);
    let ret_instruction = instr!(Opcode::Ret, 0, 0, 0);

    let call_state = call_rel_imm(&mut memory, initial_state, &call_instruction).unwrap();
    let ret_state = ret(&mut memory, call_state, &ret_instruction).unwrap();

    let expected_memory = Memory::from_iter([10, 11, 12, 0, 5].map(Into::into));

    assert_eq!(memory.data, expected_memory.data);
    assert_eq!(ret_state, initial_state.advance());
}
