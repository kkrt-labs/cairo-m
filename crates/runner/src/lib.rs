pub mod memory;
pub mod vm;

use cairo_m_compiler::CompiledProgram;
use stwo_prover::core::fields::m31::M31;
use vm::instructions::Instruction;
use vm::{Program, VmError, VM};

/// Result type for runner operations
pub type Result<T> = std::result::Result<T, RunnerError>;

/// Errors that can occur during program execution
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("Entry point '{0}' not found. Available entry points: {1:?}")]
    EntryPointNotFound(String, Vec<String>),

    #[error("VM error: {0}")]
    VmError(#[from] VmError),

    #[error("Failed to read return value: {0}")]
    ReturnValueError(String),
}

/// Options for running a Cairo program
#[derive(Debug, Clone, Default)]
pub struct RunnerOptions {
    // Empty for now
}

/// Result of running a Cairo program
#[derive(Debug)]
pub struct RunnerOutput {
    /// The return value of the program
    pub return_value: u32,
    /// The final VM state (for advanced usage)
    pub vm: VM,
}

/// Runs a compiled Cairo-M program
///
/// # Arguments
/// * `program` - The compiled program to run
/// * `entry_point` - Name of the entry point function to execute
/// * `options` - Runner options
///
/// # Returns
/// * `Ok(RunnerOutput)` - Program executed successfully with return value
/// * `Err(RunnerError)` - Execution failed
pub fn run_cairo_program(
    program: &CompiledProgram,
    entry_point: &str,
    _options: RunnerOptions,
) -> Result<RunnerOutput> {
    let pc_entrypoint = program.get_entry_point(entry_point).ok_or_else(|| {
        RunnerError::EntryPointNotFound(
            entry_point.to_string(),
            program.entry_points.keys().cloned().collect(),
        )
    })?;

    let instructions: Vec<Instruction> =
        program.instructions.iter().map(Instruction::from).collect();

    let vm_program = Program { instructions };
    let mut vm = VM::try_from(vm_program)?;

    // TODO: Get entrypoint information from the compiled program to know how many args / return data to allocate
    const FP_OFFSET: u32 = 3;
    vm.run_from_entrypoint(pc_entrypoint as u32, FP_OFFSET)?;

    // Get the return value from [fp - 3]
    let return_address = vm.state.fp - M31::from(FP_OFFSET);
    let return_value = vm
        .memory
        .get_data(return_address)
        .map_err(|e| RunnerError::ReturnValueError(e.to_string()))?;

    Ok(RunnerOutput {
        return_value: return_value.0,
        vm,
    })
}
