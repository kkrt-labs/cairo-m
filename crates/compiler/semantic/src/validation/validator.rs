//! # Validator Trait and Registry
//!
//! This module defines the trait for semantic validators and provides
//! a registry system for organizing multiple validators.

use crate::db::SemanticDb;
use crate::validation::diagnostics::{Diagnostic, DiagnosticCollection};
use crate::{File, SemanticIndex};

/// Trait for semantic validators
pub trait Validator {
    /// Validate the semantic index and return diagnostics
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic>;

    /// Get the name of this validator (for debugging/logging)
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Registry for managing multiple validators
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
pub fn create_default_registry() -> ValidatorRegistry {
    ValidatorRegistry::new().add_validator(crate::validation::scope_check::ScopeValidator)
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
        let validator1 = MockValidator::new("test1", vec![Diagnostic::undeclared_variable("var1")]);
        let validator2 = MockValidator::new("test2", vec![Diagnostic::unused_variable("var2")]);

        let registry = ValidatorRegistry::new()
            .add_validator(validator1)
            .add_validator(validator2);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }
}
