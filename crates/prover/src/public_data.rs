use serde::{Deserialize, Serialize};

use crate::adapter::{Instructions, VmRegisters};

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicData {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
}

impl PublicData {
    pub fn new(input: &Instructions) -> Self {
        Self {
            initial_registers: input.initial_registers.clone(),
            final_registers: input.final_registers.clone(),
        }
    }
}
