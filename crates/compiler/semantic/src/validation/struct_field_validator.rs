//! # Struct Field Access Validator
//!
//! This validator checks that field access operations are valid:
//! - Field exists on the struct type
//! - Object is actually a struct type (not primitive)
//! - Field access is used correctly

use crate::db::SemanticDb;
use crate::validation::{Diagnostic, Validator};
use crate::{File, SemanticIndex};

/// Validator for struct field access operations
///
/// This validator ensures that member access expressions (e.g., `obj.field`)
/// are semantically valid by checking:
/// - The object has a struct type that contains the accessed field
/// - The field name exists on the struct definition
/// - The object is not a primitive type (felt, etc.)
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// let p = Point { x: 1, y: 2 };
/// let bad = p.z; // Error: field 'z' doesn't exist on Point
///
/// let num = 42;
/// let bad = num.field; // Error: felt has no fields
/// ```
///
/// TODO: Implement this validator once type system is available
pub struct StructFieldValidator;

impl Validator for StructFieldValidator {
    fn validate(
        &self,
        _db: &dyn SemanticDb,
        _file: File,
        _index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        // TODO: Implement struct field validation
        // 1. Find all MemberAccess expressions in the semantic index
        // 2. For each member access:
        //    a. Resolve the type of the object expression
        //    b. Check if it's a struct type
        //    c. Check if the field exists on the struct
        //    d. Report error if field doesn't exist or object isn't a struct
        //
        // Requires:
        // - Type system implementation to resolve expression types
        // - Struct definition lookup in semantic index
        // - Expression type inference system

        Vec::new() // No validation yet
    }

    fn name(&self) -> &'static str {
        "StructFieldValidator"
    }
}

#[cfg(test)]
mod tests {

    // TODO: Add tests for struct field validation
    // - Test valid field access
    // - Test invalid field access (field doesn't exist)
    // - Test field access on non-struct types
    // - Test nested field access
}
