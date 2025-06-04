//! Cairo-M Assembler Module
//!
//! This module implements the assembler for Cairo-M assembly code. The assembler
//! converts CASM instructions into executable bytecode and handles label resolution
//! for jumps and function calls.

use crate::casm::*;
use json;
use std::collections::HashMap;

/// The main assembler struct for converting CASM instructions to bytecode.
///
/// The assembler maintains the list of CASM instructions and a mapping of
/// labels to their addresses in the final bytecode.
pub struct Assembler {
    /// The list of CASM instructions to assemble
    pub casm: Vec<CasmInstruction>,
    /// Mapping from label names to their addresses in the bytecode
    pub label_adresses: HashMap<String, i32>,
}

impl Assembler {
    /// Creates a new assembler instance with the given CASM instructions.
    pub fn new(casm: Vec<CasmInstruction>) -> Self {
        Self {
            casm,
            label_adresses: HashMap::new(),
        }
    }

    /// Resolves all jumps and function calls to their absolute addresses.
    ///
    /// This method performs two passes over the code:
    /// 1. First pass: Collects addresses of all labels
    /// 2. Second pass: Converts relative jumps and calls to absolute addresses
    ///
    /// The following transformations are performed:
    /// - `CallLabel` -> `CallAbs` with resolved address
    /// - `JmpLabel` -> `JmpAbs` with resolved address
    /// - `JmpLabelIfNeq` -> `JmpAbsIfNeq` with resolved address
    pub fn resolve_jumps(&mut self) {
        let mut new = Vec::new();
        let mut instruction_number = 0;

        // First pass to get the adresses of the labels
        for instruction in self.casm.clone() {
            match instruction.instruction_type {
                CasmInstructionType::Label => {
                    self.label_adresses
                        .insert(instruction.label.clone().unwrap(), instruction_number);
                }
                _ => {
                    instruction_number += 4;
                }
            }
        }

        // Second pass to resolve the jumps
        for instruction in self.casm.clone() {
            match instruction.instruction_type {
                CasmInstructionType::CallLabel => {
                    new.push(CasmInstruction {
                        instruction_type: CasmInstructionType::CallAbs,
                        label: instruction.label.clone(),
                        arg0: self.label_adresses[&instruction.label.clone().unwrap()],
                        arg1: instruction.arg0,
                        arg2: 0,
                    });
                }
                CasmInstructionType::Label => {}
                CasmInstructionType::JmpLabel => {
                    new.push(CasmInstruction {
                        instruction_type: CasmInstructionType::JmpAbs,
                        label: instruction.label.clone(),
                        arg0: self.label_adresses[&instruction.label.clone().unwrap()],
                        arg1: 0,
                        arg2: 0,
                    });
                }
                CasmInstructionType::JmpLabelIfNeq => {
                    new.push(CasmInstruction {
                        instruction_type: CasmInstructionType::JmpAbsIfNeq,
                        label: instruction.label.clone(),
                        arg0: self.label_adresses[&instruction.label.clone().unwrap()],
                        arg1: instruction.arg1,
                        arg2: 0,
                    });
                }
                _ => {
                    new.push(instruction.clone());
                }
            }
        }
        self.casm = new;
    }

    /// Converts the CASM instructions to bytecode format.
    ///
    /// Each instruction is converted to four 32-bit words:
    /// - First word: Opcode
    /// - Remaining words: Arguments (encoded as signed offsets from 0x8000)
    ///
    /// # Returns
    /// A vector of 32-bit words representing the bytecode
    pub fn to_bytes(&self) -> Vec<u32> {
        let mut bytes = Vec::new();
        for instruction in self.casm.clone() {
            let (opcode, arg0, arg1, arg2) = instruction.to_bytes();
            bytes.push(opcode);
            bytes.push(arg0);
            bytes.push(arg1);
            bytes.push(arg2);
        }
        bytes
    }

    /// Generates a Cairo-compatible JSON representation of the program.
    ///
    /// The JSON output includes:
    /// - Program bytecode
    /// - Label addresses and types
    /// - Compiler version
    /// - Other Cairo-specific metadata
    ///
    /// # Returns
    /// A JSON string containing the program representation
    pub fn to_json(&self) -> String {
        let mut data = json::JsonValue::new_object();
        data["attributes"] = json::JsonValue::new_array();
        data["builtins"] = json::JsonValue::new_array();
        data["compiler_version"] = json::JsonValue::from("0.1");
        data["data"] = json::JsonValue::from(self.to_bytes());
        data["hints"] = json::JsonValue::new_object();
        data["identifiers"] = json::JsonValue::new_object();
        for (label, address) in self.label_adresses.clone() {
            let label2 = format!("__main__.{}", label);
            data["identifiers"][label2.clone()] = json::JsonValue::new_object();
            data["identifiers"][label2.clone()]["decorators"] = json::JsonValue::new_array();
            data["identifiers"][label2.clone()]["pc"] = json::JsonValue::from(address);
            data["identifiers"][label2.clone()]["type"] = json::JsonValue::from("function");
        }
        data["main_scope"] = json::JsonValue::from("__main__");
        data["prime"] = json::JsonValue::from("0x7fffffff");
        data["reference_manager"] = json::JsonValue::new_object();
        data["reference_manager"]["references"] = json::JsonValue::new_array();
        data.to_string()
    }
}
