use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::PassManager;
use cairo_m_wasm::loader::{BlocklessDagModule, WasmLoadError};
use cairo_m_wasm::lowering::lower_program_to_mir;
use clap::Parser;
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

    let json = sonic_rs::to_string_pretty(&program).unwrap_or_else(|e| {
        eprintln!("Failed to serialize program: {}", e);
        process::exit(1);
    });

    // Write output or print to stdout
    match args.output {
        Some(output_path) => {
            fs::write(&output_path, &json).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to write output file '{}': {}",
                    output_path.display(),
                    e
                );
                process::exit(1);
            });
            println!(
                "Compilation successful. Output written to '{}'",
                output_path.display()
            );
        }
        None => {
            println!("{}", json);
        }
    }

    Ok(())
}
