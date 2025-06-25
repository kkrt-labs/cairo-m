pub mod instructions;
pub mod state;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use cairo_m_common::instruction::InstructionError;
use cairo_m_common::{Instruction, Program, State};
use instructions::opcode_to_instruction_fn;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

use crate::memory::{Memory, MemoryError};

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
    pub trace: Vec<State>,
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
        self.trace.push(self.state);
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
