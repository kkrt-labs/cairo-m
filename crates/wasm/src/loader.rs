//! WASM Module Loader
//!
//! This module provides functionality for loading and analyzing WASM modules,
//! with graceful handling of different womir API versions.

use std::fs;
use std::path::Path;

use womir::generic_ir::GenericIrSetting;
use womir::loader::{load_wasm, Program};

/// Load a WASM module from a file path
/// For now this just uses the default load_wasm function from the WOMIR loader crate
pub fn load_module(file_path: &str) -> Result<Program<GenericIrSetting>, String> {
    // Verify the file exists
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("WASM file not found: {}", file_path));
    }

    let bytes = fs::read(path).map_err(|e| format!("Failed to read WASM file: {}", e))?;

    // Leak the bytes so they live for the entire program duration (quick fix for lifetime issues)
    let leaked_bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let program = load_wasm(GenericIrSetting, leaked_bytes)
        .map_err(|e| format!("Failed to parse WASM file: {}", e))?;

    Ok(program)
}

/// Format a WOMIR program as a string
pub fn format_womir_program(program: &Program<GenericIrSetting>) -> String {
    let mut output = String::new();

    for func in program.functions.iter() {
        // Get function name - check if exported, otherwise use default
        let func_name = program
            .c
            .exported_functions
            .get(&func.func_idx)
            .map(|name| name.to_string())
            .unwrap_or_else(|| format!("func_{}", func.func_idx));

        output.push_str(&format!(
            "Function: {} ({} directives)\n",
            func_name,
            func.directives.len()
        ));

        for (i, directive) in func.directives.iter().enumerate() {
            output.push_str(&format!("  {:3}: {}\n", i, directive));
        }
        output.push('\n'); // Empty line between functions
    }

    // Remove the trailing newline if there were functions
    if !output.is_empty() {
        output.pop(); // Remove the last newline
    }

    output
}

/// Print a WOMIR program to the console
pub fn print_womir_program(program: &Program<GenericIrSetting>) {
    println!("{}", format_womir_program(program));
}
