use std::fs::File;
use std::io::Read;

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Program, State};
use num_traits::{One, Zero};
use stwo_prover::core::fields::m31::M31;

// Import test utilities
use super::test_utils::*;
use crate::memory::Memory;
use crate::vm::{VmError, VM};
use crate::RunnerOptions;

#[test]
fn test_program_from_vec_instructions() {
    let instructions = vec![
        Instruction::StoreAddFpImm {
            src_off: M31(2),
            imm: M31(3),
            dst_off: M31(4),
        },
        Instruction::StoreDoubleDerefFp {
            base_off: M31(6),
            offset: M31(7),
            dst_off: M31(8),
        },
    ];
    let program: Program = Program::from(instructions.clone());

    assert_eq!(program.instructions, instructions);
}

#[test]
fn test_vm_try_from() {
    // Create a simple program with two instructions
    let instructions = vec![
        Instruction::StoreAddFpImm {
            src_off: M31(2),
            imm: M31(3),
            dst_off: M31(4),
        },
        Instruction::StoreDoubleDerefFp {
            base_off: M31(6),
            offset: M31(7),
            dst_off: M31(8),
        },
    ];
    let program: Program = instructions.clone().into();

    let vm = VM::try_from(&program).unwrap();

    // Check that PC is set to 0 (entrypoint)
    // Check that FP is set right after the bytecode (2 instructions)
    assert_vm_state!(vm.state, 0, 2);

    // Check that the first instruction is in memory at address 0
    let loaded_smallvec = vm.memory.get_instruction(M31::zero()).unwrap();
    let loaded_instruction: Instruction = loaded_smallvec.try_into().unwrap();
    assert_eq!(loaded_instruction.opcode_value(), 4); // StoreAddFpImm
    assert_eq!(loaded_instruction, instructions[0]);

    // Check that the second instruction is in memory at address 1
    let loaded_smallvec_2 = vm.memory.get_instruction(M31::one()).unwrap();
    let loaded_instruction_2: Instruction = loaded_smallvec_2.try_into().unwrap();
    assert_eq!(loaded_instruction_2.opcode_value(), 8); // StoreDoubleDerefFp
    assert_eq!(loaded_instruction_2, instructions[1]);
}

#[test]
fn test_step_single_instruction() {
    // Create a program with a single store_imm instruction: [fp + 0] = 42
    let instructions = vec![Instruction::StoreImm {
        imm: M31(42),
        dst_off: M31(0),
    }];
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
    let bad_instr = M31::from(2_u32.pow(30));
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
    let instructions = vec![Instruction::StoreImm {
        imm: M31(42),
        dst_off: M31(0),
    }];
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
        Instruction::StoreImm {
            imm: M31(10),
            dst_off: M31(0),
        }, // [fp + 0] = 10
        Instruction::StoreImm {
            imm: M31(5),
            dst_off: M31(1),
        }, // [fp + 1] = 5
        Instruction::StoreAddFpFp {
            src0_off: M31(0),
            src1_off: M31(1),
            dst_off: M31(2),
        }, // [fp + 2] = [fp + 0] + [fp + 1]
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
        M31::from(9),
        M31::from(10), // Valid: [fp + 0] = 10
        M31::from(99), // Invalid: opcode 99
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
    let stored_value = vm.memory.get_felt(M31(2)).unwrap();
    assert_eq!(stored_value, M31(10));
}

#[test]
fn test_execute_arithmetic_operations() {
    // Test a program that performs various arithmetic operations
    let instructions = vec![
        Instruction::StoreImm {
            imm: M31(12),
            dst_off: M31(0),
        }, // [fp + 0] = 12
        Instruction::StoreImm {
            imm: M31(3),
            dst_off: M31(1),
        }, // [fp + 1] = 3
        Instruction::StoreMulFpFp {
            src0_off: M31(0),
            src1_off: M31(1),
            dst_off: M31(2),
        }, // [fp + 2] = [fp + 0] * [fp + 1] = 36
        Instruction::StoreDivFpFp {
            src0_off: M31(2),
            src1_off: M31(1),
            dst_off: M31(3),
        }, // [fp + 3] = [fp + 2] / [fp + 1] = 12
        Instruction::StoreSubFpFp {
            src0_off: M31(3),
            src1_off: M31(0),
            dst_off: M31(4),
        }, // [fp + 4] = [fp + 3] - [fp + 0] = 0
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
        Instruction::StoreImm {
            imm: M31(10),
            dst_off: M31(0),
        }, // [fp] = 10
        Instruction::StoreAddFpImm {
            src_off: M31(0),
            imm: M31(5),
            dst_off: M31(1),
        }, // [fp + 1] = [fp] + 5
    ];
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    // Initial FP is 2 in the default case, we add an offset of 2.
    // We run the program from PC = 1, so the first instruction should be ignored.
    vm.run_from_entrypoint(1, 2, &[], 0, &RunnerOptions::default())
        .unwrap();
    assert_vm_state!(vm.state, 2, 4);
    assert_eq!(
        vm.memory.get_felt(vm.state.fp + M31::one()).unwrap(),
        M31(5)
    );
}

#[test]
fn test_serialize_trace() {
    // Create a program with two instructions to generate a trace.
    let instructions = vec![
        Instruction::StoreImm {
            imm: M31(10),
            dst_off: M31(0),
        }, // [fp + 0] = 10
        Instruction::StoreImm {
            imm: M31(20),
            dst_off: M31(1),
        }, // [fp + 1] = 20
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
        Instruction::StoreImm {
            imm: M31(n),
            dst_off: M31(0),
        }, // store_imm: [fp+0] = counter
        Instruction::StoreImm {
            imm: M31(0),
            dst_off: M31(1),
        }, // store_imm: [fp+1] = a = F_0 = 0
        Instruction::StoreImm {
            imm: M31(1),
            dst_off: M31(2),
        }, // store_imm: [fp+2] = b = F_1 = 1
        // Loop condition check
        // while counter != 0 jump to loop body
        Instruction::JnzFpImm {
            cond_off: M31(0),
            offset: M31(2),
        }, // jnz_fp_imm: jmp rel 2 if [fp + 0] != 0  (pc=3 here, pc=5 in beginning of loop body)
        // Exit jump if counter was 0
        Instruction::JmpAbsImm { target: M31(10) }, // jmp_abs_imm: jmp abs 10
        // Loop body
        Instruction::StoreAddFpImm {
            src_off: M31(1),
            imm: M31(0),
            dst_off: M31(3),
        }, // store_add_fp_imm: [fp+3] = [fp+1] + 0 (tmp = a)
        Instruction::StoreAddFpImm {
            src_off: M31(2),
            imm: M31(0),
            dst_off: M31(1),
        }, // store_add_fp_imm: [fp+1] = [fp+2] + 0 (a = b)
        Instruction::StoreAddFpFp {
            src0_off: M31(3),
            src1_off: M31(2),
            dst_off: M31(2),
        }, // store_add_fp_fp: [fp+2] = [fp+3] + [fp+2] (b = temp + b)
        Instruction::StoreSubFpImm {
            src_off: M31(0),
            imm: M31(1),
            dst_off: M31(0),
        }, // store_sub_fp_imm: [fp+0] = [fp+0] - 1 (counter--)
        // Jump back to condition check
        Instruction::JmpAbsImm { target: M31(3) }, // jmp_abs_imm: jmp abs 3
    ];
    let instructions_len = instructions.len() as u32;
    let program = Program::from(instructions);
    let mut vm = VM::try_from(&program).unwrap();

    assert!(vm.execute(RunnerOptions::default().max_steps).is_ok());
    // Verify that FP is still at the end of the program
    // Verify PC reached the end of the program
    assert_vm_state!(vm.state, instructions_len, instructions_len);
    // Verify counter reached zero
    assert_eq!(vm.memory.get_felt(vm.state.fp).unwrap(), M31::zero());

    // After n iterations, a = F(n) and b = F(n+1).
    // F(n) is at [fp+1].
    // F(n+1) is at [fp+2].
    assert_eq!(
        vm.memory.get_felt(vm.state.fp + M31::one()).unwrap(),
        M31(fib(n))
    );
    assert_eq!(
        vm.memory.get_felt(vm.state.fp + M31(2)).unwrap(),
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
        Instruction::StoreImm {
            imm: M31(n),
            dst_off: M31(0),
        }, // 0: store_imm: [fp] = n
        Instruction::CallAbsImm {
            frame_off: M31(2),
            target: M31(4),
        }, // 1: call_abs_imm: call fib(n)
        // Store the computed fib(n) and return.
        Instruction::StoreAddFpImm {
            src_off: M31(1),
            imm: M31(0),
            dst_off: minus_3,
        }, // 2: store_add_fp_imm: [fp - 3] = [fp + 1] + 0
        Instruction::Ret {}, // 3: ret
        // fib(n: felt) function
        // Check if argument is 0
        Instruction::JnzFpImm {
            cond_off: minus_4,
            offset: M31(3),
        }, // 4: jnz_fp_imm: jmp rel 3 if [fp - 4] != 0
        // Argument is 0, return 0
        Instruction::StoreImm {
            imm: M31(0),
            dst_off: minus_3,
        }, // 5: store_imm: [fp - 3] = 0
        Instruction::Ret {}, // 6: ret
        // Check if argument is 1
        Instruction::StoreSubFpImm {
            src_off: minus_4,
            imm: M31(1),
            dst_off: M31(0),
        }, // 7: store_sub_fp_imm: [fp] = [fp - 4] - 1
        Instruction::JnzFpImm {
            cond_off: M31(0),
            offset: M31(3),
        }, // 8: jnz_fp_imm: jmp rel 3 if [fp] != 0
        // Argument is 1, return 1
        Instruction::StoreImm {
            imm: M31(1),
            dst_off: minus_3,
        }, // 9: store_imm: [fp - 3] = 1
        Instruction::Ret {}, // 10: ret
        // Compute fib(n-1) + fib(n-2)
        // fib(n-1)
        // n - 1 is already stored at [fp], ready to be used as argument.
        Instruction::CallAbsImm {
            frame_off: M31(2),
            target: M31(4),
        }, // 11: call_abs_imm: call fib(n-1)
        Instruction::StoreAddFpImm {
            src_off: M31(1),
            imm: M31(0),
            dst_off: minus_3,
        }, // 12: store_add_fp_imm: [fp - 3] = [fp + 1] + 0
        // fib(n-2)
        Instruction::StoreSubFpImm {
            src_off: M31(0),
            imm: M31(1),
            dst_off: M31(0),
        }, // 13: Store n - 2, from previously computed n - 1 [fp] = [fp] - 1
        Instruction::CallAbsImm {
            frame_off: M31(2),
            target: M31(4),
        }, // 14: call_abs_imm: call fib(n-2)
        // Return value of fib(n-1) + fib(n-2)
        Instruction::StoreAddFpFp {
            src0_off: minus_3,
            src1_off: M31(1),
            dst_off: minus_3,
        }, // 15: store_add_fp_fp: [fp - 3] = [fp - 3] + [fp + 1]
        Instruction::Ret {}, // 16: ret
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
        Instruction::StoreImm {
            imm: M31(10),
            dst_off: M31(0),
        }, // store_imm: [fp + 0] = 10
        Instruction::StoreImm {
            imm: M31(20),
            dst_off: M31(1),
        }, // store_imm: [fp + 1] = 20
        Instruction::StoreAddFpFp {
            src0_off: M31(0),
            src1_off: M31(1),
            dst_off: M31(2),
        }, // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
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
        Instruction::StoreImm {
            imm: M31(10),
            dst_off: M31(0),
        }, // store_imm: [fp + 0] = 10
        Instruction::StoreImm {
            imm: M31(20),
            dst_off: M31(1),
        }, // store_imm: [fp + 1] = 20
        Instruction::StoreAddFpFp {
            src0_off: M31(0),
            src1_off: M31(1),
            dst_off: M31(2),
        }, // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
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

#[test]
fn test_5_m31_u32_instruction_execution_and_pc_advancement() {
    use cairo_m_common::Instruction;

    // Create a U32StoreAddFpImm instruction (5 M31 elements)
    // This will add [fp + src_off] + (imm_hi << 16 | imm_lo) and store to [fp + dst_off]
    let u32_instruction = Instruction::U32StoreAddFpImm {
        src_off: M31(0), // Source offset: fp + 0
        imm_hi: M31(1),  // High 16 bits of immediate = 1
        imm_lo: M31(2),  // Low 16 bits of immediate = 2
        dst_off: M31(1), // Destination offset: fp + 1
    };

    // Create a simple program with the 5-M31 instruction followed by a simple instruction
    let instructions = vec![
        u32_instruction,
        Instruction::StoreImm {
            imm: M31(100),
            dst_off: M31(3),
        }, // Simple 3-M31 instruction for PC verification
    ];
    let program = Program::from(instructions);

    let mut vm = VM::try_from(&program).unwrap();

    // Set up initial memory: store a 32-bit value as two limbs at fp + 0
    let initial_fp = vm.state.fp;
    // Store value 0x0000000A (10) as [0x000A, 0x0000]
    vm.memory.insert_no_trace(initial_fp, M31(10)).unwrap();
    vm.memory
        .insert_no_trace(initial_fp + M31(1), M31(0))
        .unwrap();

    // Execute one step (the U32 instruction)
    let initial_pc = vm.state.pc;
    vm.step().unwrap();

    // Verify that PC advanced by the correct amount (2 QM31s for a 5-M31 instruction)
    let expected_pc = initial_pc + M31(2); // 5 M31s = 2 QM31s (rounded up)
    assert_eq!(
        vm.state.pc, expected_pc,
        "PC should advance by 2 QM31s for 5-M31 instruction"
    );

    // Verify the computation: 0x0000000A + 0x00010002 = 0x0001000C
    // Result should be stored as [0x000C, 0x0001]
    let result_lo = vm.memory.get_felt(initial_fp + M31(1)).unwrap();
    let result_hi = vm.memory.get_felt(initial_fp + M31(2)).unwrap();
    assert_eq!(
        result_lo,
        M31(12),
        "U32 instruction should compute correct low limb"
    );
    assert_eq!(
        result_hi,
        M31(1),
        "U32 instruction should compute correct high limb"
    );

    // Execute one more step (the simple instruction)
    vm.step().unwrap();

    // Verify PC advanced by 1 QM31 for the 3-M31 instruction
    let final_expected_pc = expected_pc + M31(1);
    assert_eq!(
        vm.state.pc, final_expected_pc,
        "PC should advance by 1 QM31 for 3-M31 instruction"
    );

    // Verify the simple instruction executed correctly
    let simple_result = vm.memory.get_felt(initial_fp + M31(3)).unwrap();
    assert_eq!(
        simple_result,
        M31(100),
        "Simple instruction should store correct value"
    );
}
