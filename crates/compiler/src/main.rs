//! Cairo-M Compiler
//!
//! This is the main executable for the Cairo-M compiler. It implements the complete
//! compilation pipeline from source code to executable bytecode:
//!
//! 1. Lexical Analysis: Converts source code to tokens
//! 2. Parsing: Builds an Abstract Syntax Tree (AST)
//! 3. Lowering: Converts AST to CASM instructions
//! 4. Assembly: Resolves labels and generates bytecode
//!
//! The compiler supports reading source files and outputs both human-readable
//! assembly and Cairo-compatible JSON format.

mod assembler;
mod ast;
mod casm;
mod error;
mod lexer;
mod lower_to_casm;
mod minivm;
mod parser;

use clap::Parser;
use std::fs;

use assembler::Assembler;
use lexer::lex;
use lower_to_casm::Compiler;
use minivm::MiniVm;
use parser::Parser as CairoParser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input Cairo source code or file path
    #[arg(short, long)]
    input: String,

    /// Output file path
    #[arg(short, long)]
    output: Option<String>,

    /// Run the compiled program in the VM
    #[arg(short, long)]
    run: bool,

    /// Output CASM instructions instead of JSON
    #[arg(long)]
    casm: bool,
}

/// Runs the complete compilation pipeline on the given source code.
///
/// # Arguments
/// * `input` - The source code to compile
/// * `file_name` - Name of the source file (used for error reporting)
///
/// # Returns
/// A tuple containing:
/// * The compiled bytecode
/// * The JSON representation
/// * The list of CASM instructions
///
/// # Panics
/// * If lexical analysis fails
/// * If parsing fails
/// * If compilation fails
fn run(input: &str, file_name: &str) -> (Vec<u32>, String, Vec<casm::CasmInstruction>) {
    let (tokens, errors) = lex(input, file_name);
    if errors > 0 {
        panic!("Lexing failed with {} errors", errors);
    }

    let mut parser = CairoParser::new(tokens, file_name.to_string(), input.to_string());
    let code_elements = parser.parse();

    let mut compiler = Compiler::new(code_elements);
    let casm = compiler.compile();
    let mut assembler = Assembler::new(casm.clone());
    assembler.resolve_jumps();

    let json = assembler.to_json();
    let bytecode = assembler.to_bytes();

    (bytecode, json, casm)
}

/// Main entry point for the compiler.
fn main() {
    let cli = Cli::parse();

    // Read input from file or use as direct source code
    let input = if fs::metadata(&cli.input).is_ok() {
        fs::read_to_string(&cli.input).expect("Could not read input file")
    } else {
        cli.input.clone()
    };

    let file_name = if fs::metadata(&cli.input).is_ok() {
        &cli.input
    } else {
        "stdin"
    };

    let (bytecode, json, casm) = run(&input, file_name);

    // Write output to file or stdout
    let output = if cli.casm {
        let mut casm_output = String::new();
        for (i, instruction) in casm.iter().enumerate() {
            casm_output.push_str(&format!("{} {}\n", i * 4, instruction));
        }
        casm_output
    } else {
        json
    };

    if let Some(output_path) = cli.output {
        fs::write(output_path, output).expect("Could not write output file");
    } else {
        println!("{}", output);
    }

    // Run in VM if requested
    if cli.run {
        let mut vm = MiniVm::new();
        vm.load_program(bytecode);
        vm.run();
        vm.print_mem();
    }
}
