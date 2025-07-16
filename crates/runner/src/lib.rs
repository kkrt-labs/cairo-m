pub mod memory;
pub mod vm;

use cairo_m_common::Program;
use memory::MemoryError;
use stwo_prover::core::fields::m31::M31;
use vm::{VM, VmError};

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

    #[error("Argument count mismatch: expected {expected}, provided {provided}")]
    ArgumentCountMismatch { expected: usize, provided: usize },
}

/// Options for running a Cairo program
#[derive(Debug, Clone, Default)]
pub struct RunnerOptions {
    /// The maximum number of steps to execute, MAX_N_STEPS by default.
    pub n_steps: Option<usize>,
}

/// Result of running a Cairo program
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The return values of the program
    pub return_values: Vec<M31>,
    /// The final VM
    pub vm: VM,
}

/// Runs a compiled Cairo-M program
///
/// ## Arguments
/// * `program` - The compiled program to run
/// * `entrypoint` - Name of the entry point function to execute
/// * `args` - Arguments to pass to the entrypoint function
/// * `options` - Runner options
///
/// ## Returns
/// * `Ok(RunnerOutput)` - Program executed successfully with return values
/// * `Err(RunnerError)` - Execution failed
pub fn run_cairo_program(
    program: &Program,
    entrypoint: &str,
    args: &[M31],
    options: RunnerOptions,
) -> Result<RunnerOutput> {
    let entrypoint_info = program.get_entrypoint(entrypoint).ok_or_else(|| {
        RunnerError::EntryPointNotFound(
            entrypoint.to_string(),
            program.entrypoints.keys().cloned().collect(),
        )
    })?;

    // Use provided num_return_values or get from entrypoint info
    let num_return_values = entrypoint_info.num_return_values;

    // Validate argument count matches expected
    if args.len() != entrypoint_info.args.len() {
        return Err(RunnerError::ArgumentCountMismatch {
            expected: entrypoint_info.args.len(),
            provided: args.len(),
        });
    }

    let mut vm = VM::try_from(program)?;

    // Calculate FP offset based on Cairo-M calling convention:
    // Frame layout: [args, return_values, old_fp, return_pc]
    // FP points to the first address after the frame
    let fp_offset = args.len() + num_return_values + 2;

    vm.run_from_entrypoint(
        entrypoint_info.pc as u32,
        fp_offset as u32,
        args,
        num_return_values,
        options,
    )?;

    // Retrieve return values from memory
    // Return values are stored at [fp - num_return_values - 2] to [fp - 3]
    let mut return_values = Vec::with_capacity(num_return_values);
    for i in 0..num_return_values {
        let return_address = vm.state.fp - M31::from((num_return_values + 2 - i) as u32);
        let value = vm.memory.get_data(return_address)?;
        return_values.push(value);
    }

    Ok(RunnerOutput { return_values, vm })
}
