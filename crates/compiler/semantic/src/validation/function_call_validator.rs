//! # Function Call Validator
//!
//! This validator checks that function calls are valid:
//! - Function exists and is callable
//! - Correct number of arguments provided
//! - Argument types match parameter types

use crate::db::SemanticDb;
use crate::validation::{Diagnostic, Validator};
use crate::{File, SemanticIndex};

/// Validator for function call expressions
///
/// This validator ensures that function calls (e.g., `foo(arg1, arg2)`)
/// are semantically valid by checking:
/// - The function exists and is accessible
/// - The correct number of arguments is provided
/// - Argument types match the function's parameter types
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// func add(x: felt, y: felt) -> felt { return x + y; }
///
/// let result1 = add(1, 2, 3); // Error: too many arguments
/// let result2 = add(1); // Error: too few arguments
/// let result3 = undefined_func(1); // Error: function doesn't exist
/// ```
///
/// TODO: Implement this validator once type system is available
pub struct FunctionCallValidator;

impl Validator for FunctionCallValidator {
    fn validate(
        &self,
        _db: &dyn SemanticDb,
        _file: File,
        _index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        // TODO: Implement function call validation
        // 1. Find all FunctionCall expressions in the semantic index
        // 2. For each function call:
        //    a. Resolve the function definition being called
        //    b. Check argument count matches parameter count
        //    c. Validate argument types match parameter types
        //    d. Check function is accessible in current scope
        //
        // Requires:
        // - Function definition lookup in semantic index
        // - Type system implementation for parameter type checking
        // - Expression type inference system
        // - Function resolution and overload handling

        Vec::new() // No validation yet
    }

    fn name(&self) -> &'static str {
        "FunctionCallValidator"
    }
}

#[cfg(test)]
mod tests {

    // TODO: Add tests for function call validation
    // - Test valid function calls
    // - Test wrong number of arguments
    // - Test argument type mismatches
    // - Test calls to undefined functions
    // - Test function accessibility/visibility
}
