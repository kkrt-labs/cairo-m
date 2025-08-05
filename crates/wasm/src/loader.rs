//! WASM Module Loader
//!
//! This module provides functionality for loading and analyzing WASM modules,
//! with graceful handling of different womir API versions.

use std::fs;
use std::path::Path;

use thiserror::Error;

use womir::generic_ir::GenericIrSetting;
use womir::loader::{load_wasm, Program};

#[derive(Error, Debug)]
pub enum WasmLoadError {
    #[error("WASM file not found: {path}")]
    FileNotFound { path: String },
    #[error("Failed to read WASM file: {source}")]
    IoError { source: std::io::Error },
    #[error("Failed to parse WASM file: {message}")]
    ParseError { message: String },
}

/// A WASM module that lazily parses when first accessed
pub struct WasmModule {
    bytes: Vec<u8>,
}

impl WasmModule {
    /// Get the parsed program (parsed on first access)
    pub fn program(&self) -> Result<Program<'_, GenericIrSetting>, WasmLoadError> {
        load_wasm(GenericIrSetting, &self.bytes).map_err(|e| WasmLoadError::ParseError {
            message: e.to_string(),
        })
    }
}

/// Load a WASM module from a file path
/// Returns a WasmModule that parses lazily when accessed
pub fn load_module(file_path: &str) -> Result<WasmModule, WasmLoadError> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(WasmLoadError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    let bytes = fs::read(path).map_err(|e| WasmLoadError::IoError { source: e })?;

    // Validate the bytes can be parsed (early error detection)
    load_wasm(GenericIrSetting, &bytes).map_err(|e| WasmLoadError::ParseError {
        message: e.to_string(),
    })?;

    Ok(WasmModule { bytes })
}

/// Format a WOMIR module as a string
pub fn format_wasm_module(module: &WasmModule) -> String {
    let program = match module.program() {
        Ok(prog) => prog,
        Err(e) => return format!("Error parsing WASM: {}", e),
    };

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
pub fn print_wasm_module(module: &WasmModule) {
    println!("{}", format_wasm_module(module));
}
