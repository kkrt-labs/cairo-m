//! # MIR Type System
//!
//! This module defines a simplified type system for MIR that doesn't depend on
//! Salsa database lifetimes. It provides essential type information for MIR
//! optimizations and code generation while remaining self-contained.

use cairo_m_compiler_semantic::types::{TypeData, TypeId};
use cairo_m_compiler_semantic::SemanticDb;

/// A simplified type representation for MIR
///
/// This is a lifetime-free representation of types that can be stored
/// alongside MIR values. It contains enough information for basic type
/// checking and optimization within the MIR layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirType {
    /// The fundamental felt type
    Felt,

    /// Boolean type (represented as felt internally)
    Bool,

    /// 32-bit unsigned integer type
    U32,

    /// Pointer to another type
    Pointer(Box<MirType>),

    /// Tuple type with element types
    Tuple(Vec<MirType>),

    /// Struct type with field layout information
    /// This contains the struct name and ordered field information for layout calculations
    Struct {
        name: String,
        fields: Vec<(String, MirType)>,
    },

    /// Function type with parameter and return types
    Function {
        params: Vec<MirType>,
        return_type: Box<MirType>,
    },

    /// Unit type (no value)
    Unit,

    /// Error type for recovery
    Error,

    /// Unknown type (for incomplete analysis)
    Unknown,
}

impl MirType {
    /// Creates a felt type
    pub const fn felt() -> Self {
        Self::Felt
    }

    /// Creates a boolean type
    pub const fn bool() -> Self {
        Self::Bool
    }

    /// Creates a u32 type
    pub const fn u32() -> Self {
        Self::U32
    }

    /// Creates a pointer type
    pub fn pointer(inner: Self) -> Self {
        Self::Pointer(Box::new(inner))
    }

    /// Creates a tuple type
    pub const fn tuple(types: Vec<Self>) -> Self {
        Self::Tuple(types)
    }

    /// Creates a struct type with field layout information
    pub const fn struct_type(name: String, fields: Vec<(String, Self)>) -> Self {
        Self::Struct { name, fields }
    }

    /// Creates a simple struct type without field information (for compatibility)
    pub const fn simple_struct_type(name: String) -> Self {
        Self::Struct {
            name,
            fields: Vec::new(),
        }
    }

    /// Creates a function type
    pub fn function(params: Vec<Self>, return_type: Self) -> Self {
        Self::Function {
            params,
            return_type: Box::new(return_type),
        }
    }

    /// Creates a unit type
    pub const fn unit() -> Self {
        Self::Unit
    }

    /// Creates an error type
    pub const fn error() -> Self {
        Self::Error
    }

    /// Creates an unknown type
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    /// Returns true if this is a numeric type
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Felt | Self::Bool | Self::U32)
    }

    /// Returns true if this is a pointer type
    pub const fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer(_))
    }

    /// Returns true if this is an error or unknown type
    pub const fn is_error_like(&self) -> bool {
        matches!(self, Self::Error | Self::Unknown)
    }

    /// Gets the size in slots (field elements) for this type
    pub fn size_in_slots(&self) -> usize {
        match self {
            Self::Felt | Self::Bool | Self::Pointer(_) => 1,
            Self::U32 => 2, // U32 takes 2 field elements (low, high)
            Self::Tuple(types) => types.iter().map(|t| t.size_in_slots()).sum(),
            Self::Struct { fields, .. } => {
                // Sum the sizes of all fields
                fields
                    .iter()
                    .map(|(_, field_type)| field_type.size_in_slots())
                    .sum()
            }
            Self::Function { .. } => 1, // Function pointers
            Self::Unit => 0,
            Self::Error | Self::Unknown => 1, // Safe default
        }
    }

    /// Calculates the offset of a struct field by name
    /// Returns None if the field is not found or this is not a struct type
    pub fn field_offset(&self, field_name: &str) -> Option<usize> {
        match self {
            Self::Struct { fields, .. } => {
                let mut offset = 0;
                for (name, field_type) in fields {
                    if name == field_name {
                        return Some(offset);
                    }
                    offset += field_type.size_in_slots();
                }
                None
            }
            _ => None,
        }
    }

    /// Calculates the offset of a tuple element by index
    /// Returns None if the index is out of bounds or this is not a tuple type
    pub fn tuple_element_offset(&self, index: usize) -> Option<usize> {
        match self {
            Self::Tuple(types) => {
                if index >= types.len() {
                    return None;
                }

                // Calculate cumulative offset by summing sizes of previous elements
                let mut offset = 0;
                for type_at_i in types.iter().take(index) {
                    offset += type_at_i.size_in_slots();
                }
                Some(offset)
            }
            _ => None,
        }
    }

    /// Gets the type of a struct field by name
    /// Returns None if the field is not found or this is not a struct type
    pub fn field_type(&self, field_name: &str) -> Option<&Self> {
        match self {
            Self::Struct { fields, .. } => fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, field_type)| field_type),
            _ => None,
        }
    }

    /// Gets the type of a tuple element by index
    /// Returns None if the index is out of bounds or this is not a tuple type
    pub fn tuple_element_type(&self, index: usize) -> Option<&Self> {
        match self {
            Self::Tuple(types) => types.get(index),
            _ => None,
        }
    }

    /// Converts a semantic TypeId to MirType
    ///
    /// This is the main conversion function that properly handles all semantic types
    /// by recursively converting inner types and extracting information from
    /// interned types like structs and functions.
    pub fn from_semantic_type(db: &dyn SemanticDb, type_id: TypeId) -> Self {
        match type_id.data(db) {
            TypeData::Felt => Self::felt(),
            TypeData::U32 => Self::u32(),
            TypeData::Bool => Self::bool(),
            TypeData::Pointer(inner_type) => {
                let inner_mir_type = Self::from_semantic_type(db, inner_type);
                Self::pointer(inner_mir_type)
            }
            TypeData::Tuple(types) => {
                let mir_types: Vec<Self> = types
                    .iter()
                    .map(|t| Self::from_semantic_type(db, *t))
                    .collect();
                Self::tuple(mir_types)
            }
            TypeData::Struct(struct_id) => {
                let struct_name = struct_id.name(db);
                let semantic_fields = struct_id.fields(db);

                // Convert semantic fields to MIR fields (name, type) pairs
                let fields: Vec<(String, Self)> = semantic_fields
                    .into_iter()
                    .map(|(field_name, field_type_id)| {
                        let field_type = Self::from_semantic_type(db, field_type_id);
                        (field_name, field_type)
                    })
                    .collect();

                Self::struct_type(struct_name, fields)
            }
            TypeData::Function(func_sig) => {
                let params: Vec<Self> = func_sig
                    .params(db)
                    .iter()
                    .map(|(_, param_type)| Self::from_semantic_type(db, *param_type))
                    .collect();
                let return_type = Self::from_semantic_type(db, func_sig.return_type(db));
                Self::function(params, return_type)
            }
            TypeData::Error => Self::error(),
            TypeData::Unknown => Self::unknown(),
        }
    }
}

impl std::fmt::Display for MirType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Felt => write!(f, "felt"),
            Self::Bool => write!(f, "bool"),
            Self::U32 => write!(f, "u32"),
            Self::Pointer(inner) => write!(f, "*{inner}"),
            Self::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{ty}")?;
                }
                write!(f, ")")
            }
            Self::Struct { name, .. } => write!(f, "{name}"),
            Self::Function {
                params,
                return_type,
            } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Self::Unit => write!(f, "()"),
            Self::Error => write!(f, "<e>"),
            Self::Unknown => write!(f, "<unknown>"),
        }
    }
}
