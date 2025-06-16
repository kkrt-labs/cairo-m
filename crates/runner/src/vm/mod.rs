pub mod instructions;
pub mod state;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::State;
use instructions::{opcode_to_instruction_fn, Instruction, InstructionError};
use num_traits::Zero;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

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

/// A compiled Cairo M program containing decoded instructions.
#[derive(Debug)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

impl From<Vec<Instruction>> for Program {
    fn from(instructions: Vec<Instruction>) -> Self {
        Self { instructions }
    }
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
#[derive(Debug, Default)]
pub struct VM {
    pub memory: Memory,
    pub state: State,
    pub trace: Vec<TraceEntry>,
}

impl TryFrom<Program> for VM {
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
    fn try_from(program: Program) -> Result<Self, Self::Error> {
        // Convert all instructions to QM31 values
        let qm31_instructions: Vec<QM31> = program
            .instructions
            .iter()
            .map(|instruction| {
                QM31::from_m31_array([
                    instruction.op,
                    instruction.args[0],
                    instruction.args[1],
                    instruction.args[2],
                ])
            })
            .collect();

        // Create memory and load instructions starting at address 0
        let instructions_len = qm31_instructions.len() as u32;
        let memory = Memory::from_iter(qm31_instructions);

        // Create state with PC at entrypoint and FP just after the bytecode
        let state = State {
            pc: M31::zero(),
            fp: M31(instructions_len),
        };

        Ok(Self {
            memory,
            state,
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
        let instruction = Instruction::from(self.memory.get_instruction(self.state.pc)?);
        let instruction_fn = opcode_to_instruction_fn(instruction.op)?;
        self.trace.push(TraceEntry {
            pc: self.state.pc,
            fp: self.state.fp,
        });
        self.state = instruction_fn(&mut self.memory, self.state, instruction)?;
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
        let instructions_len = self.memory.data.len();
        if instructions_len == 0 {
            return Ok(());
        }

        let final_pc = M31::from(instructions_len);
        while self.state.pc != final_pc {
            self.step()?;
        }

        Ok(())
    }

    /// Executes the loaded program from a given entrypoint and frame pointer.
    ///
    /// The PC entrypoint is the first instruction of the function to execute in the program.
    ///
    /// The FP offset accounts for the calling convention of the executed function: arguments, return values, return address.
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
    /// This function serializes the memory trace using the memory's [`serialize_trace()`](Memory::serialize_trace)
    /// method and writes the resulting bytes to the specified file path.
    ///
    /// Each memory trace entry consists of an address (`M31`) and a value (`QM31`).
    /// The serialization format includes the address followed by the 4 components of the `QM31` value,
    /// all in little-endian byte order.
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
        let serialized_memory_trace = self.memory.serialize_trace();
        let mut file = File::create(path)?;
        file.write_all(&serialized_memory_trace)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::vm::{
        instructions::InstructionError, Instruction, Program, TraceEntry, VmError, VM,
    };
    use num_traits::{One, Zero};
    use std::fs::File;
    use std::io::Read;
    use stwo_prover::core::fields::m31::M31;
    use tempfile::NamedTempFile;

    #[test]
    fn test_program_from_vec_instructions() {
        let instructions = vec![
            Instruction::from([1, 2, 3, 4]),
            Instruction::from([5, 6, 7, 8]),
        ];
        let program: Program = Program::from(instructions.clone());

        assert_eq!(program.instructions, instructions);
    }

    #[test]
    fn test_vm_try_from() {
        // Create a simple program with two instructions
        let instructions = vec![
            Instruction::from([1, 2, 3, 4]),
            Instruction::from([5, 6, 7, 8]),
        ];
        let program: Program = instructions.clone().into();

        let vm = VM::try_from(program).unwrap();

        // Check that PC is set to 0 (entrypoint)
        assert_eq!(vm.state.pc, M31::zero());

        // Check that FP is set right after the bytecode (2 instructions)
        assert_eq!(vm.state.fp, M31(2));

        // Check that the first instruction is in memory at address 0
        let loaded_instruction_qm31 = vm.memory.get_instruction(M31::zero()).unwrap();
        let loaded_instruction = Instruction::from(loaded_instruction_qm31);
        assert_eq!(loaded_instruction.op, M31::one());
        assert_eq!(loaded_instruction, instructions[0]);

        // Check that the second instruction is in memory at address 1
        let loaded_instruction_qm31_2 = vm.memory.get_instruction(M31::one()).unwrap();
        let loaded_instruction_2 = Instruction::from(loaded_instruction_qm31_2);
        assert_eq!(loaded_instruction_2.op, M31(5));
        assert_eq!(loaded_instruction_2, instructions[1]);
    }

    #[test]
    fn test_step_single_instruction() {
        // Create a program with a single store_imm instruction: [fp + 0] = 42
        let instructions = vec![Instruction::from([6, 42, 0, 0])]; // opcode 6 = store_imm
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        // Initial state should have PC = 0, FP = 1
        assert_eq!(vm.state.pc, M31::zero());
        assert_eq!(vm.state.fp, M31::one());

        // Execute one step
        let result = vm.step();
        assert!(result.is_ok());

        // PC should have advanced to 1, FP should be the same
        assert_eq!(vm.state.pc, M31::one());
        assert_eq!(vm.state.fp, M31::one());

        // The value 42 should be stored at memory[fp + 0] = memory[1]
        let stored_value = vm.memory.get_data(M31::one()).unwrap();
        assert_eq!(stored_value, M31(42));
    }

    #[test]
    fn test_step_invalid_instruction() {
        // Create a program with an invalid opcode
        let instructions = vec![Instruction::from([2_u32.pow(30), 0, 0, 0])];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

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
        let mut vm = VM::try_from(program).unwrap();
        let result = vm.execute();
        assert!(result.is_ok());
        assert_eq!(vm.state.pc, M31::zero());
        assert_eq!(vm.state.fp, M31::zero());
        assert_eq!(vm.memory.data.len(), 0);
    }

    #[test]
    fn test_execute_single_instruction() {
        // Create a program with a single store_imm instruction
        let instructions = vec![Instruction::from([6, 42, 0, 0])]; // [fp + 0] = 42
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        let result = vm.execute();
        assert!(result.is_ok());

        // PC should be at final position (memory.len() = 1)
        assert_eq!(vm.state.pc, M31::one());
        assert_eq!(vm.state.fp, M31::one());

        // The value should be stored correctly at fp + 0 = 1 + 0 = 1
        let stored_value = vm.memory.get_data(M31::one()).unwrap();
        assert_eq!(stored_value, M31(42));
    }

    #[test]
    fn test_execute_multiple_instructions() {
        // Create a program with multiple instructions:
        // 1. [fp + 0] = 10 (store_imm)
        // 2. [fp + 1] = 5 (store_imm)
        // 3. [fp + 2] = [fp + 0] + [fp + 1] (store_add_fp_fp)
        let instructions = vec![
            Instruction::from([6, 10, 0, 0]), // [fp + 0] = 10
            Instruction::from([6, 5, 0, 1]),  // [fp + 1] = 5
            Instruction::from([0, 0, 1, 2]),  // [fp + 2] = [fp + 0] + [fp + 1]
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        // Initial state
        assert_eq!(vm.state.pc, M31::zero());
        assert_eq!(vm.state.fp, M31(3)); // FP should be after 3 instructions

        let result = vm.execute();
        assert!(result.is_ok());

        // PC should be at final position (memory.len() = 3)
        assert_eq!(vm.state.pc, M31(3));
        assert_eq!(vm.state.fp, M31(3));

        // Check the computed values
        let val1 = vm.memory.get_data(M31(3)).unwrap(); // [fp + 0] = 10
        let val2 = vm.memory.get_data(M31(4)).unwrap(); // [fp + 1] = 5
        let sum = vm.memory.get_data(M31(5)).unwrap(); // [fp + 2] = 15

        assert_eq!(val1, M31(10));
        assert_eq!(val2, M31(5));
        assert_eq!(sum, M31(15));
    }

    #[test]
    fn test_execute_with_error() {
        // Create a program with an invalid instruction
        let instructions = vec![
            Instruction::from([6, 10, 0, 0]), // Valid: [fp + 0] = 10
            Instruction::from([99, 0, 0, 0]), // Invalid opcode
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        // Execute should fail when it hits the invalid instruction
        let result = vm.execute();
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            VmError::Instruction(InstructionError::InvalidOpcode(M31(99)))
        ));

        // PC should be at 1 (where it failed)
        assert_eq!(vm.state.pc, M31::one());

        // First instruction should have executed successfully
        let stored_value = vm.memory.get_data(M31(2)).unwrap();
        assert_eq!(stored_value, M31(10));
    }

    #[test]
    fn test_execute_arithmetic_operations() {
        // Test a program that performs various arithmetic operations
        let instructions = vec![
            Instruction::from([6, 12, 0, 0]), // [fp + 0] = 12
            Instruction::from([6, 3, 0, 1]),  // [fp + 1] = 3
            Instruction::from([7, 0, 1, 2]),  // [fp + 2] = [fp + 0] * [fp + 1] = 36
            Instruction::from([9, 2, 1, 3]),  // [fp + 3] = [fp + 2] / [fp + 1] = 12
            Instruction::from([2, 3, 0, 4]),  // [fp + 4] = [fp + 3] - [fp + 0] = 0
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        let result = vm.execute();
        assert!(result.is_ok());

        // Check all computed values
        assert_eq!(vm.memory.get_data(M31(5)).unwrap(), M31(12)); // original 12
        assert_eq!(vm.memory.get_data(M31(6)).unwrap(), M31(3)); // original 3
        assert_eq!(vm.memory.get_data(M31(7)).unwrap(), M31(36)); // 12 * 3
        assert_eq!(vm.memory.get_data(M31(8)).unwrap(), M31(12)); // 36 / 3
        assert_eq!(vm.memory.get_data(M31(9)).unwrap(), M31(0)); // 12 - 12
    }

    #[test]
    fn test_run_from_entrypoint() {
        let instructions = vec![
            Instruction::from([6, 10, 0, 0]), // [fp] = 10
            Instruction::from([1, 0, 5, 1]),  // [fp + 1] = [fp] + 5
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        // Initial FP should 2 in the default case, we add an offset of 1.
        // We run the program from PC = 1, so the first instruction should be ignored.
        vm.run_from_entrypoint(1, 1).unwrap();
        assert_eq!(vm.state.pc, M31(2));
        assert_eq!(vm.state.fp, M31(3));
        assert_eq!(
            vm.memory.get_data(vm.state.fp + M31::one()).unwrap(),
            M31(5)
        );
    }

    #[test]
    fn test_serialize_trace() {
        // Create a program with two instructions to generate a trace.
        let instructions = vec![
            Instruction::from([6, 10, 0, 0]), // [fp + 0] = 10
            Instruction::from([6, 20, 0, 1]), // [fp + 1] = 20
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

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
            Instruction::from([6, n, 0, 0]), // store_imm: [fp+0] = counter
            Instruction::from([6, 0, 0, 1]), // store_imm: [fp+1] = a = F_0 = 0
            Instruction::from([6, 1, 0, 2]), // store_imm: [fp+2] = b = F_1 = 1
            // Loop condition check
            // while counter != 0 jump to loop body
            Instruction::from([31, 0, 2, 0]), // jnz_fp_imm: jmp rel 2 if [fp + 0] != 0  (pc=3 here, pc=5 in beginning of loop body)
            // Exit jump if counter was 0
            Instruction::from([20, 10, 0, 0]), // jmp_abs_imm: jmp abs 10
            // Loop body
            Instruction::from([4, 1, 0, 3]), // store_deref_fp: [fp+3] = [fp+1] (tmp = a)
            Instruction::from([4, 2, 0, 1]), // store_deref_fp: [fp+1] = [fp+2] (a = b)
            Instruction::from([0, 3, 2, 2]), // store_add_fp_fp: [fp+2] = [fp+3] + [fp+2] (b = temp + b)
            Instruction::from([3, 0, 1, 0]), // store_sub_fp_imm: [fp+0] = [fp+0] - 1 (counter--)
            // Jump back to condition check
            Instruction::from([20, 3, 0, 0]), // jmp_abs_imm: jmp abs 3
        ];
        let instructions_len = instructions.len() as u32;
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        assert!(vm.execute().is_ok());
        // Verify that FP is still at the end of the program
        assert_eq!(vm.state.fp, M31(instructions_len));
        // Verify PC reached the end of the program
        assert_eq!(vm.state.pc, M31(instructions_len));
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
    fn test_run_from_entrypoint_recursive_fibonacci() {
        [0, 1, 2, 3, 10, 20]
            .iter()
            .for_each(|n| run_recursive_fib_test(*n));
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
    fn run_recursive_fib_test(n: u32) {
        let minus_4 = -M31(4);
        let minus_3 = -M31(3);
        let minus_2 = -M31(2);
        let minus_1 = -M31::one();
        let instructions = vec![
            // main() setup
            Instruction::from([6, 22, 0, minus_2.0]), // 0: store_imm: [fp - 2] = END_OF_PROGRAM + FP_OFFSET
            Instruction::from([6, 19, 0, minus_1.0]), // 1: store_imm: [fp - 1] = END_OF_PROGRAM
            // Setup call to fib(n)
            Instruction::from([6, n, 0, 0]), // 2: store_imm: [fp] = n
            Instruction::from([12, 2, 6, 0]), // 3: call_abs_imm: call fib(n)
            // Store the computed fib(n) and return.
            Instruction::from([4, 1, 0, minus_3.0]), // 4: store_deref_fp: [fp - 3] = [fp + 1]
            Instruction::from([15, 0, 0, 0]),        // 5: ret
            // fib(n: felt) function
            // Check if argument is 0
            Instruction::from([31, minus_4.0, 3, 0]), // 6: jnz_fp_imm: jmp rel 3 if [fp - 4] != 0
            // Argument is 0, return 0
            Instruction::from([6, 0, 0, minus_3.0]), // 7: store_imm: [fp - 3] = 0
            Instruction::from([15, 0, 0, 0]),        // 8: ret
            // Check if argument is 1
            Instruction::from([3, minus_4.0, 1, 0]), //9: store_sub_fp_imm: [fp] = [fp - 4] - 1
            Instruction::from([31, 0, 3, 0]),        // 10: jnz_fp_imm: jmp rel 3 if [fp] != 0
            // Argument is 1, return 1
            Instruction::from([6, 1, 0, minus_3.0]), // 11: store_imm: [fp - 3] = 1
            Instruction::from([15, 0, 0, 0]),        // 12: ret
            // Compute fib(n-1) + fib(n-2)
            // fib(n-1)
            // n - 1 is already stored at [fp], ready to be used as argument.
            Instruction::from([12, 2, 6, 0]), // 13: call_abs_imm: call fib(n-1)
            Instruction::from([4, 1, 0, minus_3.0]), // 14: store_deref_fp: [fp - 3] = [fp + 1]
            // fib(n-2)
            Instruction::from([3, 0, 1, 0]), // 15: Store n - 2, from previously computed n - 1 [fp] = [fp] - 1
            Instruction::from([12, 2, 6, 0]), // 16: call_abs_imm: call fib(n-2)
            // Return value of fib(n-1) + fib(n-2)
            Instruction::from([0, minus_3.0, 1, minus_3.0]), // 17: store_add_fp_fp: [fp - 3] = [fp - 3] + [fp + 1]
            Instruction::from([15, 0, 0, 0]),                // 18: ret
        ];
        let instructions_len = instructions.len() as u32;
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

        let fp_offset = 3;
        vm.run_from_entrypoint(0, fp_offset).unwrap();
        // Verify that FP is still at the end of the program
        assert_eq!(vm.state.fp, M31(instructions_len + fp_offset));
        // Verify PC reached the end of the program
        assert_eq!(vm.state.pc, M31(instructions_len));

        // Result is stored at [fp - 3].
        assert_eq!(
            vm.memory.get_data(vm.state.fp - M31(3)).unwrap(),
            M31(fib(n))
        );
    }

    #[test]
    fn test_write_binary_trace() {
        // Create a program with two instructions to generate a trace.
        let instructions = vec![
            Instruction::from([6, 10, 0, 0]), // [fp + 0] = 10
            Instruction::from([6, 20, 0, 1]), // [fp + 1] = 20
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

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
            Instruction::from([6, 10, 0, 0]), // store_imm: [fp + 0] = 10
            Instruction::from([6, 20, 0, 1]), // store_imm: [fp + 1] = 20
            Instruction::from([0, 0, 1, 2]),  // store_add_fp_fp: [fp + 2] = [fp + 0] + [fp + 1]
        ];
        let program = Program::from(instructions);
        let mut vm = VM::try_from(program).unwrap();

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
        assert_eq!(file_contents, vm.memory.serialize_trace());

        // Verify that the memory trace is not empty (we should have memory accesses).
        assert!(!file_contents.is_empty());

        // The memory trace should contain entries for:
        // 1. Instruction fetches (3 instructions)
        // 2. Memory stores (3 store operations)
        // 3. Memory loads (2 loads for the addition operation)
        // Each entry is 5 * 4 = 20 bytes (addr + 4 QM31 components)
        let expected_entries = 3 + 3 + 2; // instruction fetches + stores + loads
        let expected_size = expected_entries * 5 * 4; // 5 u32 values * 4 bytes each
        assert_eq!(file_contents.len(), expected_size);
    }
}
