use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler::project_discovery::{
    ProjectDiscoveryConfig, create_crate_from_discovery, discover_project_files, find_project_root,
};
use cairo_m_compiler::{
    CompilerError, CompilerOptions, compile_from_crate, format_diagnostics_multi_file,
};
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

    let db = cairo_m_compiler::create_compiler_database();

    // Use project discovery to find the project root and all source files
    let project_root = if args.input.is_file() {
        // For single file, find the project root starting from the file
        find_project_root(&args.input).unwrap_or_else(|| {
            // If no project root found, use the file's parent directory
            args.input.parent().unwrap().to_path_buf()
        })
    } else if args.input.is_dir() {
        args.input.clone()
    } else {
        eprintln!(
            "Input must be a file or directory: '{}'",
            args.input.display()
        );
        process::exit(1);
    };

    // Discover all project files
    let config = ProjectDiscoveryConfig::default();
    let discovered = discover_project_files(&project_root, &config).unwrap_or_else(|e| {
        eprintln!("Failed to discover project files: {}", e);
        process::exit(1);
    });

    // Create the crate from discovered files
    let cm_crate = create_crate_from_discovery(&db, &discovered).unwrap_or_else(|e| {
        eprintln!("Failed to read project files: {}", e);
        process::exit(1);
    });

    let options = CompilerOptions {
        verbose: args.verbose,
    };

    // Build a map of file paths to source text for multi-file diagnostics
    let mut source_map = std::collections::HashMap::new();
    for source_file in cm_crate.files(&db) {
        let file_path = source_file.file_path(&db).to_string();
        let source_text = source_file.text(&db).to_string();
        source_map.insert(file_path, source_text);
    }

    let output = compile_from_crate(&db, cm_crate, options).unwrap_or_else(|e| {
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
