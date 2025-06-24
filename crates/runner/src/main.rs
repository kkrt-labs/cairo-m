use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use cairo_m_common::Program;
use cairo_m_runner::run_cairo_program;
use clap::{Parser, ValueHint};

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

    let output = run_cairo_program(&compiled_program, &args.entrypoint, Default::default())
        .context("Execution failed")?;

    println!("Run succeeded and returned: [{}]", output.return_value);

    Ok(())
}
