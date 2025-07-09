use cairo_m_common::Opcode;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;

use super::*;

const JMP_REL_INITIAL_STATE: State = State {
    pc: M31(3),
    fp: M31(0),
};

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
test_jmp_abs_no_memory!(
    test_jmp_abs_imm,
    Opcode::JmpAbsImm,
    jmp_abs_imm,
    [M31::from(4), Zero::zero(), Zero::zero()],
    4
);

test_jmp_rel_no_memory!(
    test_jmp_rel_imm,
    Opcode::JmpRelImm,
    jmp_rel_imm,
    [M31::from(4), Zero::zero(), Zero::zero()],
    7
);
