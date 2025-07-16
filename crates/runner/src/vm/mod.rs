pub mod instructions;
pub mod state;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::program::Segment;
use cairo_m_common::{Instruction, Program, State};
use instructions::opcode_to_instruction_fn;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

use crate::RunnerOptions;
use crate::memory::{Memory, MemoryError};

// Current limitation is that the maximum clock difference must be < 2^20
const MAX_N_STEPS: usize = (1 << 20) - 1;

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
    #[error("VM didn't reach the final PC, step limit reached: {0}")]
    StepLimitReached(usize),
    #[error("VM I/O error: {0}")]
    Io(#[from] io::Error),
}

/// The Cairo M Virtual Machine.
///
/// ## Fields
///
/// - `final_pc`: The final program counter (PC) of the executed program.
/// - `memory`: Flat address space storing instructions and data
/// - `state`: Current processor state (PC, FP)
/// - `program_length`: Length of the program in instructions
/// - `trace`: Execution trace
#[derive(Debug, Default, Clone)]
pub struct VM {
    pub final_pc: M31,
    pub initial_memory: Vec<QM31>,
    pub memory: Memory,
    pub state: State,
    pub program_length: M31,
    pub trace: Vec<State>,
    pub steps_counter: usize,
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
            initial_memory: vec![],
            memory,
            state,
            program_length,
            trace: vec![],
            steps_counter: 0,
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
        let instruction: Instruction = self.memory.get_instruction(self.state.pc)?.try_into()?;
        let instruction_fn = opcode_to_instruction_fn(M31::from(instruction.opcode))?;
        self.trace.push(self.state);
        self.state = instruction_fn(&mut self.memory, self.state, &instruction)?;
        Ok(())
    }

    /// Executes the loaded program from start to completion with a limit on the number of steps.
    ///
    /// This method runs the VM by repeatedly calling [`step()`](Self::step) until the program
    /// counter reaches the end of the loaded instructions. It assumes that the program is loaded
    /// at the beginning of the memory ([0..instructions_length]). The prover enforces that the
    /// maximum number of steps is less than 2^20.
    ///
    /// ## Arguments
    ///
    /// * `n_steps` - The maximum number of steps to execute, MAX_N_STEPS by default.
    ///
    /// ## Errors
    ///
    /// Returns a [`VmError`] if any instruction execution fails:
    /// - Invalid opcodes ([`VmError::Instruction`])
    /// - Memory errors ([`VmError::Memory`])
    /// - Step limit error ([`VmError::StepLimitReached`])
    fn execute(&mut self, n_steps: Option<usize>) -> Result<ExecutionStatus, VmError> {
        if self.final_pc.is_zero() {
            return Ok(ExecutionStatus::Complete);
        }

        while self.state.pc != self.final_pc && self.steps_counter < n_steps.unwrap_or(MAX_N_STEPS)
        {
            self.step()?;
            self.steps_counter += 1;
        }

        // Push the final state to the trace
        self.trace.push(self.state);

        if self.state.pc == self.final_pc {
            Ok(ExecutionStatus::Complete)
        } else if self.steps_counter >= MAX_N_STEPS {
            Err(VmError::StepLimitReached(self.steps_counter))
        } else {
            Ok(ExecutionStatus::Ongoing)
        }
    }

    fn finalize_segment(&mut self) {
        // Move the current segment data into a new segment
        self.segments.push(Segment {
            initial_memory: std::mem::replace(&mut self.initial_memory, self.memory.data.clone()),
            memory_trace: std::mem::take(&mut self.memory.trace),
            trace: std::mem::take(&mut self.trace),
        });
        self.steps_counter = 0;
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
        options: RunnerOptions,
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
            self.memory.insert_no_trace(arg_address, (*arg).into())?;
        }

        self.state.pc = M31(pc_entrypoint);
        self.state.fp = new_fp;

        self.memory
            .insert_entrypoint_call(&self.final_pc, &self.state.fp)?;
        self.initial_memory = self.memory.data.clone();

        loop {
            match self.execute(options.n_steps) {
                Ok(ExecutionStatus::Complete) => {
                    self.finalize_segment();
                    break;
                }
                Ok(ExecutionStatus::Ongoing) => self.finalize_segment(),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Serializes a segment's trace to a byte vector.
    ///
    /// Each trace entry consists of `fp` and `pc` values, both `u32`.
    /// This function serializes the trace as a flat sequence of bytes.
    /// For each entry, it first serializes `fp` into little-endian bytes,
    /// followed by the little-endian bytes of `pc`.
    ///
    /// ## Arguments
    ///
    /// * `segment` - The segment to serialize
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized trace data for the segment.
    fn serialize_segment_trace(segment: &Segment) -> Vec<u8> {
        segment
            .trace
            .iter()
            .flat_map(|entry| [entry.fp.0, entry.pc.0])
            .flat_map(u32::to_le_bytes)
            .collect()
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
            let serialized_trace = Self::serialize_segment_trace(segment);
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
    /// Each file starts with the program length, followed by the serialized memory trace
    /// for that segment.
    ///
    /// ## Arguments
    ///
    /// * `path` - The base file path for the binary memory trace files.
    ///
    /// Serializes the memory trace of a single segment into a binary format.
    ///
    /// The binary format consists of a sequence of memory entries, where each entry contains:
    /// - Address (4 bytes, little-endian u32)
    /// - Value (16 bytes, representing QM31 as 4 u32 values in little-endian)
    ///
    /// ## Arguments
    ///
    /// * `segment` - The segment whose memory trace should be serialized
    ///
    /// ## Returns
    ///
    /// A vector of bytes containing the serialized memory trace
    fn serialize_segment_memory_trace(segment: &Segment) -> Vec<u8> {
        let memory_trace = segment.memory_trace.borrow();
        memory_trace
            .iter()
            .flat_map(|entry| {
                let mut bytes = Vec::with_capacity(20);
                bytes.extend_from_slice(&entry.addr.0.to_le_bytes());
                // QM31 has two CM31 fields, each CM31 has two M31 fields
                bytes.extend_from_slice(&entry.value.0.0.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.0.1.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.1.0.0.to_le_bytes());
                bytes.extend_from_slice(&entry.value.1.1.0.to_le_bytes());
                bytes
            })
            .collect()
    }

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
            let serialized = Self::serialize_segment_memory_trace(segment);
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
