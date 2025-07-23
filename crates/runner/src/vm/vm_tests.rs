use std::fs::File;
use std::io::Read;

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Opcode, Program, State};
use num_traits::{One, Zero};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

// Import test utilities
use super::test_utils::*;
use crate::RunnerOptions;
use crate::memory::Memory;
use crate::vm::{VM, VmError};

#[test]
fn test_program_from_vec_instructions() {
    let instructions = vec![
        instr!(Opcode::StoreAddFpImm, 2, 3, 4),
        instr!(Opcode::StoreDoubleDerefFp, 6, 7, 8),
    ];
    let program: Program = Program::from(instructions.clone());

    assert_eq!(program.instructions, instructions);
}

#[test]
fn test_vm_try_from() {
    // Create a simple program with two instructions
    let instructions = vec![
        instr!(Opcode::StoreAddFpImm, 2, 3, 4),
        instr!(Opcode::StoreDoubleDerefFp, 6, 7, 8),
    ];
    let program: Program = instructions.clone().into();

    let vm = VM::try_from(&program).unwrap();

    // Check that PC is set to 0 (entrypoint)
    // Check that FP is set right after the bytecode (2 instructions)
    assert_vm_state!(vm.state, 0, 2);

    // Check that the first instruction is in memory at address 0
    let loaded_instruction_qm31 = vm.memory.get_instruction(M31::zero()).unwrap();
    let loaded_instruction: Instruction = loaded_instruction_qm31.try_into().unwrap();
    assert_eq!(loaded_instruction.opcode, Opcode::StoreAddFpImm);
    assert_eq!(loaded_instruction, instructions[0]);

    // Check that the second instruction is in memory at address 1
    let loaded_instruction_qm31_2 = vm.memory.get_instruction(M31::one()).unwrap();
    let loaded_instruction_2: Instruction = loaded_instruction_qm31_2.try_into().unwrap();
    assert_eq!(loaded_instruction_2.opcode, Opcode::StoreDoubleDerefFp);
    assert_eq!(loaded_instruction_2, instructions[1]);
}

#[test]
fn test_step_single_instruction() {
    // Create a program with a single store_imm instruction: [fp + 0] = 42
    let instructions = vec![store_imm!(42, 0)];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Initial state should have PC = 0, FP = 1
    assert_vm_state!(vm.state, 0, 1);

    // Execute one step
    let result = vm.step();
    assert!(result.is_ok());

    // PC should have advanced to 1, FP should be the same
    assert_vm_state!(vm.state, 1, 1);

    // The value 42 should be stored at memory[fp + 0] = memory[1]
    assert_memory_value!(vm, addr = 1, value = 42);
}

#[test]
fn test_step_invalid_instruction() {
    let program = Program::from(vec![]);
    let mut vm = VM::try_from(&program).unwrap();
    // Load an invalid opcode in memory
    let bad_instr = QM31::from_m31_array([
        M31::from(2_u32.pow(30)),
        Zero::zero(),
        Zero::zero(),
        Zero::zero(),
    ]);
    vm.memory.insert(M31::zero(), bad_instr).unwrap();

    // Step should return an error for invalid opcode
    let result = vm.step();
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        VmError::Instruction(InstructionError::InvalidOpcode(M31(1073741824))) // 2^30, matches macro use pattern binding
    ));
}

#[test]
fn test_execute_empty_program() {
    let program = Program::from(vec![]);
    let mut vm = VM::try_from(&program).unwrap();
    let result = vm.execute(RunnerOptions::default().max_steps);
    assert!(result.is_ok());
    assert_vm_state!(vm.state, 0, 0);
    assert_eq!(vm.memory.data.len(), 0);
}

#[test]
fn test_execute_single_instruction() {
    // Create a program with a single store_imm instruction: [fp + 0] = 42
    let instructions = vec![store_imm!(42, 0)];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    let result = vm.execute(RunnerOptions::default().max_steps);
    assert!(result.is_ok());

    // PC should be at final position (memory.len() = 1)
    assert_vm_state!(vm.state, 1, 1);

    // The value should be stored correctly at fp + 0 = 1 + 0 = 1
    assert_memory_value!(vm, addr = 1, value = 42);
}

#[test]
fn test_execute_multiple_instructions() {
    // Create a program with multiple instructions:
    // 1. [fp + 0] = 10 (store_imm)
    // 2. [fp + 1] = 5 (store_imm)
    // 3. [fp + 2] = [fp + 0] + [fp + 1] (store_add_fp_fp)
    let instructions = vec![
        store_imm!(10, 0),                     // [fp + 0] = 10
        store_imm!(5, 1),                      // [fp + 1] = 5
        instr!(Opcode::StoreAddFpFp, 0, 1, 2), // [fp + 2] = [fp + 0] + [fp + 1]
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Initial state
    assert_vm_state!(vm.state, 0, 3); // FP should be after 3 instructions

    let result = vm.execute(RunnerOptions::default().max_steps);
    assert!(result.is_ok());

    // PC should be at final position (memory.len() = 3)
    assert_vm_state!(vm.state, 3, 3);

    // Check the computed values
    assert_memory_value!(vm, addr = 3, value = 10); // [fp + 0] = 10
    assert_memory_value!(vm, addr = 4, value = 5); // [fp + 1] = 5
    assert_memory_value!(vm, addr = 5, value = 15); // [fp + 2] = 15
}

#[test]
fn test_execute_with_error() {
    // Create a program with an invalid instructions

    let instructions = [
        QM31::from_m31_array([M31::from(5), M31::from(10), Zero::zero(), Zero::zero()]), // Valid: [fp + 0] = 10
        QM31::from_m31_array([M31::from(99), Zero::zero(), Zero::zero(), Zero::zero()]), // Invalid: opcode 99
    ];
    let initial_memory = Memory::from_iter(instructions);
    let mut vm = VM {
        final_pc: M31::from(instructions.len() as u32),
        initial_memory: instructions.to_vec(),
        memory: initial_memory,
        state: State {
            pc: M31::zero(),
            fp: M31::from(instructions.len() as u32),
        },
        program_length: M31::from(instructions.len() as u32),
        trace: vec![],
        segments: vec![],
    };
    // Execute should fail when it hits the invalid instruction
    let result = vm.execute(RunnerOptions::default().max_steps);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        VmError::Instruction(InstructionError::InvalidOpcode(M31(99)))
    ));

    // PC should be at 1 (where it failed)
    // FP should be at 2 (after the valid instruction)
    assert_vm_state!(vm.state, 1, 2);

    // First instruction should have executed successfully
    let stored_value = vm.memory.get_data(M31(2)).unwrap();
    assert_eq!(stored_value, M31(10));
}

#[test]
fn test_execute_arithmetic_operations() {
    // Test a program that performs various arithmetic operations
    let instructions = vec![
        store_imm!(12, 0),                     // [fp + 0] = 12
        store_imm!(3, 1),                      // [fp + 1] = 3
        instr!(Opcode::StoreMulFpFp, 0, 1, 2), // [fp + 2] = [fp + 0] * [fp + 1] = 36
        instr!(Opcode::StoreDivFpFp, 2, 1, 3), // [fp + 3] = [fp + 2] / [fp + 1] = 12
        instr!(Opcode::StoreSubFpFp, 3, 0, 4), // [fp + 4] = [fp + 3] - [fp + 0] = 0
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    let result = vm.execute(RunnerOptions::default().max_steps);
    assert!(result.is_ok());

    // Check all computed values
    assert_memory_value!(vm, addr = 5, value = 12); // original 12
    assert_memory_value!(vm, addr = 6, value = 3); // original 3
    assert_memory_value!(vm, addr = 7, value = 36); // 12 * 3
    assert_memory_value!(vm, addr = 8, value = 12); // 36 / 3
    assert_memory_value!(vm, addr = 9, value = 0); // 12 - 12
}

#[test]
fn test_run_from_entrypoint() {
    let instructions = vec![
        instr!(Opcode::StoreImm, 10, 0, 0),     // [fp] = 10
        instr!(Opcode::StoreAddFpImm, 0, 5, 1), // [fp + 1] = [fp] + 5
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Initial FP is 2 in the default case, we add an offset of 2.
    // We run the program from PC = 1, so the first instruction should be ignored.
    vm.run_from_entrypoint(1, 2, &[], 0, &RunnerOptions::default())
        .unwrap();
    assert_vm_state!(vm.state, 2, 4);
    assert_eq!(
        vm.memory.get_data(vm.state.fp + M31::one()).unwrap(),
        M31(5)
    );
}

#[test]
fn test_serialize_trace() {
    // Create a program with two instructions to generate a trace.
    let instructions = vec![
        instr!(Opcode::StoreImm, 10, 0, 0), // [fp + 0] = 10
        instr!(Opcode::StoreImm, 20, 0, 1), // [fp + 1] = 20
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Execute the program to generate a trace.
    assert!(vm.execute(RunnerOptions::default().max_steps).is_ok());

    // The trace should have 3 entries, one for each instruction executed.
    // The last one is the final state of the VM.
    assert_eq!(vm.trace.len(), 3);

    // Verify the trace contents.
    assert_eq!(
        vm.trace[0],
        State {
            pc: M31::zero(),
            fp: M31(2)
        }
    );
    assert_eq!(
        vm.trace[1],
        State {
            pc: M31::one(),
            fp: M31(2)
        }
    );

    // Finalize the segment to move trace data into segments
    vm.finalize_segment(true); // Last segment

    // Serialize the trace from the first segment and verify its contents.
    assert_eq!(vm.segments.len(), 1);
    let serialized_trace = vm.segments[0].serialize_segment_trace();

    // Expected serialized data:
    // Entry 1: fp=2, pc=0.
    // Entry 2: fp=2, pc=1.
    // Entry 3: fp=2, pc=2. (final state)
    let expected_bytes = Vec::from([2, 0, 2, 1, 2, 2].map(u32::to_le_bytes).as_flattened());

    assert_eq!(serialized_trace, expected_bytes);
}

/// Reference implementation of Fibonacci sequence for diff testing.
fn fib(n: u32) -> u32 {
    let mut a = 0;
    let mut b = 1;
    for _ in 0..n {
        let temp = a;
        a = b;
        b += temp;
    }
    a
}

/// Runs a Fibonacci program on the VM and asserts the result against the reference implementation.
/// The program is written in Cairo M assembly and performs the following steps:
/// 1. **Setup**:
///    - `[fp+0]` is initialized with `n` (the loop counter).
///    - `[fp+1]` is initialized with `0` (Fibonacci number `a = F_0`).
///    - `[fp+2]` is initialized with `1` (Fibonacci number `b = F_1`).
///
/// 2. **Loop Condition**:
///    - Checks if the counter at `[fp+0]` is zero.
///    - If `counter != 0`, it jumps to the loop body.
///    - If `counter == 0`, it jumps to the end of the program.
///
/// 3. **Loop Body**:
///    - `tmp = a` (`[fp+3] = [fp+1]`)
///    - `a = b` (`[fp+1] = [fp+2]`)
///    - `b = tmp + b` (`[fp+2] = [fp+3] + [fp+2]`)
///    - `counter--` (`[fp+0] = [fp+0] - 1`)
///    - Jumps back to the loop condition.
///
/// After `n` iterations, `[fp+1]` will hold `F(n)` and `[fp+2]` will hold `F(n+1)`.
fn run_fib_test(n: u32) {
    let instructions = vec![
        // Setup
        instr!(Opcode::StoreImm, n, 0, 0), // store_imm: [fp+0] = counter
        instr!(Opcode::StoreImm, 0, 0, 1), // store_imm: [fp+1] = a = F_0 = 0
        instr!(Opcode::StoreImm, 1, 0, 2), // store_imm: [fp+2] = b = F_1 = 1
        // Loop condition check
        // while counter != 0 jump to loop body
        instr!(Opcode::JnzFpImm, 0, 2, 0), // jnz_fp_imm: jmp rel 2 if [fp + 0] != 0  (pc=3 here, pc=5 in beginning of loop body)
        // Exit jump if counter was 0
        instr!(Opcode::JmpAbsImm, 10, 0, 0), // jmp_abs_imm: jmp abs 10
        // Loop body
        instr!(Opcode::StoreAddFpImm, 1, 0, 3), // store_add_fp_imm: [fp+3] = [fp+1] + 0 (tmp = a)
        instr!(Opcode::StoreAddFpImm, 2, 0, 1), // store_add_fp_imm: [fp+1] = [fp+2] + 0 (a = b)
        instr!(Opcode::StoreAddFpFp, 3, 2, 2), // store_add_fp_fp: [fp+2] = [fp+3] + [fp+2] (b = temp + b)
        instr!(Opcode::StoreSubFpImm, 0, 1, 0), // store_sub_fp_imm: [fp+0] = [fp+0] - 1 (counter--)
        // Jump back to condition check
        instr!(Opcode::JmpAbsImm, 3, 0, 0), // jmp_abs_imm: jmp abs 3
    ];
    let instructions_len = instructions.len() as u32;
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    assert!(vm.execute(RunnerOptions::default().max_steps).is_ok());
    // Verify that FP is still at the end of the program
    // Verify PC reached the end of the program
    assert_vm_state!(vm.state, instructions_len, instructions_len);
    // Verify counter reached zero
    assert_eq!(vm.memory.get_data(vm.state.fp).unwrap(), M31::zero());

    // After n iterations, a = F(n) and b = F(n+1).
    // F(n) is at [fp+1].
    // F(n+1) is at [fp+2].
    assert_eq!(
        vm.memory.get_data(vm.state.fp + M31::one()).unwrap(),
        M31(fib(n))
    );
    assert_eq!(
        vm.memory.get_data(vm.state.fp + M31(2)).unwrap(),
        M31(fib(n + 1))
    );
}

#[test]
fn test_execute_fibonacci() {
    [0, 1, 2, 3, 10, 20].iter().for_each(|n| run_fib_test(*n));
}

#[test]
fn test_run_from_entrypoint_exponential_recursive_fibonacci() {
    [0, 1, 2, 3, 10, 20]
        .iter()
        .for_each(|n| run_exponential_recursive_fib_test(*n));
}

/// Runs a Fibonacci program on the VM and asserts the result against the reference implementation.
///
/// ```cairo-m
/// fn main() -> felt {
///   let n = 10;
///   let result = fib(n);
///   return result;
/// }
///
/// fn fib(n: felt) -> felt {
///   if n == 0 {
///     return 0;
///   }
///   if n == 1 {
///     return 1;
///   }
///   return fib(n - 1) + fib(n - 2);
/// }
/// ```
fn run_exponential_recursive_fib_test(n: u32) {
    let minus_4 = -M31(4);
    let minus_3 = -M31(3);
    let instructions = vec![
        // Setup call to fib(n)
        instr!(Opcode::StoreImm, n, 0, 0), // 0: store_imm: [fp] = n
        instr!(Opcode::CallAbsImm, 2, 4, 0), // 1: call_abs_imm: call fib(n)
        // Store the computed fib(n) and return.
        instr!(Opcode::StoreAddFpImm, 1, 0, minus_3), // 2: store_add_fp_imm: [fp - 3] = [fp + 1] + 0
        instr!(Opcode::Ret, 0, 0, 0),                 // 3: ret
        // fib(n: felt) function
        // Check if argument is 0
        instr!(Opcode::JnzFpImm, minus_4, 3, 0), // 4: jnz_fp_imm: jmp rel 3 if [fp - 4] != 0
        // Argument is 0, return 0
        instr!(Opcode::StoreImm, 0, 0, minus_3), // 5: store_imm: [fp - 3] = 0
        instr!(Opcode::Ret, 0, 0, 0),            // 6: ret
        // Check if argument is 1
        instr!(Opcode::StoreSubFpImm, minus_4, 1, 0), // 7: store_sub_fp_imm: [fp] = [fp - 4] - 1
        instr!(Opcode::JnzFpImm, 0, 3, 0),            // 8: jnz_fp_imm: jmp rel 3 if [fp] != 0
        // Argument is 1, return 1
        instr!(Opcode::StoreImm, 1, 0, minus_3), // 9: store_imm: [fp - 3] = 1
        instr!(Opcode::Ret, 0, 0, 0),            // 10: ret
        // Compute fib(n-1) + fib(n-2)
        // fib(n-1)
        // n - 1 is already stored at [fp], ready to be used as argument.
        instr!(Opcode::CallAbsImm, 2, 4, 0), // 11: call_abs_imm: call fib(n-1)
        instr!(Opcode::StoreAddFpImm, 1, 0, minus_3), // 12: store_add_fp_imm: [fp - 3] = [fp + 1] + 0
        // fib(n-2)
        instr!(Opcode::StoreSubFpImm, 0, 1, 0), // 13: Store n - 2, from previously computed n - 1 [fp] = [fp] - 1
        instr!(Opcode::CallAbsImm, 2, 4, 0),    // 1
        // Return value of fib(n-1) + fib(n-2)
        instr!(Opcode::StoreAddFpFp, minus_3, 1, minus_3), // 15: store_add_fp_fp: [fp - 3] = [fp - 3] + [fp + 1]
        instr!(Opcode::Ret, 0, 0, 0),                      // 16: ret
    ];
    let instructions_len = instructions.len() as u32;
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    let fp_offset = 3;
    vm.run_from_entrypoint(0, fp_offset, &[], 0, &RunnerOptions::default())
        .unwrap();
    // Verify that FP is still at the end of the program
    assert_eq!(vm.state.fp, M31(instructions_len + fp_offset));
    // Verify PC reached the end of the program
    assert_eq!(vm.state.pc, M31(instructions_len));

    // Result is stored at [fp - 3].
    assert_memory_value!(vm, addr = vm.state.fp - M31(3), value = fib(n));
}

#[test]
fn test_write_binary_trace_per_segment() {
    // Create a program that will be executed with segments
    let instructions = vec![
        instr!(Opcode::StoreImm, 10, 0, 0),    // store_imm: [fp + 0] = 10
        instr!(Opcode::StoreImm, 20, 0, 1),    // store_imm: [fp + 1] = 20
        instr!(Opcode::StoreAddFpFp, 0, 1, 2), // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Execute with segments - this will hit step limit and create segments
    let _ = vm.run_from_entrypoint(0, 3, &[], 0, &RunnerOptions { max_steps: 2 });

    // Create a temporary directory for the trace files
    let temp_dir = tempfile::tempdir().unwrap();
    let trace_path = temp_dir.path().join("trace.bin");

    // Write the trace files per segment
    let result = vm.write_binary_trace(&trace_path);
    assert!(result.is_ok());

    // Verify that segment files were created
    for i in 0..vm.segments.len() {
        let segment_file = temp_dir.path().join(format!("trace_segment_{}.bin", i));
        assert!(segment_file.exists(), "Segment {} file should exist", i);

        // Read and verify each segment file
        let mut file = File::open(&segment_file).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        // Should contain serialized trace data for the segment
        assert!(!contents.is_empty(), "Segment {} should have trace data", i);

        // Verify the data is a multiple of 8 bytes (2 u32 values per entry)
        assert_eq!(contents.len() % 8, 0, "Trace data should be aligned");
    }
}

#[test]
fn test_write_binary_memory_trace_per_segment() {
    // Create a program that will be executed with segments
    let instructions = vec![
        instr!(Opcode::StoreImm, 10, 0, 0),    // store_imm: [fp + 0] = 10
        instr!(Opcode::StoreImm, 20, 0, 1),    // store_imm: [fp + 1] = 20
        instr!(Opcode::StoreAddFpFp, 0, 1, 2), // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Execute with segments (limit steps to create multiple segments)
    let _ = vm.run_from_entrypoint(0, 3, &[], 0, &RunnerOptions { max_steps: 2 });

    // Create a temporary directory for the memory trace files
    let temp_dir = tempfile::tempdir().unwrap();
    let memory_trace_path = temp_dir.path().join("memory_trace.bin");

    // Write the memory trace files per segment
    let result = vm.write_binary_memory_trace(&memory_trace_path);
    assert!(result.is_ok());

    // Verify that segment files were created
    for i in 0..vm.segments.len() {
        let segment_file = temp_dir
            .path()
            .join(format!("memory_trace_segment_{}.bin", i));
        assert!(
            segment_file.exists(),
            "Memory segment {} file should exist",
            i
        );

        // Read and verify each segment file
        let mut file = File::open(&segment_file).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        // Should contain program length (4 bytes) + memory trace data
        assert!(
            contents.len() >= 4,
            "Segment {} should have at least program length",
            i
        );

        // Extract program length
        let mut program_length_bytes = [0u8; 4];
        program_length_bytes.copy_from_slice(&contents[0..4]);
        let program_length = u32::from_le_bytes(program_length_bytes);
        assert_eq!(program_length, vm.program_length.0);

        // Remaining data should be memory entries (20 bytes each)
        let memory_data_len = contents.len() - 4;
        assert_eq!(
            memory_data_len % 20,
            0,
            "Memory trace data should be aligned"
        );
    }
}
