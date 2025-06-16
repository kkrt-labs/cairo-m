use serde::de::Deserializer;
use serde::ser::Serializer;
use sonic_rs::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::CasmInstruction;

#[derive(Debug, Serialize, Deserialize)]
pub struct Program {
    pub data: Vec<CasmInstruction>,
    pub function_addresses: HashMap<String, usize>,
}

impl Serialize for CasmInstruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CasmInstruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_vec: Vec<String> = Vec::deserialize(deserializer)?;
        let opcode = u32::from_str_radix(&hex_vec[0], 16).map_err(serde::de::Error::custom)?;
        let off0 = hex_vec[1].parse::<i32>().ok();
        let off1 = hex_vec[2].parse::<i32>().ok();
        let off2 = hex_vec[3].parse::<i32>().ok();
        let mut instruction = CasmInstruction::new(opcode);
        if let Some(off0) = off0 {
            instruction = instruction.with_off0(off0);
        }
        if let Some(off1) = off1 {
            instruction = instruction.with_off1(off1);
        }
        if let Some(off2) = off2 {
            instruction = instruction.with_off2(off2);
        }
        Ok(instruction)
    }
}
