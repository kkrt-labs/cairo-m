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
use womir::loader::{load_wasm, FunctionProcessingStage, PartiallyParsedProgram};

#[derive(Error, Debug)]
pub enum WasmLoadError {
    #[error("WASM file not found: {path}")]
    FileNotFound { path: String },
    #[error("Failed to read WASM file: {source}")]
    IoError { source: std::io::Error },
    #[error("Failed to parse WASM file: {message}")]
    ParseError { message: String },
}

fn load_blockless_dag(wasm: &[u8]) -> wasmparser::Result<PartiallyParsedProgram<GenericIrSetting>> {
    let mut pp = load_wasm(GenericIrSetting, wasm)?;
    let mut label_gen = 0..;

    // Advance each function until BlocklessDag using iterator + collect
    let original_functions = std::mem::take(&mut pp.functions);
    pp.functions = original_functions
        .into_iter()
        .enumerate()
        .map(|(i, mut stage)| loop {
            match stage {
                FunctionProcessingStage::BlocklessDag(_) => break Ok(stage),
                other => {
                    stage = other.advance_stage(&pp.s, &pp.m, i as u32, &mut label_gen, None)?;
                }
            }
        })
        .collect::<wasmparser::Result<_>>()?;
    Ok(pp)
}

/// Module loaded by the womir crate.
/// TODO : find a way to avoid using ouroboros and #allow(clippy::future_not_send)
#[self_referencing]
pub struct BlocklessDagModule {
    wasm_binary: Vec<u8>,
    #[borrows(wasm_binary)]
    #[not_covariant]
    pub program: PartiallyParsedProgram<'this, GenericIrSetting>,
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
                load_blockless_dag(wasm_binary).map_err(|e| WasmLoadError::ParseError {
                    message: e.to_string(),
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
            for (func_idx, func) in program.functions.iter().enumerate() {
                let func_name = program
                    .m
                    .exported_functions
                    .get(&(func_idx as u32))
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| format!("func_{}", func_idx));

                output.push_str(&format!("{}:\n", func_name));
                match func {
                    FunctionProcessingStage::BlocklessDag(dag) => {
                        Self::format_nodes(&dag.nodes, &mut output, 1);
                    }
                    _ => {
                        return Err(std::fmt::Error);
                    }
                }
            }
            write!(f, "{}", output)
        })
    }
}

impl BlocklessDagModule {
    /// Recursively format nodes with proper indentation for nested structures
    fn format_nodes(
        nodes: &[womir::loader::blockless_dag::Node],
        output: &mut String,
        indent_level: usize,
    ) {
        let indent = "  ".repeat(indent_level);
        for node in nodes {
            match &node.operation {
                womir::loader::blockless_dag::Operation::Loop { sub_dag, .. } => {
                    // Format the loop node itself
                    output.push_str(&format!("{}{:?}\n", indent, node));
                    // Recursively format the sub-DAG with increased indentation
                    Self::format_nodes(&sub_dag.nodes, output, indent_level + 1);
                }
                _ => {
                    // Regular node with current indentation
                    output.push_str(&format!("{}{:?}\n", indent, node));
                }
            }
        }
    }
}

impl Debug for BlocklessDagModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_basic() {
        // Test basic loading functionality
        let result = BlocklessDagModule::from_file("tests/test_cases/add.wasm");
        assert!(result.is_ok(), "Should load add.wasm successfully");

        let module = result.unwrap();
        module.with_program(|program| assert!(!program.functions.is_empty()));
    }
}
