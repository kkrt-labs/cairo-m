mod flattening;
mod loader;

use flattening::DagToMir;
use loader::BlocklessDagModule;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <wasm_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    match BlocklessDagModule::from_file(filename) {
        Ok(module) => match DagToMir::new(module).to_mir() {
            Ok(mir) => {
                println!("{mir:?}");
            }
            Err(e) => {
                eprintln!("Error converting to MIR: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error loading WASM module: {}", e);
            std::process::exit(1);
        }
    }
}
