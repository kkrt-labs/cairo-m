use std::collections::HashMap;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use crate::Instruction;
use stwo::core::fields::qm31::QM31;

/// ABI-visible Cairo-M type description for parameters and return values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiType {
    Felt,
    Bool,
    U32,
    /// Pointer to memory containing values of `element` type
    Pointer {
        /// The element type the pointer points to
        element: Box<AbiType>,
        /// Optional length of the segment pointed to by the pointer
        len: Option<u32>,
    },
    Tuple(Vec<AbiType>),
    Struct {
        name: String,
        fields: Vec<(String, AbiType)>,
    },
    FixedSizeArray {
        element: Box<AbiType>,
        size: u32,
    },
    Unit,
}

impl AbiType {
    /// Number of field element slots this type occupies when flattened.
    pub fn size_in_slots(&self) -> usize {
        match self {
            Self::Felt | Self::Bool => 1,
            Self::U32 => 2,
            Self::Pointer { .. } => 1,
            Self::Tuple(types) => types.iter().map(|t| t.size_in_slots()).sum(),
            Self::Struct { fields, .. } => fields.iter().map(|(_, t)| t.size_in_slots()).sum(),
            Self::FixedSizeArray { size, element } => (*size as usize) * element.size_in_slots(),
            Self::Unit => 0,
        }
    }

    /// Number of call slots this type occupies when flattened.
    /// CairoM convention:
    /// - Values and aggregates are passed by values and take N slots
    /// - FixedSizeArray is passed by reference (pointer) -> 1 slot
    /// - Unit: 0 slots
    pub fn call_slot_size(ty: &Self) -> usize {
        match ty {
            Self::Felt | Self::Bool => 1,
            Self::U32 => 2,
            Self::Pointer { .. } => 1,
            Self::Tuple(ts) => ts.iter().map(Self::call_slot_size).sum(),
            Self::Struct { fields, .. } => {
                fields.iter().map(|(_, t)| Self::call_slot_size(t)).sum()
            }
            Self::FixedSizeArray { .. } => 1, // passed by pointer
            Self::Unit => 0,
        }
    }
}

/// One parameter or return value in the ABI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AbiSlot {
    /// Name of the parameter or return value (empty if the compiler had no debug name)
    pub name: String,
    /// The Cairo-M type description for this value
    pub ty: AbiType,
}

impl AbiSlot {
    /// Convenience: size in slots for this slot's type
    pub fn size_in_slots(&self) -> usize {
        self.ty.size_in_slots()
    }
}

/// Information about a function entrypoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EntrypointInfo {
    /// The program counter (instruction index) where the function starts
    pub pc: usize,
    /// Information about each parameter
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<AbiSlot>,
    /// Information about each return value
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub returns: Vec<AbiSlot>,
}

/// Public address ranges for structured access to program, input, and output data
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PublicAddressRanges {
    /// Program addresses (instructions)
    pub program: Range<u32>,
    /// Input addresses (function arguments)
    pub input: Range<u32>,
    /// Output addresses (function return values)
    pub output: Range<u32>,
}

impl PublicAddressRanges {
    /// Creates public address ranges from program length and function signature
    pub const fn new(program_length: u32, num_args: usize, num_return_values: usize) -> Self {
        let program_end = program_length;
        let input_end = program_end + num_args as u32;
        let output_end = input_end + num_return_values as u32;

        Self {
            program: 0..program_end,
            input: program_end..input_end,
            output: input_end..output_end,
        }
    }
}

/// Metadata about the compiled program
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProgramMetadata {
    /// Source file name if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,

    /// Timestamp of compilation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiled_at: Option<String>,

    /// Compiler version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_version: Option<String>,
}

/// Either an decoded instruction or a raw QM31 value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramData {
    Instruction(Instruction),
    Value(QM31),
}

impl ProgramData {
    pub fn to_qm31_vec(&self) -> Vec<QM31> {
        match self {
            Self::Instruction(instruction) => instruction.to_qm31_vec(),
            Self::Value(q) => vec![*q],
        }
    }
}

/// A compiled Cairo-M program with linear data (instructions + rodata) and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    /// Linear program data: instructions followed by rodata values
    pub data: Vec<ProgramData>,
    /// Entrypoint names mapped to their information
    pub entrypoints: HashMap<String, EntrypointInfo>,
    /// Program metadata
    pub metadata: ProgramMetadata,
}

impl From<Vec<Instruction>> for Program {
    fn from(instructions: Vec<Instruction>) -> Self {
        let data = instructions
            .into_iter()
            .map(ProgramData::Instruction)
            .collect();
        Self {
            data,
            entrypoints: HashMap::new(),
            metadata: ProgramMetadata::default(),
        }
    }
}

impl Program {
    /// Create a new program
    pub const fn new(
        data: Vec<ProgramData>,
        entrypoints: HashMap<String, EntrypointInfo>,
        metadata: ProgramMetadata,
    ) -> Self {
        Self {
            data,
            entrypoints,
            metadata,
        }
    }

    /// Get the full entrypoint information for a given function name
    pub fn get_entrypoint(&self, name: &str) -> Option<&EntrypointInfo> {
        self.entrypoints.get(name)
    }

    /// Get the total number of data entries
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the program is empty
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_type_roundtrip() {
        let types = vec![
            AbiType::Felt,
            AbiType::Bool,
            AbiType::U32,
            AbiType::Unit,
            AbiType::Pointer {
                element: Box::new(AbiType::Felt),
                len: Some(10),
            },
            AbiType::Tuple(vec![AbiType::Felt, AbiType::Bool]),
            AbiType::Struct {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), AbiType::Felt),
                    ("y".to_string(), AbiType::Felt),
                ],
            },
            AbiType::FixedSizeArray {
                element: Box::new(AbiType::U32),
                size: 5,
            },
        ];

        for ty in types {
            let json = serde_json::to_string(&ty).unwrap();
            let deserialized: AbiType = serde_json::from_str(&json).unwrap();
            assert_eq!(ty, deserialized);

            let bytes = bincode::serde::encode_to_vec(&ty, bincode::config::standard()).unwrap();
            let deserialized: AbiType =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                    .unwrap()
                    .0;
            assert_eq!(ty, deserialized);
        }
    }

    #[test]
    fn test_program_roundtrip() {
        let mut entrypoints = HashMap::new();
        entrypoints.insert(
            "main".to_string(),
            EntrypointInfo {
                pc: 0,
                params: vec![AbiSlot {
                    name: "x".to_string(),
                    ty: AbiType::Felt,
                }],
                returns: vec![AbiSlot {
                    name: "result".to_string(),
                    ty: AbiType::Bool,
                }],
            },
        );

        let program = Program {
            data: vec![
                ProgramData::Value(QM31::from_u32_unchecked(1, 2, 3, 4)),
                ProgramData::Instruction(Instruction::Ret {}),
            ],
            entrypoints,
            metadata: ProgramMetadata {
                source_file: Some("test.cm".to_string()),
                compiled_at: Some("2025-01-01".to_string()),
                compiler_version: Some("0.1.0".to_string()),
            },
        };

        // JSON roundtrip
        let json = serde_json::to_string(&program).unwrap();
        let deserialized: Program = serde_json::from_str(&json).unwrap();
        assert_eq!(program, deserialized);

        let bytes = bincode::serde::encode_to_vec(&program, bincode::config::standard()).unwrap();
        let dsr: Program = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .unwrap()
            .0;
        assert_eq!(program, dsr);
    }
}
