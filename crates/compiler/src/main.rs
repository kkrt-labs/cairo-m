use cairo_m_compiler_diagnostics::build_diagnostic_message;
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
}

fn main() {
    let args = Args::parse();
    println!("Reading file: {}", args.input.display());

    match fs::read_to_string(&args.input) {
        Ok(content) => {
            // Initialize DB and source input
            let db = cairo_m_compiler_semantic::SemanticDatabaseImpl::default();
            let source = SourceProgram::new(&db, content.clone());

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
            println!("\nCompilation successful!");
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}
