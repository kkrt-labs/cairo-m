mod flattening;
mod loader;

use flattening::WasmModuleToMir;
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
        Ok(module) => {
            let mir = WasmModuleToMir::new(module).to_mir().unwrap();
            println!("{mir:?}");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
