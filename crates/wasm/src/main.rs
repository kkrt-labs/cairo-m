mod flattening;
mod loader;

use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::{PassManager, PrettyPrint};
use clap::Parser;
use flattening::DagToMir;
use loader::BlocklessDagModule;
use std::{fs, path::PathBuf};

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

    /// Show only MIR without compiling to final program
    #[arg(long)]
    mir_only: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load WASM file into WOMIR representation
    let module = BlocklessDagModule::from_file(&args.input.to_string_lossy())?;

    if args.verbose {
        println!("Successfully loaded WASM file: {}", args.input.display());
        let function_count = module.with_program(|program| program.functions.len());
        println!("Functions found: {}", function_count);
    }

    // Convert WASM to MIR
    let mir_module = DagToMir::new(module).to_mir(PassManager::standard_pipeline())?;

    if args.verbose {
        println!("Successfully converted to MIR");
        println!("MIR functions: {}", mir_module.function_count());
    }

    // If user wants MIR only, print it and exit
    if args.mir_only {
        let mir_output = mir_module.pretty_print(0);
        match args.output {
            Some(output_path) => {
                fs::write(&output_path, &mir_output)?;
                println!("MIR output written to '{}'", output_path.display());
            }
            None => {
                println!("{}", mir_output);
            }
        }
        return Ok(());
    }

    // Compile MIR to final program
    let program = compile_module(&mir_module)?;

    if args.verbose {
        println!("Successfully compiled to final program");
    }

    // Serialize the program to JSON
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

    Ok(())
}
