use cairo_m_compiler_codegen::generate_json;
use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_mir::generate_mir;
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

    /// Output file to write the compiled program to
    #[arg(short, long)]
    output: Option<PathBuf>,

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

            let generated_mir = generate_mir(&db, source);

            if generated_mir.is_none() {
                eprintln!("Failed to generate MIR");
                std::process::exit(1);
            }

            let mir_module = generated_mir.unwrap();

            let generated_json = generate_json(&mir_module);

            if let Err(e) = generated_json {
                eprintln!("Failed to generate JSON: {e}");
                std::process::exit(1);
            }

            let generated_json = generated_json.unwrap();

            if let Some(output_path) = args.output {
                fs::write(output_path, generated_json).unwrap();
            } else {
                println!("Generated JSON: {generated_json}");
            }
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}
