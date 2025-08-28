use std::collections::HashMap;
use std::ops::Range;

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::Instruction;

/// ABI-visible Cairo-M type description for parameters and return values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiType {
    Felt,
    Bool,
    U32,
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

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A compiled Cairo-M program with instructions and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    /// The program instructions
    pub instructions: Vec<Instruction>,
    /// Entrypoint names mapped to their information
    pub entrypoints: HashMap<String, EntrypointInfo>,
    /// Program metadata
    pub metadata: ProgramMetadata,
}

// Manual serde for AbiType to preserve a stable {kind: ...} shape while avoiding serde's
// internally-tagged enum machinery (which triggers trait-solver recursion on our toolchain).
impl Serialize for AbiType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Felt => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "Felt")?;
                map.end()
            }
            Self::Bool => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "Bool")?;
                map.end()
            }
            Self::U32 => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "U32")?;
                map.end()
            }
            Self::Tuple(elements) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("kind", "Tuple")?;
                map.serialize_entry("elements", elements)?;
                map.end()
            }
            Self::Struct { name, fields } => {
                #[derive(Serialize)]
                struct FieldSer<'a> {
                    name: &'a str,
                    #[serde(rename = "ty")]
                    ty: &'a AbiType,
                }
                let fields_ser: Vec<FieldSer> = fields
                    .iter()
                    .map(|(n, t)| FieldSer {
                        name: n.as_str(),
                        ty: t,
                    })
                    .collect();

                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("kind", "Struct")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("fields", &fields_ser)?;
                map.end()
            }
            Self::FixedSizeArray { element, size } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("kind", "FixedSizeArray")?;
                map.serialize_entry("element", element.as_ref())?;
                map.serialize_entry("size", size)?;
                map.end()
            }
            Self::Unit => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "Unit")?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for AbiType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AbiTypeVisitor;

        impl<'de> Visitor<'de> for AbiTypeVisitor {
            type Value = AbiType;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "ABI type object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut kind: Option<String> = None;
                let mut elements: Option<Vec<AbiType>> = None;
                let mut name: Option<String> = None;
                let mut fields: Option<Vec<(String, AbiType)>> = None;
                let mut element: Option<AbiType> = None;
                let mut size: Option<u32> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => kind = Some(map.next_value()?),
                        "elements" => elements = Some(map.next_value()?),
                        "name" => name = Some(map.next_value()?),
                        "fields" => {
                            #[derive(Deserialize)]
                            struct FieldDe {
                                name: String,
                                #[serde(rename = "ty")]
                                ty: AbiType,
                            }
                            let items: Vec<FieldDe> = map.next_value()?;
                            fields = Some(items.into_iter().map(|f| (f.name, f.ty)).collect());
                        }
                        "element" => element = Some(map.next_value()?),
                        "size" => size = Some(map.next_value()?),
                        _ => {
                            return Err(de::Error::unknown_field(
                                key.as_str(),
                                &[
                                    "kind", "pointee", "elements", "name", "fields", "element",
                                    "size",
                                ],
                            ));
                        }
                    }
                }

                let kind = kind.ok_or_else(|| de::Error::missing_field("kind"))?;
                match kind.as_str() {
                    "Felt" => Ok(AbiType::Felt),
                    "Bool" => Ok(AbiType::Bool),
                    "U32" => Ok(AbiType::U32),
                    "Tuple" => Ok(AbiType::Tuple(elements.unwrap_or_default())),
                    "Struct" => Ok(AbiType::Struct {
                        name: name.ok_or_else(|| de::Error::missing_field("name"))?,
                        fields: fields.unwrap_or_default(),
                    }),
                    "FixedSizeArray" => Ok(AbiType::FixedSizeArray {
                        element: Box::new(
                            element.ok_or_else(|| de::Error::missing_field("element"))?,
                        ),
                        size: size.ok_or_else(|| de::Error::missing_field("size"))?,
                    }),
                    "Unit" => Ok(AbiType::Unit),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &[
                            "Felt", "Bool", "U32", "Pointer", "Tuple", "Struct", "Array", "Unit",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(AbiTypeVisitor)
    }
}

impl From<Vec<Instruction>> for Program {
    fn from(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            entrypoints: HashMap::new(),
            metadata: ProgramMetadata::default(),
        }
    }
}

impl Program {
    /// Create a new program
    pub const fn new(
        instructions: Vec<Instruction>,
        entrypoints: HashMap<String, EntrypointInfo>,
        metadata: ProgramMetadata,
    ) -> Self {
        Self {
            instructions,
            entrypoints,
            metadata,
        }
    }

    /// Get the full entrypoint information for a given function name
    pub fn get_entrypoint(&self, name: &str) -> Option<&EntrypointInfo> {
        self.entrypoints.get(name)
    }

    /// Get the total number of instructions
    pub const fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if the program is empty
    pub const fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
