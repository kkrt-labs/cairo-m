use std::env;
mod assembler;
mod ast;
mod casm;
mod error;
mod lexer;
mod lower_to_casm;
mod minivm;
mod parser;

fn run(input: &str, file_name: &str) {
    let (tokens, errors) = lexer::lex(input, file_name);
    if errors > 0 {
        panic!("Lexing failed with {} errors", errors);
    }

    let mut parser = parser::Parser::new(tokens, file_name.to_string(), input.to_string());
    let code_elements = parser.parse();

    let mut compiler = lower_to_casm::Compiler::new(code_elements);
    let casm = compiler.compile();
    let mut assembler = assembler::Assembler::new(casm);
    assembler.resolve_jumps();
    for (i, instruction) in assembler.casm.clone().iter().enumerate() {
        println!("{} {}", i * 4, instruction);
    }
    println!("{}", assembler.to_json());
}

fn from_file(path: &str) {
    let contents = std::fs::read_to_string(path).expect("Could not read file.");
    run(&contents, path);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        from_file(&args[1]);
    } else {
        panic!("No file provided");
    }
}
