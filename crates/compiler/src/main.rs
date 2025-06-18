use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler::{compile_cairo, format_diagnostics, CompilerError, CompilerOptions};
use clap::Parser;

/// Cairo-M compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file to compile
    #[arg(short, long)]
    input: PathBuf,

    /// Output file to write the compiled program to
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose output (shows MIR)
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    let source_text = fs::read_to_string(&args.input).unwrap_or_else(|e| {
        eprintln!("Error reading file '{}': {}", args.input.display(), e);
        process::exit(1);
    });

    let source_name = args.input.display().to_string();
    let options = CompilerOptions {
        verbose: args.verbose,
    };

    let output = compile_cairo(source_text.clone(), source_name, options).unwrap_or_else(|e| {
        match &e {
            CompilerError::ParseErrors(diagnostics)
            | CompilerError::SemanticErrors(diagnostics) => {
                let error_msg = format_diagnostics(&source_text, diagnostics, true);
                eprintln!("{}", error_msg);
            }
            CompilerError::MirGenerationFailed => {
                eprintln!("Failed to generate MIR");
            }
            CompilerError::CodeGenerationFailed(msg) => {
                eprintln!("Code generation failed: {}", msg);
            }
        }
        process::exit(1);
    });

    // Print any warnings
    if !output.diagnostics.is_empty() {
        let diagnostic_messages = format_diagnostics(&source_text, &output.diagnostics, true);
        println!("{}", diagnostic_messages);
    }

    let json = sonic_rs::to_string_pretty(&*output.program).unwrap_or_else(|e| {
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
}
