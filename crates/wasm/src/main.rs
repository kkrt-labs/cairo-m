mod loader;

use loader::WasmModule;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <wasm_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    match WasmModule::from_file(filename) {
        Ok(module) => println!("{module:?}"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
