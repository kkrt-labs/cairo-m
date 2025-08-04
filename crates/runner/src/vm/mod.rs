pub mod instructions;
pub mod state;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use cairo_m_common::execution::Segment;
use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Program, State};
use instructions::opcode_to_instruction_fn;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use thiserror::Error;

use crate::memory::{Memory, MemoryError};
use crate::RunnerOptions;

/// The status of the overall program execution.
///
/// - `Complete`: The program has reached the final program counter (PC) and is complete.
/// - `Ongoing`: The program has reached the step limit and will continue in the next continuation segment.
enum ExecutionStatus {
    Complete,
    Ongoing,
}

/// Custom error type for VM operations.
#[derive(Debug, Error)]
pub enum VmError {
    #[error("VM memory error: {0}")]
    Memory(#[from] MemoryError),
    #[error("VM instruction error: {0}")]
    Instruction(#[from] InstructionError),
    #[error("VM instruction execution error: {0}")]
    InstructionExecution(#[from] instructions::InstructionExecutionError),
    #[error("VM I/O error: {0}")]
    Io(#[from] io::Error),
}

/// The Cairo M Virtual Machine.
///
/// ## Fields
///
/// - `final_pc`: The final program counter (PC) of the executed program.
/// - `initial_memory`: the memory right before the first step.
/// - `memory`: Flat address space storing instructions and data
/// - `state`: Current processor state (PC, FP)
/// - `program_length`: Length of the program in instructions
/// - `trace`: Execution trace
/// - `segments`: chunks of execution containing necessary data for continuation.
#[derive(Debug, Default, Clone)]
pub struct VM {
    pub final_pc: M31,
    pub initial_memory: Vec<M31>,
    pub memory: Memory,
    pub state: State,
    pub program_length: M31,
    pub trace: Vec<State>,
    pub segments: Vec<Segment>,
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
    /// Instructions are variable-sized (1-5 M31 elements) and are packed into QM31 values
    /// (4 M31 elements each) with zero padding as needed.
    ///
    /// ## Arguments
    ///
    /// * `program` - The [`Program`] to load.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Memory`] if memory insertion fails.
    fn try_from(program: &Program) -> Result<Self, Self::Error> {
        // Flatten variable-sized instructions into memory words
        let mut memory_words = Vec::new();
        for instruction in &program.instructions {
            memory_words.extend(instruction.to_m31_vec());
        }

        // Create memory and load instructions starting at address 0
        let program_length = M31(memory_words.len() as u32);
        let final_pc = program_length;
        let memory = Memory::from_iter(memory_words);

        // Create state with PC at entrypoint and FP just after the bytecode
        let state = State {
            pc: M31::zero(),
            fp: final_pc,
        };

        Ok(Self {
            final_pc,
            initial_memory: vec![],
            memory,
            state,
            program_length,
            trace: vec![],
            segments: vec![],
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
        // Get the complete instruction from memory
        let instruction_m31s = self.memory.get_instruction(self.state.pc)?;

        let instruction: Instruction = instruction_m31s.try_into()?;

        // Get opcode from the instruction for dispatch
        let instruction_fn = opcode_to_instruction_fn(instruction.opcode_value().into())?;
        self.trace.push(self.state);
        self.state = instruction_fn(&mut self.memory, self.state, &instruction)?;
        Ok(())
    }

    /// Executes the loaded program from start to either completion or target number of steps is reached.
    ///
    /// This method runs the VM by repeatedly calling [`step()`](Self::step) until the program
    /// counter reaches the end of the loaded instructions. It assumes that the program is loaded
    /// at the beginning of the memory ([0..instructions_length]).
    ///
    /// ## Arguments
    ///
    /// * `max_steps` - The maximum number of steps to execute, DEFAULT_max_steps by default.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError`] if any instruction execution fails:
    /// - Invalid opcodes ([`VmError::Instruction`])
    /// - Memory errors ([`VmError::Memory`])
    fn execute(&mut self, max_steps: usize) -> Result<ExecutionStatus, VmError> {
        if self.final_pc.is_zero() {
            return Ok(ExecutionStatus::Complete);
        }

        while self.state.pc != self.final_pc && self.trace.len() < max_steps {
            self.step()?;
        }

        // Push the final state to the trace
        self.trace.push(self.state);

        if self.state.pc == self.final_pc {
            Ok(ExecutionStatus::Complete)
        } else {
            Ok(ExecutionStatus::Ongoing)
        }
    }

    /// Finalizes the current segment by moving the current segment data into a new segment.
    ///
    /// This method is called when the numbers of steps for the current segment is reached or for the last segment.
    ///
    /// ## Arguments
    ///
    /// * `is_last_segment` - If true, this is the last segment and we can move all data without cloning. If false, we need to clone memory for the next segment.
    pub fn finalize_segment(&mut self, is_last_segment: bool) {
        if is_last_segment {
            // For the last segment, we can move everything without cloning
            self.segments.push(Segment {
                initial_memory: std::mem::take(&mut self.initial_memory),
                memory_trace: std::mem::take(&mut self.memory.trace),
                trace: std::mem::take(&mut self.trace),
            });
        } else {
            // For intermediate segments, we need to clone memory for the next segment
            self.segments.push(Segment {
                initial_memory: std::mem::replace(
                    &mut self.initial_memory,
                    self.memory.data.clone(),
                ),
                memory_trace: std::mem::take(&mut self.memory.trace),
                trace: std::mem::take(&mut self.trace),
            });
        }
    }

    /// Executes the loaded program from a given entrypoint and frame pointer.
    ///
    /// - The PC entrypoint is the first instruction of the function to execute in the program.
    /// - The FP offset accounts for the calling convention of the executed function: arguments, return values, return address.
    /// - Arguments are written to memory before the frame pointer.
    ///
    /// The call stack of the entrypoint is initialized here.
    ///
    /// ## Arguments
    ///
    /// * `pc_entrypoint` - The program counter (PC) to start execution from.
    /// * `fp_offset` - The frame pointer (FP) offset to start execution from.
    /// * `args` - The arguments to pass to the function.
    /// * `num_return_values` - The number of return values to expect from the function.
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
        args: &[M31],
        num_return_values: usize,
        options: &RunnerOptions,
    ) -> Result<(), VmError> {
        // Write arguments to memory before the frame pointer
        // Arguments should be at [new_fp - M - K - 2 + i] for arg i
        // Writing the arguments does not log an trace entry.
        let initial_fp = self.state.fp;
        let new_fp = initial_fp + M31::from(fp_offset);
        for (i, arg) in args.iter().enumerate() {
            // For M args and K returns: arg_i is at [fp - M - K - 2 + i]
            let offset = args.len() + num_return_values + 2 - i;
            let arg_address = new_fp - M31::from(offset as u32);
            self.memory.insert_no_trace(arg_address, *arg)?;
        }

        self.state.pc = M31(pc_entrypoint);
        self.state.fp = new_fp;

        self.memory
            .insert_entrypoint_call(&self.final_pc, &self.state.fp)?;
        self.initial_memory = self.memory.data.clone();

        loop {
            match self.execute(options.max_steps) {
                Ok(ExecutionStatus::Complete) => break self.finalize_segment(true),
                Ok(ExecutionStatus::Ongoing) => self.finalize_segment(false),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes the serialized trace to binary files, one per segment.
    ///
    /// This function creates a file for each segment with the naming pattern:
    /// `<base_path>_segment_<index>.<extension>`
    ///
    /// For example, if the path is "trace.bin", the files will be:
    /// - trace_segment_0.bin
    /// - trace_segment_1.bin
    /// - etc.
    ///
    /// ## Arguments
    ///
    /// * `path` - The base file path for the binary trace files.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Io`] if:
    /// - Any file cannot be created or opened for writing
    /// - Writing to any file fails
    pub fn write_binary_trace<P: AsRef<Path>>(&self, path: P) -> Result<(), VmError> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        // Extract base name and extension
        let (base, ext) = path_str.rfind('.').map_or_else(
            || (path_str.as_ref(), ""),
            |dot_pos| (&path_str[..dot_pos], &path_str[dot_pos..]),
        );

        // Write each segment's trace
        for (i, segment) in self.segments.iter().enumerate() {
            let segment_path = format!("{}_segment_{}{}", base, i, ext);
            let serialized_trace = segment.serialize_segment_trace();
            let mut file = File::create(&segment_path)?;
            file.write_all(&serialized_trace)?;
        }

        Ok(())
    }

    /// Writes the serialized memory trace to binary files, one per segment.
    ///
    /// This function creates a file for each segment with the naming pattern:
    /// `<base_path>_segment_<index>.<extension>`
    ///
    /// ## File Format
    ///
    /// Each file contains:
    /// 1. Program length (4 bytes, little-endian u32)
    /// 2. Memory entries, each consisting of:
    ///    - Address (4 bytes, little-endian u32)
    ///    - QM31 value (16 bytes, 4 x little-endian u32)
    ///
    /// ## Arguments
    ///
    /// * `path` - The base file path for the binary memory trace files.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError::Io`] if:
    /// - Any file cannot be created or opened for writing
    /// - Writing to any file fails
    pub fn write_binary_memory_trace<P: AsRef<Path>>(&self, path: P) -> Result<(), VmError> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        // Extract base name and extension
        let (base, ext) = path_str.rfind('.').map_or_else(
            || (path_str.as_ref(), ""),
            |dot_pos| (&path_str[..dot_pos], &path_str[dot_pos..]),
        );

        // Write each segment's memory trace
        for (i, segment) in self.segments.iter().enumerate() {
            let segment_path = format!("{}_segment_{}{}", base, i, ext);
            let mut file = File::create(&segment_path)?;

            // Write program length
            file.write_all(&self.program_length.0.to_le_bytes())?;

            // Serialize and write the segment's memory trace
            let serialized = segment.serialize_segment_memory_trace();
            file.write_all(&serialized)?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "./vm_tests.rs"]
mod vm_tests;

#[cfg(test)]
pub mod test_utils {

    macro_rules! assert_memory_value {
        ($vm:expr, addr = $addr:expr, value = $val:expr) => {
            assert_eq!(
                $vm.memory
                    .get_felt(stwo_prover::core::fields::m31::M31::from($addr))
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
    pub(crate) use {assert_memory_value, assert_vm_state};
}
