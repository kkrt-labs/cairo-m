use std::path::PathBuf;
use std::{fs, process};

use cairo_m_compiler::{CompilerError, CompilerOptions, compile_from_crate, format_diagnostics};
use cairo_m_compiler_parser::{Crate, SourceFile};
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

    let cm_crate = if args.input.is_file() {
        let source_text = fs::read_to_string(&args.input).unwrap_or_else(|e| {
            eprintln!("Error reading file '{}': {}", args.input.display(), e);
            process::exit(1);
        });

        let file_path = args
            .input
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let root_dir = args.input.parent().unwrap().display().to_string();
        let source_file = SourceFile::new(&db, source_text, file_path.clone());

        Crate::new(&db, root_dir, file_path, vec![source_file])
    } else if args.input.is_dir() {
        let root_dir = args.input.display().to_string();
        let mut files = vec![];
        let mut has_main = false;

        for entry in fs::read_dir(&args.input).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "cm") {
                let source_text = fs::read_to_string(&path).unwrap_or_else(|e| {
                    eprintln!("Error reading file '{}': {}", path.display(), e);
                    process::exit(1);
                });
                let file_path = path.file_name().unwrap().to_string_lossy().to_string();
                let source_file = SourceFile::new(&db, source_text, file_path.clone());
                files.push(source_file);
                if file_path == "main.cm" {
                    has_main = true;
                }
            }
        }

        if !has_main {
            eprintln!("No main.cm found in directory '{}'", args.input.display());
            process::exit(1);
        }

        Crate::new(&db, root_dir, "main.cm".to_string(), files)
    } else {
        eprintln!(
            "Input must be a file or directory: '{}'",
            args.input.display()
        );
        process::exit(1);
    };

    let options = CompilerOptions {
        verbose: args.verbose,
    };

    // For diagnostics, we use the entry file's content (temporary, will update for multi-file)
    let entry_path = cm_crate.entry_file(&db);
    let entry_text = cm_crate
        .files(&db)
        .iter()
        .find(|f| f.file_path(&db) == entry_path)
        .map(|f| f.text(&db).clone())
        .unwrap_or_default();

    let output = compile_from_crate(&db, cm_crate, options).unwrap_or_else(|e| {
        match &e {
            CompilerError::ParseErrors(diagnostics)
            | CompilerError::SemanticErrors(diagnostics) => {
                let error_msg = format_diagnostics(&entry_text, diagnostics, true);
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
        let diagnostic_messages = format_diagnostics(&entry_text, &output.diagnostics, true);
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
