use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use cairo_m_common::{Program, parse_cli_arg};
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

    /// Arguments to pass to the entrypoint function
    ///
    /// Supported types:
    ///   • Numbers: 42, -5 (use quotes for negative: "-5")
    ///   • Booleans: true, false
    ///   • Tuples: (1,2,3) or [1,2,3] or [1,[2,3]] for nested
    ///   • Structs: {1,2,3} or {1,{2,3}} for nested (fields are positional)
    ///   • Mixed: {1,[true,{2,3}]} combining structs and tuples
    ///
    /// Note: Fixed-size arrays are not currently supported as input arguments.
    ///       See Linear issue CORE-1118 for array support tracking.
    ///
    /// Examples:
    ///   --arguments 42 true "(10,20)"
    ///   --arguments "{25,true,[10,20]}"
    #[arg(short, long, value_parser = parse_cli_arg, num_args = 0.., allow_hyphen_values = true, verbatim_doc_comment)]
    arguments: Vec<cairo_m_common::InputValue>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file_content = fs::read_to_string(&args.compiled_file)
        .with_context(|| format!("Error reading file '{}'", args.compiled_file.display()))?;

    let compiled_program: Program =
        sonic_rs::from_str(&file_content).context("Failed to parse compiled program")?;

    let output = run_cairo_program(
        &compiled_program,
        &args.entrypoint,
        &args.arguments,
        Default::default(),
    )
    .context("Execution failed")?;

    println!("Run succeeded and returned: {:?}", output.return_values);

    Ok(())
}
