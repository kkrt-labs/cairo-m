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

    // Read the input file
    let source_text = match fs::read_to_string(&args.input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", args.input.display(), e);
            process::exit(1);
        }
    };

    let source_name = args.input.display().to_string();
    let options = CompilerOptions {
        verbose: args.verbose,
    };

    // Compile the program
    let result = compile_cairo(source_text.clone(), source_name, options);

    match result {
        Ok(output) => {
            // Print any warnings
            if !output.warnings.is_empty() {
                let warning_msg = format_diagnostics(&source_text, &output.warnings, true);
                eprintln!("{}", warning_msg);
            }

            // Serialize the program to JSON
            let json = match sonic_rs::to_string_pretty(&*output.program) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to serialize program: {}", e);
                    process::exit(1);
                }
            };

            // Write output or print to stdout
            match args.output {
                Some(output_path) => {
                    if let Err(e) = fs::write(&output_path, &json) {
                        eprintln!(
                            "Failed to write output file '{}': {}",
                            output_path.display(),
                            e
                        );
                        process::exit(1);
                    }
                    println!(
                        "Compilation successful. Output written to '{}'",
                        output_path.display()
                    );

                    if args.verbose {
                        // Print program statistics
                        println!("\nProgram Statistics:");
                        println!(
                            "  Total instructions: {}",
                            output.program.instructions.len()
                        );
                        println!("  Entry points: {}", output.program.entry_points.len());
                        for (name, pc) in &output.program.entry_points {
                            println!("    {}: PC {}", name, pc);
                        }
                    }
                }
                None => {
                    println!("{}", json);
                }
            }
        }
        Err(e) => {
            // Print the error
            match &e {
                CompilerError::ParseErrors(diagnostics) => {
                    eprintln!("Parse errors:");
                    let error_msg = format_diagnostics(&source_text, diagnostics, true);
                    eprintln!("{}", error_msg);
                }
                CompilerError::SemanticErrors(diagnostics) => {
                    eprintln!("Semantic errors:");
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
        }
    }
}
