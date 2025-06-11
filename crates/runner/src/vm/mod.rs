pub mod instructions;
pub mod state;

use crate::memory::{Memory, MemoryError};
use crate::vm::state::State;
use instructions::Instruction;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

/// Custom error type for VM operations.
#[derive(Debug, Error)]
pub enum VmError {
    /// An error occurred in the memory module.
    #[error("VM memory error: {0}")]
    Memory(#[from] MemoryError),
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

        Ok(Self { memory, state })
    }
}

#[cfg(test)]
mod tests {
    use num_traits::{One, Zero};

    use crate::vm::{Instruction, Program, VM};
    use stwo_prover::core::fields::m31::M31;

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
}
