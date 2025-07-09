use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use cairo_m_common::Program;
use cairo_m_prover::adapter::{MockHasher, import_from_runner_output};
use cairo_m_prover::prover::prove_cairo_m;
use cairo_m_runner::run_cairo_program;
use clap::{Parser, ValueHint};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Cairo-M Prover - Run and prove compiled Cairo-M programs",
    long_about = None
)]
struct Args {
    /// Path to the compiled Cairo file (JSON format)
    #[arg(value_hint = ValueHint::FilePath)]
    compiled_file: PathBuf,

    /// Entry point function name to execute
    #[arg(short, long)]
    entrypoint: String,

    /// Arguments to pass to the entrypoint
    #[arg(short, long)]
    arguments: Vec<u32>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let file_content = fs::read_to_string(&args.compiled_file)
        .with_context(|| format!("Error reading file '{}'", args.compiled_file.display()))?;

    let compiled_program: Program =
        sonic_rs::from_str(&file_content).context("Failed to parse compiled program")?;

    let fn_args: Vec<M31> = args.arguments.iter().map(|arg| M31::from(*arg)).collect();
    let output = run_cairo_program(
        &compiled_program,
        &args.entrypoint,
        &fn_args,
        Default::default(),
    )
    .context("Execution failed")?;

    let mut prover_input =
        import_from_runner_output(output).context("Failed to import from runner output")?;
    let _proof: cairo_m_prover::Proof<stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleHasher> =
        prove_cairo_m::<Blake2sMerkleChannel, MockHasher>(&mut prover_input, None)
            .context("Failed to prove")?;

    Ok(())
}
