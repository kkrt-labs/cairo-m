mod loader;

use loader::{load_module, print_wasm_module};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <wasm_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    match load_module(filename) {
        Ok(module) => print_wasm_module(&module),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
