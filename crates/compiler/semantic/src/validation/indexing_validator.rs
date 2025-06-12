//! # Indexing Validator
//!
//! This validator checks that indexing operations are valid:
//! - Object being indexed is actually indexable (array, slice, etc.)
//! - Index expression has appropriate type
//! - Bounds checking where possible

use crate::db::SemanticDb;
use crate::validation::Validator;
use crate::{File, SemanticIndex};
use cairo_m_compiler_diagnostics::Diagnostic;

/// Validator for indexing operations
///
/// This validator ensures that index access expressions (e.g., `arr[0]`)
/// are semantically valid by checking:
/// - The indexed object has an indexable type (array, slice, tuple, etc.)
/// - The index expression is of integer type
/// - Array bounds where statically determinable
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// let num = 42;
/// let bad = num[0]; // Error: cannot index felt
///
/// let arr = [1, 2, 3];
/// let bad = arr["string"]; // Error: index must be integer type
/// ```
///
/// TODO: Implement this validator once type system is available
pub struct IndexingValidator;

impl Validator for IndexingValidator {
    fn validate(
        &self,
        _db: &dyn SemanticDb,
        _file: File,
        _index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        // TODO: Implement indexing validation
        // 1. Find all IndexAccess expressions in the semantic index
        // 2. For each index access:
        //    a. Resolve the type of the array expression
        //    b. Check if it's an indexable type
        //    c. Resolve the type of the index expression
        //    d. Check if index is integer type
        //    e. Report error if types are invalid
        //
        // Requires:
        // - Type system implementation to resolve expression types
        // - Definition of indexable types (arrays, slices, tuples)
        // - Expression type inference system

        Vec::new() // No validation yet
    }

    fn name(&self) -> &'static str {
        "IndexingValidator"
    }
}

#[cfg(test)]
mod tests {

    // TODO: Add tests for indexing validation
    // - Test valid array indexing
    // - Test indexing non-indexable types
    // - Test non-integer index types
    // - Test bounds checking (when possible)
}
