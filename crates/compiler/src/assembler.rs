use crate::casm::*;
use json;
use std::collections::HashMap;

pub struct Assembler {
    pub casm: Vec<CasmInstruction>,
    pub function_adresses: HashMap<String, u64>,
}

impl Assembler {
    pub fn new(casm: Vec<CasmInstruction>) -> Self {
        Self {
            casm,
            function_adresses: HashMap::new(),
        }
    }

    pub fn resolve_jumps(&mut self) {
        let mut new = Vec::new();
        let mut instruction_number = 0;

        let mut label_adresses = HashMap::new();

        // First pass to get the adresses of the labels
        for instruction in self.casm.clone() {
            match instruction.instruction_type {
                CasmInstructionType::Label => {
                    label_adresses.insert(instruction.label, instruction_number);
                }
                _ => {
                    instruction_number += 1;
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
                        arg0: label_adresses[&instruction.label.clone()] as i32,
                        arg1: 0,
                        arg2: 0,
                    });
                }
                CasmInstructionType::Label => {}
                CasmInstructionType::JmpLabel => {
                    new.push(CasmInstruction {
                        instruction_type: CasmInstructionType::JmpAbs,
                        label: instruction.label.clone(),
                        arg0: label_adresses[&instruction.label.clone()] as i32,
                        arg1: 0,
                        arg2: 0,
                    });
                }
                CasmInstructionType::JmpLabelIfNeq => {
                    new.push(CasmInstruction {
                        instruction_type: CasmInstructionType::JmpAbsIfNeq,
                        label: instruction.label.clone(),
                        arg0: label_adresses[&instruction.label.clone()] as i32,
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

    pub fn to_json(&self) -> String {
        let mut data = json::JsonValue::new_object();
        data["attributes"] = json::JsonValue::new_array();
        data["builtins"] = json::JsonValue::new_array();
        data["compiler_version"] = json::JsonValue::from("0.1");
        data["data"] = json::JsonValue::new_array();
        for instruction in self.casm.clone() {
            let (opcode, arg0, arg1, arg2) = instruction.to_bytes();
            data["data"].push(format!("{:#x}", opcode));
            data["data"].push(format!("{:#x}", arg0));
            data["data"].push(format!("{:#x}", arg1));
            data["data"].push(format!("{:#x}", arg2));
        }
        data["hints"] = json::JsonValue::new_object();
        data["identifiers"] = json::JsonValue::new_object();
        for (label, address) in self.function_adresses.clone() {
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
