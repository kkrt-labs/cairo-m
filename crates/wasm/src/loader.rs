//! WASM Module Loader
//!
//! This module provides functionality for loading and analyzing WASM modules,
//! with graceful handling of different womir API versions.

use std::fmt::{Debug, Display};
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
    /// Converts the WASM module into a WOMIR program.
    /// This is inefficient, as it will parse the WASM module every time it is called.
    /// However we don't plan on using the WOMIR representation in the future.
    pub fn program(&self) -> Result<Program<'_, GenericIrSetting>, WasmLoadError> {
        load_wasm(GenericIrSetting, &self.bytes).map_err(|e| WasmLoadError::ParseError {
            message: e.to_string(),
        })
    }

    /// Loads a WASM module from a file.
    /// For now this just copies the bytes into the struct.
    pub fn from_file(file_path: &str) -> Result<Self, WasmLoadError> {
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

        Ok(Self { bytes })
    }
}

impl Display for WasmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let program = match self.program() {
            Ok(prog) => prog,
            Err(e) => return write!(f, "Error parsing WASM: {}", e),
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

        write!(f, "{}", output)
    }
}

impl Debug for WasmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
