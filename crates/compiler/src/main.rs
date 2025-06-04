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

use std::env;
mod assembler;
mod ast;
mod casm;
mod error;
mod lexer;
mod lower_to_casm;
mod minivm;
mod parser;

use assembler::Assembler;
use lexer::lex;
use lower_to_casm::Compiler;
use minivm::MiniVm;
use parser::Parser;

/// Runs the complete compilation pipeline on the given source code.
///
/// # Arguments
/// * `input` - The source code to compile
/// * `file_name` - Name of the source file (used for error reporting)
///
/// # Panics
/// * If lexical analysis fails
/// * If parsing fails
/// * If compilation fails
fn run(input: &str, file_name: &str) {
    let (tokens, errors) = lex(input, file_name);
    if errors > 0 {
        panic!("Lexing failed with {} errors", errors);
    }

    let mut parser = Parser::new(tokens, file_name.to_string(), input.to_string());
    let code_elements = parser.parse();

    let mut compiler = Compiler::new(code_elements);
    let casm = compiler.compile();
    let mut assembler = Assembler::new(casm);
    assembler.resolve_jumps();
    for (i, instruction) in assembler.casm.clone().iter().enumerate() {
        println!("{} {}", i * 4, instruction);
    }
    println!("{}", assembler.to_json());

    let mut vm = MiniVm::new();
    vm.load_program(assembler.to_bytes());
    vm.run();
    vm.print_mem();
}

/// Reads and compiles a source file.
///
/// # Arguments
/// * `path` - Path to the source file to compile
///
/// # Panics
/// * If the file cannot be read
/// * If compilation fails
fn from_file(path: &str) {
    let contents = std::fs::read_to_string(path).expect("Could not read file.");
    run(&contents, path);
}

/// Main entry point for the compiler.
///
/// # Arguments
/// Command line arguments:
/// * First argument: Path to the source file to compile
///
/// # Panics
/// * If no source file is provided
/// * If compilation fails
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        from_file(&args[1]);
    } else {
        panic!("No file provided");
    }
}
