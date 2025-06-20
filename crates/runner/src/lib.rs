pub mod memory;
pub mod vm;

use cairo_m_common::Program;
use memory::MemoryError;
use stwo_prover::core::fields::m31::M31;
use vm::{VmError, VM};

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
    ReturnValueError(#[from] MemoryError),
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
    /// The final VM
    pub vm: VM,
}

/// Runs a compiled Cairo-M program
///
/// ## Arguments
/// * `program` - The compiled program to run
/// * `entrypoint` - Name of the entry point function to execute
/// * `options` - Runner options
///
/// ## Returns
/// * `Ok(RunnerOutput)` - Program executed successfully with return value
/// * `Err(RunnerError)` - Execution failed
pub fn run_cairo_program(
    program: &Program,
    entrypoint: &str,
    _options: RunnerOptions,
) -> Result<RunnerOutput> {
    let entrypoint_pc = program.get_entrypoint(entrypoint).ok_or_else(|| {
        RunnerError::EntryPointNotFound(
            entrypoint.to_string(),
            program.entrypoints.keys().cloned().collect(),
        )
    })?;

    let mut vm = VM::try_from(program)?;

    // TODO: Get entrypoint information from the compiled program to know how many args / return data to allocate
    const FP_OFFSET: u32 = 3;
    vm.run_from_entrypoint(entrypoint_pc as u32, FP_OFFSET)?;

    // Get the return value from [fp - 3]
    let return_address = vm.state.fp - M31::from(FP_OFFSET);
    let return_value = vm.memory.get_data(return_address)?;

    Ok(RunnerOutput {
        return_value: return_value.0,
        vm,
    })
}
