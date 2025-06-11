use ariadne::{Label, Report, ReportKind, Source};
use cairo_m_compiler_parser::SourceProgram;
use cairo_m_compiler_semantic::{validate_semantics, Diagnostic};
use clap::Parser;
use salsa::Database;
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

            // Unified compilation: parsing + semantic analysis
            let semantic_diagnostics = validate_semantics(&db, source);

            if semantic_diagnostics.has_errors() {
                println!("\nCompilation errors:");
                for error in semantic_diagnostics.errors() {
                    println!("{}", build_semantic_diagnostic_message(&content, error));
                }
                std::process::exit(1);
            }

            println!("\nCompilation successful!");
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}

/// Build a formatted error message for a semantic diagnostic
fn build_semantic_diagnostic_message(source: &str, diagnostic: &Diagnostic) -> String {
    let mut write_buffer = Vec::new();
    Report::build(ReportKind::Error, ((), diagnostic.span.into_range()))
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(true), // Use color for better visibility in terminal
        )
        .with_code(3)
        .with_message(&diagnostic.message)
        .with_label(
            Label::new(((), diagnostic.span.into_range())).with_message(&diagnostic.message),
        )
        .finish()
        .write(Source::from(source), &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
