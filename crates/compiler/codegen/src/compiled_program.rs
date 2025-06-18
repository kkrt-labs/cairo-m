use std::collections::HashMap;

use num_traits::Zero;
use serde::{Deserializer, Serializer};
use sonic_rs::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;

/// Represents a fully compiled Cairo-M program ready for execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledProgram {
    /// The compiled instructions
    pub instructions: Vec<CompiledInstruction>,

    /// Entry points mapping function names to instruction indices
    pub entrypoints: HashMap<String, usize>,

    /// Metadata about the compilation
    #[serde(default)]
    pub metadata: ProgramMetadata,
}

/// A compiled instruction with all operands resolved
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledInstruction {
    /// The opcode for this instruction
    pub opcode: u32,

    /// The operands for this instruction (offsets and immediate)
    /// Format: [off0, off1, off2] where the second operand can be an immediate value or an offset
    pub operands: Vec<M31>,
}

/// Metadata about the compiled program
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgramMetadata {
    /// Source file name if available
    pub source_file: Option<String>,

    /// Timestamp of compilation
    pub compiled_at: Option<String>,

    /// Compiler version
    pub compiler_version: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CompiledProgram {
    /// Create a new compiled program
    pub fn new(
        instructions: Vec<CompiledInstruction>,
        entrypoints: HashMap<String, usize>,
    ) -> Self {
        Self {
            instructions,
            entrypoints,
            metadata: ProgramMetadata::default(),
        }
    }

    /// Get the entry point for a function
    pub fn get_entrypoint(&self, function_name: &str) -> Option<usize> {
        self.entrypoints.get(function_name).copied()
    }

    /// Get instruction at a specific program counter
    pub fn get_instruction(&self, pc: usize) -> Option<&CompiledInstruction> {
        self.instructions.get(pc)
    }

    /// Total number of instructions
    pub const fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
}

impl CompiledInstruction {
    /// Create a new compiled instruction
    pub const fn new(opcode: u32, operands: Vec<M31>) -> Self {
        Self { opcode, operands }
    }

    /// Get operand 0 (fp-relative source 1)
    pub fn op0(&self) -> Option<M31> {
        self.operands.first().copied()
    }

    /// Get operand 1 (fp-relative source 2)
    pub fn op1(&self) -> Option<M31> {
        self.operands.get(1).copied()
    }

    /// Get operand 2 (fp-relative destination)
    pub fn op2(&self) -> Option<M31> {
        self.operands.get(2).copied()
    }
}

impl Serialize for CompiledInstruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Include opcode as the first element
        let mut values = vec![M31::from(self.opcode)];

        // Add the three offsets
        values.push(self.op0().unwrap_or_else(Zero::zero));
        values.push(self.op1().unwrap_or_else(Zero::zero));
        values.push(self.op2().unwrap_or_else(Zero::zero));

        let hex_values: Vec<String> = values
            .iter()
            .map(|&val| format!("0x{:08x}", val.0))
            .collect();
        hex_values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CompiledInstruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_vec: Vec<String> = Vec::deserialize(deserializer)?;

        // Ensure we have _exactly_ 4 elements (opcode + 3 offsets)
        if hex_vec.len() != 4 {
            return Err(serde::de::Error::custom(
                "Expected at least 4 elements in instruction array",
            ));
        }

        // Parse opcode from hex
        let opcode = u32::from_str_radix(hex_vec[0].trim_start_matches("0x"), 16)
            .map_err(serde::de::Error::custom)?;

        // Parse offsets from hex
        let op0 = parse_hex_offset(&hex_vec[1])
            .ok_or_else(|| serde::de::Error::custom("Failed to parse off0"))?;
        let op1 = parse_hex_offset(&hex_vec[2])
            .ok_or_else(|| serde::de::Error::custom("Failed to parse off1"))?;
        let op2 = parse_hex_offset(&hex_vec[3])
            .ok_or_else(|| serde::de::Error::custom("Failed to parse off2"))?;

        let operands = vec![op0, op1, op2];

        Ok(Self::new(opcode, operands))
    }
}

/// Helper function to parse hex offset back to M31
fn parse_hex_offset(hex_str: &str) -> Option<M31> {
    // Parse the hex string (removing 0x prefix if present)
    let hex = hex_str.trim_start_matches("0x");
    let u32_value = u32::from_str_radix(hex, 16).ok();
    u32_value.map(M31::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiled_program_serialization() {
        let mut entrypoints = HashMap::new();

        let instructions = vec![
            CompiledInstruction::new(
                6,
                vec![M31::zero(), M31::zero(), M31::from(-1), M31::from(42)],
            ), // store imm
            CompiledInstruction::new(15, vec![]), // ret
        ];

        entrypoints.insert("main".to_string(), 0);
        let program = CompiledProgram::new(instructions, entrypoints);

        // Test serialization round-trip
        let json = sonic_rs::to_string_pretty(&program).unwrap();
        println!("Serialized JSON:\n{json}");
        let deserialized: CompiledProgram = sonic_rs::from_str(&json).unwrap();

        assert_eq!(program.instructions.len(), deserialized.instructions.len());
        assert_eq!(program.entrypoints, deserialized.entrypoints);
    }
}
