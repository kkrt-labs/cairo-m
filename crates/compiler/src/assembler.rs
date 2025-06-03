use crate::casm::*;
use json;
use std::collections::HashMap;

pub struct Assembler {
    pub casm: Vec<CasmInstruction>,
    pub label_adresses: HashMap<String, i32>,
}

impl Assembler {
    pub fn new(casm: Vec<CasmInstruction>) -> Self {
        Self {
            casm,
            label_adresses: HashMap::new(),
        }
    }

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
