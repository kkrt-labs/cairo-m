use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler::{
    compile_project, format_diagnostics_multi_file, CompilerError, CompilerOptions,
};
use cairo_m_compiler_mir::pipeline::OptimizationLevel;
use cairo_m_project::discover_project;
use clap::Parser;
use tracing::Level;

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

    /// Optimization level (0: disabled, 1: enabled)
    #[arg(long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=1), default_value_t = 1)]
    opt_level: u8,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    }

    let db = cairo_m_compiler::create_compiler_database();

    // Discover the project
    let project = match discover_project(&args.input).unwrap_or_else(|e| {
        eprintln!("Failed to discover project: {}", e);
        process::exit(1);
    }) {
        Some(project) => project,
        None => {
            eprintln!("No Cairo-M project found at '{}'", args.input.display());
            eprintln!("Make sure there's a cairom.toml file in the project root");
            process::exit(1);
        }
    };

    let options = CompilerOptions {
        verbose: args.verbose,
        optimization_level: match args.opt_level {
            0 => OptimizationLevel::None,
            _ => OptimizationLevel::Standard,
        },
    };

    // Build a map of file paths to source text for multi-file diagnostics
    let mut source_map = std::collections::HashMap::new();
    // We'll need to read the source files for error reporting
    if let Ok(source_files) = project.source_files() {
        for file_path in source_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                source_map.insert(file_path.to_string_lossy().to_string(), content);
            }
        }
    }

    let output = compile_project(&db, project, options).unwrap_or_else(|e| {
        match &e {
            CompilerError::ParseErrors(diagnostics)
            | CompilerError::SemanticErrors(diagnostics) => {
                let error_msg = format_diagnostics_multi_file(&source_map, diagnostics, true);
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
        let diagnostic_messages =
            format_diagnostics_multi_file(&source_map, &output.diagnostics, true);
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
