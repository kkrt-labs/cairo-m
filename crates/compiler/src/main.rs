use std::fs;
use std::path::PathBuf;

use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_parser::{parse_program, SourceProgram};
use clap::Parser;

mod db;
use db::CompilerDatabase;

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
            // Initialize unified compiler database
            let db = CompilerDatabase::new();
            let source = SourceProgram::new(&db, content.clone(), args.input.display().to_string());

            let parsed_program = parse_program(&db, source);

            if !parsed_program.diagnostics.is_empty() {
                for error in parsed_program.diagnostics {
                    println!("{}", build_diagnostic_message(&content, &error, true));
                }
                std::process::exit(1);
            }

            // Validate semantics using the tracked query
            let semantic_diagnostics =
                cairo_m_compiler_semantic::db::validate_semantics(&db, source);

            for diagnostic in semantic_diagnostics.iter() {
                println!("{}", build_diagnostic_message(&content, diagnostic, true));
            }
            if !semantic_diagnostics.errors().is_empty() {
                std::process::exit(1);
            }

            // Generate MIR using the tracked query
            let generated_mir = cairo_m_compiler_mir::db::generate_mir(&db, source);

            if generated_mir.is_none() {
                eprintln!("Failed to generate MIR");
                std::process::exit(1);
            }

            let mir_module = generated_mir.unwrap();

            if args.verbose {
                println!("\nGenerated MIR:\n{mir_module:#?}");
            }

            // Compile using the tracked query
            let compiled_program = cairo_m_compiler_codegen::db::compile_module(&db, source);

            if let Err(ref e) = compiled_program {
                eprintln!("Failed to generate program: {e}");
                std::process::exit(1);
            }

            let program = compiled_program.unwrap();

            if args.verbose {
                // Print program statistics
                println!("\nProgram Statistics:");
                println!("  Total instructions: {}", program.instructions.len());
                println!("  Entry points: {}", program.entry_points.len());
                for (name, pc) in &program.entry_points {
                    println!("    {name}: PC {pc}");
                }
            }

            let generated_json = sonic_rs::to_string_pretty(&program).unwrap();

            if let Some(output_path) = args.output {
                fs::write(output_path, generated_json).unwrap();
            } else {
                println!("Generated JSON: {generated_json}");
            }
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}
