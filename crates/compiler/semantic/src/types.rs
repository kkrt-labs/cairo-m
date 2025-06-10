//! # Type System for Cairo-M
//!
//! This module implements the core type system using Salsa for incremental computation.
//! Types are represented as interned IDs for efficient comparison and storage.
//!
//! ## Architecture
//!
//! - `TypeId`: Main type identifier that wraps `TypeData`
//! - `TypeData`: The actual type information (primitives, structs, functions, etc.)
//! - `StructTypeId`: Interned struct type with fields
//! - `FunctionSignatureId`: Interned function signature with parameters and return type
//!
//! ## Design Notes
//!
//! All complex types (structs, functions) are interned separately to enable efficient
//! structural comparison and avoid deep recursion during type checking.

#[cfg(test)]
mod types_test;

use crate::place::FileScopeId;
use crate::semantic_index::DefinitionId;
use crate::SemanticDb;

/// Main type identifier that represents any type in the system
///
/// This is a Salsa-interned type that wraps `TypeData`. The interning ensures
/// that identical types have the same `TypeId`, enabling fast equality comparisons
/// and reducing memory usage.
#[salsa::interned(debug)]
pub struct TypeId<'db> {
    #[return_ref]
    pub data: TypeData<'db>,
}

/// The actual type information
///
/// This enum represents all the different kinds of types in Cairo-M.
/// Complex types like structs and functions are represented by their own
/// interned IDs to keep this enum lightweight.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeData<'db> {
    /// The `felt` primitive type - Cairo's basic field element type
    Felt,

    /// A struct type, identified by its interned struct type ID
    Struct(StructTypeId<'db>),

    /// A tuple type containing an ordered list of component types
    Tuple(Vec<TypeId<'db>>),

    /// A pointer type pointing to another type
    Pointer(TypeId<'db>),

    /// A function type with its signature
    Function(FunctionSignatureId<'db>),

    /// Represents an unknown type during inference
    /// Used when type information is not yet available or during inference cycles
    Unknown,

    /// Represents a type error
    /// Used to prevent cascading errors when type checking fails
    Error,
}

/// Interned struct type definition
///
/// This contains all the semantic information about a struct type,
/// including its fields and their types. The interning ensures that
/// structurally identical struct types are represented by the same ID.
#[salsa::interned(debug)]
pub struct StructTypeId<'db> {
    /// The definition ID that corresponds to this struct in the semantic index
    pub definition_id: DefinitionId<'db>,

    /// The name of the struct
    pub name: String,

    /// The fields of the struct as an ordered map of name to type.
    /// Preserves field declaration order while allowing fast name lookup.
    /// TODO: Move to a hashmap once we figure out how to make salsa work with it.
    #[return_ref]
    pub fields: Vec<(String, TypeId<'db>)>,

    /// The scope where this struct is defined
    pub scope_id: FileScopeId,
}

/// Interned function signature
///
/// This represents the type signature of a function, including its
/// parameters and return type. Interning allows efficient comparison
/// of function types and enables function type compatibility checking.
#[salsa::interned(debug)]
pub struct FunctionSignatureId<'db> {
    /// The definition ID that corresponds to this function in the semantic index
    pub definition_id: DefinitionId<'db>,

    /// The parameters of the function as an ordered map of name to type.
    /// Preserves parameter declaration order while allowing fast name lookup.
    /// TODO: Move to a hashmap once we figure out how to make salsa work with it.
    #[return_ref]
    pub params: Vec<(String, TypeId<'db>)>,

    /// The return type of the function
    pub return_type: TypeId<'db>,
}

impl<'db> TypeData<'db> {
    /// Check if this type is a primitive type
    pub const fn is_primitive(&self) -> bool {
        matches!(self, TypeData::Felt)
    }

    /// Check if this type represents an error state
    pub const fn is_error(&self) -> bool {
        matches!(self, TypeData::Error)
    }

    /// Check if this type is unknown/unresolved
    pub const fn is_unknown(&self) -> bool {
        matches!(self, TypeData::Unknown)
    }

    /// Check if this type is a concrete type (not error or unknown)
    pub const fn is_concrete(&self) -> bool {
        !self.is_error() && !self.is_unknown()
    }

    /// Get a human-readable display name for this type
    pub fn display_name(&self, db: &dyn SemanticDb) -> String {
        match self {
            TypeData::Felt => "felt".to_string(),
            TypeData::Struct(struct_id) => struct_id.name(db),
            TypeData::Tuple(types) => {
                let type_names: Vec<String> =
                    types.iter().map(|t| t.data(db).display_name(db)).collect();
                format!("({})", type_names.join(", "))
            }
            TypeData::Pointer(inner) => {
                format!("{}*", inner.data(db).display_name(db))
            }
            TypeData::Function(_) => "function".to_string(),
            TypeData::Unknown => "<unknown>".to_string(),
            TypeData::Error => "<error>".to_string(),
        }
    }
}

impl<'db> StructTypeId<'db> {
    /// Get the type of a specific field by name
    pub fn field_type(&self, db: &'db dyn SemanticDb, field_name: &str) -> Option<TypeId<'db>> {
        self.fields(db)
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, type_id)| *type_id)
    }

    /// Check if this struct has a field with the given name
    pub fn has_field(&self, db: &dyn SemanticDb, field_name: &str) -> bool {
        self.fields(db).iter().any(|(name, _)| name == field_name)
    }

    /// Get all field names in declaration order
    pub fn field_names(&self, db: &'db dyn SemanticDb) -> Vec<String> {
        self.fields(db)
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl<'db> FunctionSignatureId<'db> {
    /// Get the type of a specific parameter by name
    pub fn param_type(&self, db: &'db dyn SemanticDb, param_name: &str) -> Option<TypeId<'db>> {
        self.params(db)
            .iter()
            .find(|(name, _)| name == param_name)
            .map(|(_, type_id)| *type_id)
    }

    /// Check if this function has a parameter with the given name
    pub fn has_param(&self, db: &dyn SemanticDb, param_name: &str) -> bool {
        self.params(db).iter().any(|(name, _)| name == param_name)
    }

    /// Get all parameter names in declaration order
    pub fn param_names(&self, db: &'db dyn SemanticDb) -> Vec<String> {
        self.params(db)
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get the number of parameters
    pub fn param_count(&self, db: &dyn SemanticDb) -> usize {
        self.params(db).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;

    #[test]
    fn test_type_data_classification() {
        let felt_data = TypeData::Felt;
        assert!(felt_data.is_primitive());
        assert!(felt_data.is_concrete());
        assert!(!felt_data.is_error());
        assert!(!felt_data.is_unknown());

        let error_data = TypeData::Error;
        assert!(error_data.is_error());
        assert!(!error_data.is_concrete());

        let unknown_data = TypeData::Unknown;
        assert!(unknown_data.is_unknown());
        assert!(!unknown_data.is_concrete());
    }

    #[test]
    fn test_type_display_names() {
        let db = test_db();
        assert_eq!(TypeData::Felt.display_name(&db), "felt");
        assert_eq!(TypeData::Error.display_name(&db), "<error>");
        assert_eq!(TypeData::Unknown.display_name(&db), "<unknown>");
    }

    #[test]
    fn test_type_interning() {
        let db = test_db();

        // Same TypeData should produce the same TypeId
        let type1 = TypeId::new(&db, TypeData::Felt);
        let type2 = TypeId::new(&db, TypeData::Felt);
        assert_eq!(type1, type2);

        // Different TypeData should produce different TypeId
        let type3 = TypeId::new(&db, TypeData::Error);
        assert_ne!(type1, type3);
    }
}
