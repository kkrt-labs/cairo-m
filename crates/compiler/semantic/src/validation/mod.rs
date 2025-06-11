//! # Semantic Validation Framework
//!
//! This module implements validation rules for Cairo-M semantic analysis.
//! It provides a diagnostic system and validator trait pattern for extensible
//! semantic checking.

pub mod diagnostics;
pub mod scope_check;
pub mod validator;

// TODO: Implement these validators once type system is available
pub mod function_call_validator;
pub mod indexing_validator;
pub mod struct_field_validator;
pub mod struct_literal_validator;

#[cfg(test)]
pub mod tests;

pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCollection, DiagnosticSeverity};
pub use function_call_validator::FunctionCallValidator;
pub use scope_check::ScopeValidator;
pub use validator::Validator;

// TODO: Export these validators once implemented
// pub use struct_field_validator::StructFieldValidator;
// pub use struct_literal_validator::StructLiteralValidator;
// pub use indexing_validator::IndexingValidator;
