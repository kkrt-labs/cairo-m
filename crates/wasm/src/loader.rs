//! WASM Module Loader
//!
//! This module provides functionality for loading the WOMIR BlockLess DAG representation of a WASM module.

#![allow(clippy::future_not_send)] // Allow this lint for self-referencing structs in this module

use std::fmt::{Debug, Display};
use std::fs;
use std::path::Path;

use thiserror::Error;

use ouroboros::self_referencing;

use womir::generic_ir::GenericIrSetting;
use womir::loader::{load_wasm_unflattened, UnflattenedProgram};

#[derive(Error, Debug)]
pub enum WasmLoadError {
    #[error("WASM file not found: {path}")]
    FileNotFound { path: String },
    #[error("Failed to read WASM file: {source}")]
    IoError { source: std::io::Error },
    #[error("Failed to parse WASM file: {message}")]
    ParseError { message: String },
}

/// Module loaded by the womir crate.
/// TODO : find a way to avoid using ouroboros and #allow(clippy::future_not_send)
#[self_referencing]
pub struct BlocklessDagModule {
    wasm_binary: Vec<u8>,
    #[borrows(wasm_binary)]
    #[covariant]
    pub program: UnflattenedProgram<'this, GenericIrSetting>,
}

impl BlocklessDagModule {
    /// Loads a WASM module from a file and converts it to the WOMIR BlockLess DAG representation.
    pub fn from_file(file_path: &str) -> Result<Self, WasmLoadError> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(WasmLoadError::FileNotFound {
                path: file_path.to_string(),
            });
        }

        let bytes = fs::read(path).map_err(|e| WasmLoadError::IoError { source: e })?;

        BlocklessDagModuleTryBuilder {
            wasm_binary: bytes,
            program_builder: |wasm_binary: &Vec<u8>| {
                load_wasm_unflattened(GenericIrSetting, wasm_binary).map_err(|e| {
                    WasmLoadError::ParseError {
                        message: e.to_string(),
                    }
                })
            },
        }
        .try_build()
    }
}

impl Display for BlocklessDagModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with_program(|program| {
            let mut output = String::new();
            for (func_idx, func) in program.functions.iter() {
                let func_name = program
                    .c
                    .exported_functions
                    .get(func_idx)
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| format!("func_{}", func_idx));

                output.push_str(&format!("{}:\n", func_name));
                for node in func.nodes.iter() {
                    output.push_str(&format!("  {:?}\n", node));
                }
            }
            write!(f, "{}", output)
        })
    }
}

impl Debug for BlocklessDagModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
