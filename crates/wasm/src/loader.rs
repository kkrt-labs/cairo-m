//! WASM Module Loader
//!
//! This module provides functionality for loading the WOMIR BlockLess DAG representation of a WASM module.

use std::fmt::{Debug, Display};

use thiserror::Error;
use womir::generic_ir::GenericIrSetting;
use womir::loader::{FunctionProcessingStage, PartiallyParsedProgram, load_wasm};

#[derive(Error, Debug)]
pub enum WasmLoadError {
    #[error("Failed to read WASM file: {source}")]
    IoError { source: std::io::Error },
    #[error("Failed to parse WASM file: {message}")]
    ParseError { message: String },
}

/// Module loaded by the womir crate.
pub struct BlocklessDagModule<'a>(pub PartiallyParsedProgram<'a, GenericIrSetting>);

impl<'a> BlocklessDagModule<'a> {
    /// Loads the blockless DAG representation of a WASM file.
    pub fn from_bytes(wasm_file: &'a [u8]) -> Result<Self, WasmLoadError> {
        let mut pp =
            load_wasm(GenericIrSetting, wasm_file).map_err(|e| WasmLoadError::ParseError {
                message: e.to_string(),
            })?;
        let mut label_gen = 0..;

        // Advance each function until BlocklessDag using iterator + collect
        let original_functions = std::mem::take(&mut pp.functions);
        pp.functions = original_functions
            .into_iter()
            .enumerate()
            .map(|(i, mut stage)| {
                loop {
                    match stage {
                        FunctionProcessingStage::BlocklessDag(_) => break Ok(stage),
                        other => {
                            stage = other.advance_stage(
                                &pp.s,
                                &pp.m,
                                i as u32,
                                &mut label_gen,
                                None,
                            )?;
                        }
                    }
                }
            })
            .collect::<wasmparser::Result<_>>()
            .map_err(|e| WasmLoadError::ParseError {
                message: e.to_string(),
            })?;

        Ok(BlocklessDagModule(pp))
    }

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

impl<'a> Display for BlocklessDagModule<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let program = &self.0;
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
    }
}

impl<'a> Debug for BlocklessDagModule<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use wat::parse_file;

    use super::*;

    #[test]
    fn test_loader_basic() {
        // Test basic loading functionality
        let wasm_bytes = parse_file("tests/test_cases/i32_arithmetic.wat").unwrap();
        let result = BlocklessDagModule::from_bytes(&wasm_bytes);
        assert!(result.is_ok(), "Should load add.wasm successfully");

        let module = result.unwrap();
        assert!(!module.0.functions.is_empty());
    }
}
