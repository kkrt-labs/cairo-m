pub mod instructions;
pub mod state;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Program};
use instructions::opcode_to_instruction_fn;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::State;

/// Custom error type for VM operations.
#[derive(Debug, Error)]
pub enum VmError {
    #[error("VM memory error: {0}")]
    Memory(#[from] MemoryError),
    #[error("VM instruction error: {0}")]
    Instruction(#[from] InstructionError),
    #[error("VM I/O error: {0}")]
    Io(#[from] io::Error),
}

/// A single entry in the trace.
#[derive(Debug, PartialEq, Eq)]
pub struct TraceEntry {
    pub pc: M31,
    pub fp: M31,
}

/// The Cairo M Virtual Machine.
///
/// ## Fields
///
/// - `memory`: Flat address space storing instructions and data
/// - `state`: Current processor state (PC, FP)
/// - `program_length`: Length of the program in instructions
/// - `trace`: Execution trace
#[derive(Debug, Default)]
pub struct VM {
    pub final_pc: M31,
    pub memory: Memory,
    pub state: State,
    pub program_length: M31,
    pub trace: Vec<TraceEntry>,
}

impl TryFrom<&Program> for VM {
    type Error = VmError;

    /// Creates a VM instance from a given [`Program`].
    ///
    /// This method initializes the VM state for execution:
    /// 1. It loads all program instructions into memory starting at address `0`.
    /// 2. It sets the Program Counter (`pc`) to `0` to begin at the program's entrypoint.
    /// 3. It sets the Frame Pointer (`fp`) to the address immediately following the loaded bytecode.
    ///
    /// Instructions are encoded as [`QM31`] values and stored sequentially in memory.
    ///
    /// ## Arguments
    ///
    /// * `program` - The [`Program`] to load.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Memory`] if memory insertion fails.
    fn try_from(program: &Program) -> Result<Self, Self::Error> {
        // Convert all instructions to QM31 values
        let qm31_instructions: Vec<QM31> = program
            .instructions
            .iter()
            .map(|instruction| instruction.into())
            .collect();

        // Create memory and load instructions starting at address 0
        let program_length = M31(qm31_instructions.len() as u32);
        let final_pc = program_length;
        let memory = Memory::from_iter(qm31_instructions);

        // Create state with PC at entrypoint and FP just after the bytecode
        let state = State {
            pc: M31::zero(),
            fp: final_pc,
        };

        Ok(Self {
            final_pc,
            memory,
            state,
            program_length,
            trace: vec![],
        })
    }
}

impl VM {
    /// Executes a single instruction at the current program counter (PC).
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError`] if:
    /// - The opcode is invalid ([`VmError::Instruction`])
    /// - The instruction execution fails due to memory operations ([`VmError::Memory`])
    fn step(&mut self) -> Result<(), VmError> {
        let instruction: Instruction = self.memory.get_instruction(self.state.pc)?.try_into()?;
        let instruction_fn = opcode_to_instruction_fn(M31::from(instruction.opcode))?;
        self.trace.push(TraceEntry {
            pc: self.state.pc,
            fp: self.state.fp,
        });
        self.state = instruction_fn(&mut self.memory, self.state, &instruction)?;
        Ok(())
    }

    /// Executes the loaded program from start to completion.
    ///
    /// This method runs the VM by repeatedly calling [`step()`](Self::step) until the program
    /// counter reaches the end of the loaded instructions. It assumes that the program is loaded
    /// at the beginning of the memory ([0..instructions_length]).
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError`] if any instruction execution fails:
    /// - Invalid opcodes ([`VmError::Instruction`])
    /// - Memory errors ([`VmError::Memory`])
    fn execute(&mut self) -> Result<(), VmError> {
        if self.final_pc.is_zero() {
            return Ok(());
        }

        while self.state.pc != self.final_pc {
            self.step()?;
        }

        Ok(())
    }

    /// Executes the loaded program from a given entrypoint and frame pointer.
    ///
    /// - The PC entrypoint is the first instruction of the function to execute in the program.
    /// - The FP offset accounts for the calling convention of the executed function: arguments, return values, return address.
    ///
    /// The call stack of the entrypoint is initialized here.
    ///
    /// ## Arguments
    ///
    /// * `pc_entrypoint` - The program counter (PC) to start execution from.
    /// * `fp_offset` - The frame pointer (FP) offset to start execution from.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError`] if any instruction execution fails:
    /// - Invalid opcodes ([`VmError::Instruction`])
    /// - Memory errors ([`VmError::Memory`])
    pub fn run_from_entrypoint(
        &mut self,
        pc_entrypoint: u32,
        fp_offset: u32,
    ) -> Result<(), VmError> {
        self.state.pc = M31(pc_entrypoint);
        self.state.fp += M31(fp_offset);

        self.memory
            .insert_entrypoint_call(&self.final_pc, &self.state.fp)?;

        self.execute()
    }

    /// Serializes the trace to a byte vector.
    ///
    /// Each trace entry consists of `fp` and `pc` values, both `u32`.
    /// This function serializes the entire trace as a flat sequence of bytes.
    /// For each entry, it first serializes `fp` into little-endian bytes,
    /// followed by the little-endian bytes of `pc`.
    ///
    /// The final output is a single `Vec<u8>` concatenating the bytes for all entries.
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized trace data.
    pub fn serialize_trace(&self) -> Vec<u8> {
        self.trace
            .iter()
            .flat_map(|entry| [entry.fp.0, entry.pc.0])
            .flat_map(u32::to_le_bytes)
            .collect()
    }

    /// Writes the serialized trace to a binary file.
    ///
    /// This function serializes the trace using [`serialize_trace()`](Self::serialize_trace)
    /// and writes the resulting bytes to the specified file path.
    ///
    /// ## Arguments
    ///
    /// * `path` - The file path where the binary trace will be written.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Io`] if:
    /// - The file cannot be created or opened for writing
    /// - Writing to the file fails
    pub fn write_binary_trace<P: AsRef<Path>>(&self, path: P) -> Result<(), VmError> {
        let serialized_trace = self.serialize_trace();
        let mut file = File::create(path)?;
        file.write_all(&serialized_trace)?;
        Ok(())
    }

    /// Writes the serialized memory trace to a binary file.
    ///
    /// This function first writes the program length to the file, then serializes
    /// the memory trace using the memory's [`serialize_trace()`](Memory::serialize_trace)
    /// method and writes the resulting bytes to the specified file path.
    ///
    /// Each memory trace entry consists of an address (`M31`) and a value (`QM31`).
    /// The serialization format includes the program length first, followed by the address
    /// and the 4 components of the `QM31` value for each entry, all in little-endian byte order.
    ///
    /// ## Arguments
    ///
    /// * `path` - The file path where the binary memory trace will be written.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Io`] if:
    /// - The file cannot be created or opened for writing
    /// - Writing to the file fails
    pub fn write_binary_memory_trace<P: AsRef<Path>>(&self, path: P) -> Result<(), VmError> {
        let mut file = File::create(path)?;
        let serialized_memory_trace = self.memory.serialize_trace();
        file.write_all(&self.program_length.0.to_le_bytes())?;
        file.write_all(&serialized_memory_trace)?;
        Ok(())
    }
}

// #[cfg(test)]
// #[path = "./vm_tests.rs"]
// mod vm_tests;

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;

    use cairo_m_common::instruction::InstructionError;
    use cairo_m_common::{Instruction, Opcode, Program};
    use num_traits::{One, Zero};
    use stwo_prover::core::fields::m31::M31;
    use stwo_prover::core::fields::qm31::QM31;
    use tempfile::NamedTempFile;

    // Import test utilities
    use super::test_utils::*;
    use crate::memory::Memory;
    use crate::vm::state::State;
    use crate::vm::{TraceEntry, VmError, VM};

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
        let result = vm.execute();
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

        let result = vm.execute();
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

        let result = vm.execute();
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
            QM31::from_m31_array([M31::from(6), M31::from(10), Zero::zero(), Zero::zero()]), // Valid: [fp + 0] = 10
            QM31::from_m31_array([M31::from(99), Zero::zero(), Zero::zero(), Zero::zero()]), // Invalid: opcode 99
        ];
        let mut vm = VM {
            final_pc: M31::from(instructions.len() as u32),
            memory: Memory::from_iter(instructions),
            state: State {
                pc: M31::zero(),
                fp: M31::from(instructions.len() as u32),
            },
            program_length: M31::from(instructions.len() as u32),
            trace: vec![],
        };
        // Execute should fail when it hits the invalid instruction
        let result = vm.execute();
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

        let result = vm.execute();
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
        vm.run_from_entrypoint(1, 2).unwrap();
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
        assert!(vm.execute().is_ok());

        // The trace should have 2 entries, one for each instruction executed.
        assert_eq!(vm.trace.len(), 2);

        // Verify the trace contents.
        assert_eq!(
            vm.trace[0],
            TraceEntry {
                pc: M31::zero(),
                fp: M31(2)
            }
        );
        assert_eq!(
            vm.trace[1],
            TraceEntry {
                pc: M31::one(),
                fp: M31(2)
            }
        );

        // Serialize the trace and verify its contents.
        let serialized_trace = vm.serialize_trace();
        // Expected serialized data:
        // Entry 1: fp=2, pc=0.
        // Entry 2: fp=2, pc=1.
        let expected_bytes = Vec::from([2, 0, 2, 1].map(u32::to_le_bytes).as_flattened());

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
            instr!(Opcode::StoreDerefFp, 1, 0, 3), // store_deref_fp: [fp+3] = [fp+1] (tmp = a)
            instr!(Opcode::StoreDerefFp, 2, 0, 1), // store_deref_fp: [fp+1] = [fp+2] (a = b)
            instr!(Opcode::StoreAddFpFp, 3, 2, 2), // store_add_fp_fp: [fp+2] = [fp+3] + [fp+2] (b = temp + b)
            instr!(Opcode::StoreSubFpImm, 0, 1, 0), // store_sub_fp_imm: [fp+0] = [fp+0] - 1 (counter--)
            // Jump back to condition check
            instr!(Opcode::JmpAbsImm, 3, 0, 0), // jmp_abs_imm: jmp abs 3
        ];
        let instructions_len = instructions.len() as u32;
        let program = Program::from(instructions);
        let mut vm = VM::try_from(&program).unwrap();

        assert!(vm.execute().is_ok());
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
    /// func main() -> felt {
    ///   let n = 10;
    ///   let result = fib(n);
    ///   return result;
    /// }
    ///
    /// func fib(n: felt) -> felt {
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
            instr!(Opcode::StoreDerefFp, 1, 0, minus_3), // 2: store_deref_fp: [fp - 3] = [fp + 1]
            instr!(Opcode::Ret, 0, 0, 0),                // 3: ret
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
            instr!(Opcode::StoreDerefFp, 1, 0, minus_3), // 12: store_deref_fp: [fp - 3] = [fp + 1]
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
        vm.run_from_entrypoint(0, fp_offset).unwrap();
        // Verify that FP is still at the end of the program
        assert_eq!(vm.state.fp, M31(instructions_len + fp_offset));
        // Verify PC reached the end of the program
        assert_eq!(vm.state.pc, M31(instructions_len));

        // Result is stored at [fp - 3].
        assert_memory_value!(vm, addr = vm.state.fp - M31(3), value = fib(n));
    }

    #[test]
    fn test_write_binary_trace() {
        // Create a program with two instructions to generate a trace.
        let instructions = vec![
            Instruction::new(
                Opcode::StoreImm,
                [M31::from(10), Zero::zero(), Zero::zero()],
            ), // [fp + 0] = 10
            Instruction::new(Opcode::StoreImm, [M31::from(20), Zero::zero(), One::one()]), // [fp + 1] = 20
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(&program).unwrap();

        // Execute the program to generate a trace.
        assert!(vm.execute().is_ok());

        // Create a temporary file for the trace.
        let temp_file = NamedTempFile::new().unwrap();
        let temp_file_path = temp_file.path();

        // Write the trace to the temporary file.
        let result = vm.write_binary_trace(temp_file_path);
        assert!(result.is_ok());

        // Read the file back and verify its contents.
        let mut file = File::open(temp_file_path).unwrap();
        let mut file_contents = Vec::new();
        file.read_to_end(&mut file_contents).unwrap();

        // Compare with the expected serialized trace.
        assert_eq!(file_contents, vm.serialize_trace());

        // Expected serialized data:
        // Entry 1: fp=2, pc=0.
        // Entry 2: fp=2, pc=1.
        let expected_bytes = Vec::from([2, 0, 2, 1].map(u32::to_le_bytes).as_flattened());

        assert_eq!(file_contents, expected_bytes);
    }

    #[test]
    fn test_write_binary_memory_trace() {
        // Create a program with instructions that access memory to generate a memory trace.
        let instructions = vec![
            instr!(Opcode::StoreImm, 10, 0, 0),    // store_imm: [fp + 0] = 10
            instr!(Opcode::StoreImm, 20, 0, 1),    // store_imm: [fp + 1] = 20
            instr!(Opcode::StoreAddFpFp, 0, 1, 2), // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(&program).unwrap();

        // Execute the program to generate memory accesses.
        assert!(vm.execute().is_ok());

        // Create a temporary file for the memory trace.
        let temp_file = NamedTempFile::new().unwrap();
        let temp_file_path = temp_file.path();

        // Write the memory trace to the temporary file.
        let result = vm.write_binary_memory_trace(temp_file_path);
        assert!(result.is_ok());

        // Read the file back and verify its contents.
        let mut file = File::open(temp_file_path).unwrap();
        let mut file_contents = Vec::new();
        file.read_to_end(&mut file_contents).unwrap();

        // Compare with the expected serialized memory trace.
        let mut expected_contents = Vec::new();
        expected_contents.extend_from_slice(&vm.program_length.0.to_le_bytes());
        expected_contents.extend_from_slice(&vm.memory.serialize_trace());
        assert_eq!(file_contents, expected_contents);

        // Verify that the memory trace is not empty (we should have memory accesses).
        assert!(!file_contents.is_empty());

        // The memory trace should contain entries for:
        // 1. Instruction fetches (3 instructions)
        // 2. Memory stores (3 store operations)
        // 3. Memory loads (2 loads for the addition operation)
        // Each entry is 5 * 4 = 20 bytes (addr + 4 QM31 components)
        let expected_entries = 3 + 3 + 2; // instruction fetches + stores + loads
        let expected_size = expected_entries * 5 * 4 + 4; // 5 u32 values * 4 bytes each + 4 bytes for program length
        assert_eq!(file_contents.len(), expected_size);
    }
}

#[cfg(test)]
pub mod test_utils {
    // Helper macros for common patterns in tests
    macro_rules! instr {
        ($opcode:expr, $a:expr, $b:expr, $c:expr) => {
            cairo_m_common::Instruction::new(
                $opcode,
                [
                    stwo_prover::core::fields::m31::M31::from($a),
                    stwo_prover::core::fields::m31::M31::from($b),
                    stwo_prover::core::fields::m31::M31::from($c),
                ],
            )
        };
    }

    macro_rules! store_imm {
        ($val:expr, $offset:expr) => {
            instr!(cairo_m_common::Opcode::StoreImm, $val, 0, $offset)
        };
    }

    macro_rules! assert_memory_value {
        ($vm:expr, addr = $addr:expr, value = $val:expr) => {
            assert_eq!(
                $vm.memory
                    .get_data(stwo_prover::core::fields::m31::M31::from($addr))
                    .unwrap(),
                stwo_prover::core::fields::m31::M31::from($val)
            );
        };
    }

    macro_rules! assert_vm_state {
        ($state:expr, $pc:expr, $fp:expr) => {
            assert_eq!($state.pc, stwo_prover::core::fields::m31::M31::from($pc));
            assert_eq!($state.fp, stwo_prover::core::fields::m31::M31::from($fp));
        };
    }

    // Export macros
    pub(crate) use {assert_memory_value, assert_vm_state, instr, store_imm};
}
