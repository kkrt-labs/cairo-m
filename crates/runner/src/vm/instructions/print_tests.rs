use cairo_m_common::{Instruction, State};
use stwo_prover::core::fields::m31::M31;

use crate::memory::Memory;
use crate::vm::instructions::InstructionExecutionError;
use crate::vm::instructions::print::{print_m31, print_u32};

#[test]
fn test_print_m31() -> Result<(), InstructionExecutionError> {
    // Setup memory with a value at address 10
    let mut memory = Memory::default();
    memory.insert(M31(10), M31(42).into())?;

    // Setup state with fp = 5, so [fp + 5] = address 10
    let state = State {
        pc: M31(0),
        fp: M31(5),
    };

    // Create PrintM31 instruction with offset 5
    let instruction = Instruction::PrintM31 { offset: M31(5) };

    // Execute the instruction - should print "[PrintM31] [10] = 42"
    let new_state = print_m31(&mut memory, state, &instruction)?;

    // Verify state advances by instruction size (1 QM31)
    assert_eq!(new_state.pc, M31(1));
    assert_eq!(new_state.fp, M31(5));

    // Verify memory trace is NOT modified by print instruction
    // (the initial insert will have created 1 trace entry)
    assert_eq!(memory.trace.borrow().len(), 1);

    Ok(())
}

#[test]
fn test_print_u32() -> Result<(), InstructionExecutionError> {
    // Setup memory with a U32 value 0x12345678 at address 20
    // Low limb: 0x5678, High limb: 0x1234
    let mut memory = Memory::default();
    memory.insert(M31(20), M31(0x5678).into())?; // Low limb
    memory.insert(M31(21), M31(0x1234).into())?; // High limb

    // Setup state with fp = 10, so [fp + 10] = address 20
    let state = State {
        pc: M31(0),
        fp: M31(10),
    };

    // Create PrintU32 instruction with offset 10
    let instruction = Instruction::PrintU32 { offset: M31(10) };

    // Execute the instruction - should print "[PrintU32] [20] = 305419896" (0x12345678)
    let new_state = print_u32(&mut memory, state, &instruction)?;

    // Verify state advances by instruction size (1 QM31)
    assert_eq!(new_state.pc, M31(1));
    assert_eq!(new_state.fp, M31(10));

    // Verify memory trace is NOT modified (no trace entries for print)
    // The initial inserts will have created 2 trace entries, but print_u32 should not add any
    let initial_trace_len = memory.trace.borrow().len();
    assert_eq!(initial_trace_len, 2); // From the two inserts

    Ok(())
}

#[test]
fn test_print_m31_invalid_value() -> Result<(), InstructionExecutionError> {
    // Setup memory with an invalid M31 value (has extension components)
    let mut memory = Memory::default();
    let invalid_value = stwo_prover::core::fields::qm31::QM31::from_u32_unchecked(1, 2, 0, 0);
    memory.insert(M31(10), invalid_value)?;

    // Setup state
    let state = State {
        pc: M31(0),
        fp: M31(5),
    };

    // Create PrintM31 instruction
    let instruction = Instruction::PrintM31 { offset: M31(5) };

    // Execute should fail with BaseFieldProjectionFailed
    let result = print_m31(&mut memory, state, &instruction);
    assert!(matches!(
        result,
        Err(InstructionExecutionError::Memory(
            crate::memory::MemoryError::BaseFieldProjectionFailed { .. }
        ))
    ));

    Ok(())
}

#[test]
fn test_print_u32_invalid_limbs() -> Result<(), InstructionExecutionError> {
    // Setup memory with invalid U32 limbs (exceeding 16-bit range)
    let mut memory = Memory::default();
    memory.insert(M31(20), M31(0x10000).into())?; // Invalid low limb (> 0xFFFF)
    memory.insert(M31(21), M31(0x1234).into())?; // Valid high limb

    // Setup state
    let state = State {
        pc: M31(0),
        fp: M31(10),
    };

    // Create PrintU32 instruction
    let instruction = Instruction::PrintU32 { offset: M31(10) };

    // Execute should fail with U32LimbOutOfRange
    let result = print_u32(&mut memory, state, &instruction);
    assert!(matches!(
        result,
        Err(InstructionExecutionError::Memory(
            crate::memory::MemoryError::U32LimbOutOfRange { .. }
        ))
    ));

    Ok(())
}
