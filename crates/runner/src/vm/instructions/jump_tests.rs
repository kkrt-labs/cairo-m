use cairo_m_common::Opcode;
use num_traits::{One, Zero};
use stwo_prover::core::fields::m31::M31;

use super::*;

const JMP_REL_INITIAL_STATE: State = State {
    pc: M31(3),
    fp: M31(0),
};

/// Macro for absolute jump tests
macro_rules! test_jmp_abs {
    ($test_name:ident, $opcode:expr, $func:ident, $memory_values:expr, $operands:expr, $expected_pc:expr) => {
        #[test]
        fn $test_name() -> Result<(), MemoryError> {
            let mut memory = Memory::from_iter($memory_values.map(Into::into));
            let state = State::default();
            let instruction = Instruction::new($opcode, $operands);

            let new_state = $func(&mut memory, state, &instruction)?;

            let expected_state = State {
                pc: M31($expected_pc),
                fp: M31::zero(),
            };
            assert_eq!(new_state, expected_state);

            Ok(())
        }
    };
}

/// Macro for absolute jump tests with empty memory
macro_rules! test_jmp_abs_no_memory {
    ($test_name:ident, $opcode:expr, $func:ident, $operands:expr, $expected_pc:expr) => {
        #[test]
        fn $test_name() -> Result<(), MemoryError> {
            let mut memory = Memory::default();
            let state = State::default();
            let instruction = Instruction::new($opcode, $operands);

            let new_state = $func(&mut memory, state, &instruction)?;

            let expected_state = State {
                pc: M31($expected_pc),
                fp: M31::zero(),
            };
            assert_eq!(new_state, expected_state);

            Ok(())
        }
    };
}

/// Macro for relative jump tests
macro_rules! test_jmp_rel {
    ($test_name:ident, $opcode:expr, $func:ident, $memory_values:expr, $operands:expr, $expected_pc:expr) => {
        #[test]
        fn $test_name() -> Result<(), MemoryError> {
            let mut memory = Memory::from_iter($memory_values.map(Into::into));
            let instruction = Instruction::new($opcode, $operands);

            let new_state = $func(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

            let expected_state = State {
                pc: M31($expected_pc),
                fp: M31::zero(),
            };
            assert_eq!(new_state, expected_state);

            Ok(())
        }
    };
}

/// Macro for relative jump tests with empty memory
macro_rules! test_jmp_rel_no_memory {
    ($test_name:ident, $opcode:expr, $func:ident, $operands:expr, $expected_pc:expr) => {
        #[test]
        fn $test_name() -> Result<(), MemoryError> {
            let mut memory = Memory::default();
            let instruction = Instruction::new($opcode, $operands);

            let new_state = $func(&mut memory, JMP_REL_INITIAL_STATE, &instruction)?;

            let expected_state = State {
                pc: M31($expected_pc),
                fp: M31::zero(),
            };
            assert_eq!(new_state, expected_state);

            Ok(())
        }
    };
}

// Absolute jump tests using macros
test_jmp_abs!(
    test_jmp_abs_add_fp_fp,
    Opcode::JmpAbsAddFpFp,
    jmp_abs_add_fp_fp,
    [1, 2],
    [Zero::zero(), One::one(), Zero::zero()],
    3
);

test_jmp_abs!(
    test_jmp_abs_add_fp_imm,
    Opcode::JmpAbsAddFpImm,
    jmp_abs_add_fp_imm,
    [2],
    [Zero::zero(), M31::from(4), Zero::zero()],
    6
);

test_jmp_abs!(
    test_jmp_abs_deref_fp,
    Opcode::JmpAbsDerefFp,
    jmp_abs_deref_fp,
    [2],
    [Zero::zero(), Zero::zero(), Zero::zero()],
    2
);

test_jmp_abs!(
    test_jmp_abs_double_deref_fp,
    Opcode::JmpAbsDoubleDerefFp,
    jmp_abs_double_deref_fp,
    [0, 3],
    [Zero::zero(), One::one(), Zero::zero()],
    3
);

test_jmp_abs_no_memory!(
    test_jmp_abs_imm,
    Opcode::JmpAbsImm,
    jmp_abs_imm,
    [M31::from(4), Zero::zero(), Zero::zero()],
    4
);

test_jmp_abs!(
    test_jmp_abs_mul_fp_fp,
    Opcode::JmpAbsMulFpFp,
    jmp_abs_mul_fp_fp,
    [2, 3],
    [Zero::zero(), One::one(), Zero::zero()],
    6
);

test_jmp_abs!(
    test_jmp_abs_mul_fp_imm,
    Opcode::JmpAbsMulFpImm,
    jmp_abs_mul_fp_imm,
    [2],
    [Zero::zero(), M31::from(4), Zero::zero()],
    8
);

// Relative jump tests using macros
test_jmp_rel!(
    test_jmp_rel_add_fp_fp,
    Opcode::JmpRelAddFpFp,
    jmp_rel_add_fp_fp,
    [1, 2],
    [Zero::zero(), One::one(), Zero::zero()],
    6
);

test_jmp_rel!(
    test_jmp_rel_add_fp_imm,
    Opcode::JmpRelAddFpImm,
    jmp_rel_add_fp_imm,
    [2],
    [Zero::zero(), M31::from(4), Zero::zero()],
    9
);

test_jmp_rel!(
    test_jmp_rel_deref_fp,
    Opcode::JmpRelDerefFp,
    jmp_rel_deref_fp,
    [2],
    [Zero::zero(), Zero::zero(), Zero::zero()],
    5
);

test_jmp_rel!(
    test_jmp_rel_double_deref_fp,
    Opcode::JmpRelDoubleDerefFp,
    jmp_rel_double_deref_fp,
    [0, 3],
    [Zero::zero(), One::one(), Zero::zero()],
    6
);

test_jmp_rel_no_memory!(
    test_jmp_rel_imm,
    Opcode::JmpRelImm,
    jmp_rel_imm,
    [M31::from(4), Zero::zero(), Zero::zero()],
    7
);

test_jmp_rel!(
    test_jmp_rel_mul_fp_fp,
    Opcode::JmpRelMulFpFp,
    jmp_rel_mul_fp_fp,
    [2, 3],
    [Zero::zero(), One::one(), Zero::zero()],
    9
);

test_jmp_rel!(
    test_jmp_rel_mul_fp_imm,
    Opcode::JmpRelMulFpImm,
    jmp_rel_mul_fp_imm,
    [2],
    [Zero::zero(), M31::from(4), Zero::zero()],
    11
);
