pub mod memory;
pub mod vm;

use cairo_m_common::{
    decode_abi_values, encode_input_args, AbiCodecError, CairoMValue, InputValue,
};
use cairo_m_common::{Program, PublicAddressRanges};
use memory::MemoryError;
use stwo_prover::core::fields::m31::M31;
use vm::{VmError, VM};

/// Result type for runner operations
pub type Result<T> = std::result::Result<T, RunnerError>;

// Current limitation is that the maximum clock difference must be < 2^20
const DEFAULT_MAX_STEPS: usize = (1 << 20) - 1;

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

    #[error("ABI encode/decode error: {0}")]
    AbiError(#[from] AbiCodecError),
}

/// Options for running a Cairo program
#[derive(Debug, Clone)]
pub struct RunnerOptions {
    /// The maximum number of steps to execute, DEFAULT_max_steps by default.
    pub max_steps: usize,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
        }
    }
}

/// Result of running a Cairo program
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The return values of the program
    pub return_values: Vec<M31>,
    /// The final VM
    pub vm: VM,
    /// The public address ranges for structured access to program, input, and output data
    pub public_address_ranges: PublicAddressRanges,
}

/// Runs a compiled Cairo-M program using input values.
/// Encodes `args` according to the ABI types, runs the program, then decodes the return values.
pub fn run_cairo_program(
    program: &Program,
    entrypoint: &str,
    args: &[InputValue],
    options: RunnerOptions,
) -> Result<(Vec<CairoMValue>, RunnerOutput)> {
    let entrypoint_info = program.get_entrypoint(entrypoint).ok_or_else(|| {
        RunnerError::EntryPointNotFound(
            entrypoint.to_string(),
            program.entrypoints.keys().cloned().collect(),
        )
    })?;
    let encoded_args = encode_input_args(&entrypoint_info.params, args)?;
    let output = run_cairo_program_raw_args(program, entrypoint, &encoded_args, options)?;
    let decoded = decode_abi_values(&entrypoint_info.returns, &output.return_values)?;
    Ok((decoded, output))
}

/// Runs a compiled Cairo-M program with raw M31 values as arguments.
///
/// ## Arguments
/// * `program` - The compiled program to run
/// * `entrypoint` - Name of the entry point function to execute
/// * `args` - Raw M31 arguments to pass to the entrypoint function
/// * `options` - Runner options
///
/// ## Returns
/// * `Ok(RunnerOutput)` - Program executed successfully with return values
/// * `Err(RunnerError)` - Execution failed
fn run_cairo_program_raw_args(
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

    let arg_slots: usize = entrypoint_info
        .params
        .iter()
        .map(|p| p.size_in_slots())
        .sum();
    let ret_slots: usize = entrypoint_info
        .returns
        .iter()
        .map(|r| r.size_in_slots())
        .sum();

    // Validate argument count matches expected slots
    if args.len() != arg_slots {
        return Err(RunnerError::ArgumentCountMismatch {
            expected: arg_slots,
            provided: args.len(),
        });
    }

    let mut vm = VM::try_from(program)?;

    // Calculate FP offset based on Cairo-M calling convention:
    // Frame layout: [args, return_values, old_fp, return_pc]
    // FP points to the first address after the frame
    let fp_offset = args.len() + ret_slots + 2;

    vm.run_from_entrypoint(
        entrypoint_info.pc as u32,
        fp_offset as u32,
        args,
        ret_slots,
        &options,
    )?;

    // Retrieve return values from memory
    // Return values are stored at [fp - return_value_slots - 2] to [fp - 3]
    let mut return_values = Vec::with_capacity(ret_slots);
    for i in 0..ret_slots {
        let return_address = vm.state.fp - M31::from((ret_slots + 2 - i) as u32);
        let value = vm.memory.get_data(return_address)?;
        return_values.push(value);
    }

    // Define public addresses
    let public_address_ranges =
        PublicAddressRanges::new(vm.program_length.0, args.len(), ret_slots);

    Ok(RunnerOutput {
        return_values,
        vm,
        public_address_ranges,
    })
}
