use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_mir::{generate_mir, PrettyPrint};
use cairo_m_compiler_parser::{parse_program, SourceProgram};
use cairo_m_compiler_semantic::validate_semantics;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

/// Cairo-M compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file to compile
    #[arg(short, long)]
    input: PathBuf,

    /// Enable verbose output (shows MIR)
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    println!("Reading file: {}", args.input.display());

    match fs::read_to_string(&args.input) {
        Ok(content) => {
            // Initialize DB and source input
            let db = cairo_m_compiler_semantic::SemanticDatabaseImpl::default();
            let source = SourceProgram::new(&db, content.clone(), args.input.display().to_string());

            let parsed_program = parse_program(&db, source);

            if !parsed_program.diagnostics.is_empty() {
                for error in parsed_program.diagnostics {
                    println!("{}", build_diagnostic_message(&content, &error, true));
                }
                std::process::exit(1);
            }

            // For now, just collect diagnostics
            let semantic_diagnostics = validate_semantics(&db, &parsed_program.module, source);

            for diagnostic in semantic_diagnostics.iter() {
                println!("{}", build_diagnostic_message(&content, diagnostic, true));
            }
            if !semantic_diagnostics.is_empty() {
                std::process::exit(1);
            }

            // Generate MIR
            println!("Generating MIR...");
            match generate_mir(&db, source) {
                Some(mir_module) => {
                    if args.verbose {
                        println!("\n=== Generated MIR ===");
                        println!("{}", mir_module.pretty_print(0));
                        println!("=====================\n");
                    }

                    // Validate MIR structure
                    if let Err(validation_error) = mir_module.validate() {
                        eprintln!("MIR validation failed: {validation_error}");
                        std::process::exit(1);
                    }

                    println!("MIR generation successful!");
                    println!("Generated {} function(s)", mir_module.function_count());

                    // TODO: In Phase 4, this will be passed to code generation
                    println!("\nCompilation successful!");
                }
                None => {
                    eprintln!(
                        "Failed to generate MIR - semantic analysis errors prevent MIR generation"
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}
