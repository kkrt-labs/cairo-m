use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler::CompiledProgram;
use cairo_m_runner::{run_cairo_program, RunnerOptions};
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

fn main() {
    let args = Args::parse();

    // Read the compiled program
    let file_content = match fs::read_to_string(&args.compiled_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "Error reading file '{}': {}",
                args.compiled_file.display(),
                e
            );
            process::exit(1);
        }
    };

    let compiled_program: CompiledProgram = match sonic_rs::from_str(&file_content) {
        Ok(program) => program,
        Err(e) => {
            eprintln!("Failed to parse compiled program: {}", e);
            process::exit(1);
        }
    };

    // Run the program
    match run_cairo_program(&compiled_program, &args.entrypoint, Default::default()) {
        Ok(output) => {
            println!("Run succeeded and returned: [{}]", output.return_value);
        }
        Err(e) => {
            eprintln!("Execution failed: {}", e);
            process::exit(1);
        }
    }
}
