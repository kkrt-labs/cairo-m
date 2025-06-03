use crate::{ast::*, casm::*};
use json;
use std::collections::HashMap;

pub struct Assembler {
    pub casm: Vec<CasmInstruction>,
    pub instructions: Vec<Instruction>,
    pub function_adresses: HashMap<String, u64>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            casm: Vec::new(),
            instructions: Vec::new(),
            function_adresses: HashMap::new(),
        }
    }

    pub fn resolve_jumps(&mut self) {
        let mut new = Vec::new();
        let mut instruction_number = 0;
        //let mut function_adresses = HashMap::new();
        for instruction in self.casm.clone() {
            match instruction {
                CasmInstruction::Label(label) => {
                    self.function_adresses.insert(label, instruction_number);
                }
                CasmInstruction::Call(label) => {
                    instruction_number += 2;
                }
                CasmInstruction::Jmp(label) => {
                    instruction_number += 2;
                }
                CasmInstruction::JmpIfNeq(label, op) => {
                    instruction_number += 2;
                }
                _ => {
                    instruction_number += nops(instruction.clone());
                }
            }
        }
        instruction_number = 0;
        for instruction in self.casm.clone() {
            match instruction {
                CasmInstruction::Call(label) => {
                    new.push(CasmInstruction::CallRel(
                        self.function_adresses[&label] as i32 - instruction_number as i32,
                    ));
                    instruction_number += 2;
                }
                CasmInstruction::Label(label) => {}
                CasmInstruction::Jmp(label) => {
                    new.push(CasmInstruction::JmpRel(
                        self.function_adresses[&label] as i32 - instruction_number as i32,
                    ));
                    instruction_number += 2;
                }
                CasmInstruction::JmpIfNeq(label, op) => {
                    new.push(CasmInstruction::JmpIfNeqRel(
                        self.function_adresses[&label] as i32 - instruction_number as i32,
                        op,
                    ));
                    instruction_number += 2;
                }
                _ => {
                    new.push(instruction.clone());
                    instruction_number += nops(instruction);
                }
            }
        }
        self.casm = new;
    }

    pub fn build_instructions(&mut self) {
        for instruction in self.casm.clone() {
            self.instructions.push(build_instruction(instruction));
        }
    }

    pub fn to_json(&self) -> String {
        let mut data = json::JsonValue::new_object();
        data["attributes"] = json::JsonValue::new_array();
        data["builtins"] = json::JsonValue::new_array();
        data["compiler_version"] = json::JsonValue::from("0.1");
        data["data"] = json::JsonValue::new_array();
        for instruction in self.instructions.clone() {
            let (bytes, imm) = instruction.to_bytes();
            data["data"].push(format!("{:#x}", bytes));
            if let Some(imm) = imm {
                data["data"].push(format!("{:#x}", imm));
            }
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
