mod loader;
mod lowering;

use cairo_m_common::InputValue;
use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::{PassManager, PrettyPrint};
use cairo_m_runner::run_cairo_program;
use clap::Parser;
use loader::{BlocklessDagModule, WasmLoadError};
use lowering::lower_program_to_mir;
use std::{fs, path::PathBuf};
use tracing::Level;

/// Cairo-M WASM to MIR compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input WASM file to compile
    #[arg(value_name = "WASM_FILE")]
    input: PathBuf,

    /// Output file to write the compiled program to
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose output (shows MIR)
    #[arg(short, long)]
    verbose: bool,

    /// Function name to run after compilation (entrypoint)
    #[arg(short = 'f', long)]
    function: Option<String>,

    /// Arguments to pass to the entrypoint (repeat -a for multiple args)
    #[arg(short = 'a', long = "arg")]
    args: Vec<i64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.verbose {
        tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    }

    let wasm_file = fs::read(&args.input).map_err(|e| WasmLoadError::IoError { source: e })?;
    let module = BlocklessDagModule::from_bytes(&wasm_file)?;
    let mir_module = lower_program_to_mir(&module, PassManager::standard_pipeline())?;
    let program = compile_module(&mir_module)?;

    // If a function is provided, execute it with args and print return values
    if let Some(func) = args.function.as_deref() {
        if args.verbose {
            println!(
                "Running entrypoint '{}' with {} args",
                func,
                args.args.len()
            );
        }
        let input_values = args
            .args
            .iter()
            .map(|&v| InputValue::Number(v))
            .collect::<Vec<_>>();
        let output = run_cairo_program(&program, func, &input_values, Default::default())?;
        println!("{:?}", output.return_values);
    } else {
        // Otherwise, serialize the program to JSON
        let json = sonic_rs::to_string_pretty(&program)?;

        // Write output or print to stdout
        match args.output {
            Some(output_path) => {
                fs::write(&output_path, &json)?;
                println!(
                    "Compilation successful. Output written to '{}'",
                    output_path.display()
                );
            }
            None => {
                println!("{}", json);
            }
        }
    }

    Ok(())
}
