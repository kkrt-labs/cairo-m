//! # Validator Trait and Registry
//!
//! This module defines the trait for semantic validators and provides
//! a registry system for organizing multiple validators.
//!
//! # Architecture
//!
//! The validator system uses a plugin-like architecture where different
//! validation concerns are separated into individual validators. This allows
//! for:
//! - Modular validation logic
//! - Easy addition of new validation rules
//! - Configurable validation passes
//! - Independent testing of validation rules
//!
//! # Usage
//!
//! ```rust,ignore
//! // Create a registry with default validators
//! let registry = create_default_registry();
//!
//! // Or build a custom registry
//! let registry = ValidatorRegistry::new()
//!     .add_validator(ScopeValidator)
//!     .add_validator(TypeValidator);  // TODO: Implement
//!
//! // Run validation
//! let diagnostics = registry.validate_all(&db, file, &index);
//! ```

use crate::db::SemanticDb;
use crate::validation::diagnostics::{Diagnostic, DiagnosticCollection};
use crate::{File, SemanticIndex};

/// Trait for semantic validators
///
/// Each validator implements a specific category of semantic analysis,
/// such as scope checking, type checking, or control flow analysis.
///
/// # Implementation Guidelines
///
/// - Validators should be stateless and thread-safe
/// - Each validator should focus on a single concern
/// - Validators should not modify the semantic index
/// - Return diagnostics in order of source location when possible
pub trait Validator {
    /// Validate the semantic index and return diagnostics
    ///
    /// This is the main entry point for validation logic. Implementers
    /// should analyze the provided semantic index and return any issues
    /// found as a vector of diagnostics.
    ///
    /// # Parameters
    ///
    /// - `db`: Database for additional queries if needed
    /// - `file`: The file being validated (for context)
    /// - `index`: The semantic index containing all semantic information
    ///
    /// # Returns
    ///
    /// A vector of diagnostics representing any issues found during validation.
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic>;

    /// Get the name of this validator (for debugging/logging)
    ///
    /// By default, this returns the type name, but implementers can
    /// override it to provide more descriptive names.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    // TODO: Add more trait methods for advanced validator features:
}

/// Registry for managing multiple validators
///
/// The registry maintains a collection of validators and provides
/// methods to run them collectively. It handles:
/// - Aggregating diagnostics from multiple validators
/// - Managing validator lifecycle
/// - Providing a unified interface for validation
///
/// TODO: Add support for:
/// - Validator ordering/dependencies
/// - Parallel validation execution
/// - Validator configuration and filtering
/// - Performance metrics collection
#[derive(Default)]
pub struct ValidatorRegistry {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validator to the registry
    pub fn add_validator<V: Validator + 'static>(mut self, validator: V) -> Self {
        self.validators.push(Box::new(validator));
        self
    }

    /// Run all validators and collect diagnostics
    pub fn validate_all(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
    ) -> DiagnosticCollection {
        let mut collection = DiagnosticCollection::new();

        for validator in &self.validators {
            let diagnostics = validator.validate(db, file, index);
            collection.extend(diagnostics);
        }

        collection
    }

    /// Get the number of registered validators
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}

/// Create a default validator registry with basic semantic validators
///
/// This provides a sensible default set of validators for most use cases.
/// Currently includes:
/// - **ScopeValidator**: Undeclared variables, unused variables, duplicate definitions
/// - **FunctionCallValidator**: Validate function call arity and argument types
///
/// TODO: Expand default registry with additional validators:
/// - **StructFieldValidator**: Validate field access on struct types (p.z should error)
/// - **StructLiteralValidator**: Validate struct literal completeness and field names
/// - **IndexingValidator**: Validate indexing on non-array types (num[0] should error)
/// - **AssignmentValidator**: Validate assignment compatibility and mutability
/// - **ReturnValidator**: Validate return type consistency and placement
/// - **TypeValidator**: General type checking and compatibility
/// - **ControlFlowValidator**: Reachability analysis, dead code detection
/// - **RecursiveTypeValidator**: Detect recursive struct definitions without indirection
/// - **ConstExpressionValidator**: Validate const expressions are compile-time evaluable
/// - **ModuleValidator**: Import/export validation, module resolution
/// - **StyleValidator**: Code style and best practices
/// - **SecurityValidator**: Security-related checks
/// - **PerformanceValidator**: Performance hints and optimizations
pub fn create_default_registry() -> ValidatorRegistry {
    ValidatorRegistry::new()
        .add_validator(crate::validation::scope_check::ScopeValidator)
        .add_validator(crate::validation::function_call_validator::FunctionCallValidator)
        .add_validator(crate::validation::struct_field_validator::StructFieldValidator)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock validator for testing
    struct MockValidator {
        name: &'static str,
        diagnostics: Vec<Diagnostic>,
    }

    impl MockValidator {
        fn new(name: &'static str, diagnostics: Vec<Diagnostic>) -> Self {
            Self { name, diagnostics }
        }
    }

    impl Validator for MockValidator {
        fn validate(
            &self,
            _db: &dyn SemanticDb,
            _file: File,
            _index: &SemanticIndex,
        ) -> Vec<Diagnostic> {
            self.diagnostics.clone()
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn test_validator_registry() {
        let span = chumsky::span::SimpleSpan::from(0..4);
        let validator1 =
            MockValidator::new("test1", vec![Diagnostic::undeclared_variable("var1", span)]);
        let validator2 =
            MockValidator::new("test2", vec![Diagnostic::unused_variable("var2", span)]);

        let registry = ValidatorRegistry::new()
            .add_validator(validator1)
            .add_validator(validator2);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }
}
