use std::fs;
use std::path::PathBuf;

use cairo_m_compiler::CompiledProgram;
use cairo_m_runner::vm::instructions::Instruction;
use cairo_m_runner::vm::{Program, VM};
use clap::{Parser, ValueHint};
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Cairo-M Runner - Execute compiled Cairo-M programs",
    long_about = None
)]
struct Args {
    /// Path to the compiled Cairo file (JSON format)
    #[arg(value_hint = ValueHint::FilePath)]
    compiled_file: PathBuf,

    /// Entry point function name to execute
    #[arg(short, long)]
    entrypoint: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let file_content = fs::read_to_string(&args.compiled_file)?;
    let compiled_program: CompiledProgram = sonic_rs::from_str(&file_content)?;

    let pc_entrypoint = compiled_program
        .get_entry_point(&args.entrypoint)
        .ok_or_else(|| {
            format!(
                "Entry point '{}' not found. Available entry points: {:?}",
                args.entrypoint,
                compiled_program.entry_points.keys().collect::<Vec<_>>()
            )
        })?;

    // Convert compiled instructions to VM instructions
    // TODO: unify instruction generation with the compiler
let instructions: Vec<Instruction> = compiled_program
        .instructions
        .iter()
        .map(Instruction::from)
        .collect();

    let program = Program { instructions };
    let mut vm = VM::try_from(program)?;

    // TODO: Get entrypoint information from the compiled program to know how many args / return data to allocate.
    const FP_OFFSET: u32 = 3;
    vm.run_from_entrypoint(pc_entrypoint as u32, FP_OFFSET)?;

    // TODO: add support for multiple return values once supported.
    // Get the return value from [fp - 3]
    let return_address = vm.state.fp - M31::from(FP_OFFSET);
    let return_value = vm
        .memory
        .get_data(return_address)
        .map_err(|e| format!("Failed to read return value: {e}"))?;

    println!("Run succeeded and returned: [{}]", return_value.0);

    Ok(())
}
