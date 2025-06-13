//! # Diagnostic System Tests
//!
//! This module tests the diagnostic infrastructure itself, including:
//! - Diagnostic creation and formatting
//! - DiagnosticCollection operations

use cairo_m_compiler_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCollection, DiagnosticSeverity,
};
use chumsky::span::SimpleSpan;

#[test]
fn test_diagnostic_creation() {
    let span = SimpleSpan::from(10..20);

    let error = Diagnostic::undeclared_variable("test.cm".to_string(), "test_var", span);
    assert_eq!(error.severity, DiagnosticSeverity::Error);
    assert_eq!(error.code, DiagnosticCode::UndeclaredVariable);
    assert!(error.message.contains("test_var"));
    assert_eq!(error.span, span);

    let warning = Diagnostic::unused_variable("test.cm".to_string(), "unused_var", span);
    assert_eq!(warning.severity, DiagnosticSeverity::Warning);
    assert_eq!(warning.code, DiagnosticCode::UnusedVariable);
    assert!(warning.message.contains("unused_var"));

    let duplicate = Diagnostic::duplicate_definition("test.cm".to_string(), "dup_var", span);
    assert_eq!(duplicate.severity, DiagnosticSeverity::Error);
    assert_eq!(duplicate.code, DiagnosticCode::DuplicateDefinition);
    assert!(duplicate.message.contains("dup_var"));
}

#[test]
fn test_diagnostic_with_related_spans() {
    let span = SimpleSpan::from(10..20);
    let related_span = SimpleSpan::from(5..8);

    let diagnostic = Diagnostic::undeclared_variable("test.cm".to_string(), "test_var", span)
        .with_related_span(
            "test.cm".to_string(),
            related_span,
            "first defined here".to_string(),
        );

    assert_eq!(diagnostic.related_spans.len(), 1);
    assert_eq!(diagnostic.related_spans[0].0, related_span);
    assert_eq!(diagnostic.related_spans[0].1, "first defined here");
}

#[test]
fn test_diagnostic_collection_basic() {
    let mut collection = DiagnosticCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);

    collection.add(Diagnostic::error(
        DiagnosticCode::UndeclaredVariable,
        "error message".to_string(),
    ));
    collection.add(Diagnostic::warning(
        DiagnosticCode::UnusedVariable,
        "warning message".to_string(),
    ));
    collection.add(Diagnostic::info(
        DiagnosticCode::TypeMismatch,
        "info message".to_string(),
    ));

    assert!(!collection.is_empty());
    assert_eq!(collection.len(), 3);
    assert!(collection.has_errors());
}

#[test]
fn test_diagnostic_collection_filtering() {
    let mut collection = DiagnosticCollection::new();

    collection.add(Diagnostic::error(
        DiagnosticCode::UndeclaredVariable,
        "error 1".to_string(),
    ));
    collection.add(Diagnostic::warning(
        DiagnosticCode::UnusedVariable,
        "warning 1".to_string(),
    ));
    collection.add(Diagnostic::error(
        DiagnosticCode::DuplicateDefinition,
        "error 2".to_string(),
    ));
    collection.add(Diagnostic::warning(
        DiagnosticCode::UnusedVariable,
        "warning 2".to_string(),
    ));

    let errors = collection.errors();
    assert_eq!(errors.len(), 2);

    let warnings = collection.warnings();
    assert_eq!(warnings.len(), 2);

    assert!(collection.has_errors());
}

#[test]
fn test_diagnostic_collection_from_vec() {
    let diagnostics = vec![
        Diagnostic::error(DiagnosticCode::UndeclaredVariable, "error".to_string()),
        Diagnostic::warning(DiagnosticCode::UnusedVariable, "warning".to_string()),
    ];

    let collection = DiagnosticCollection::from(diagnostics);
    assert_eq!(collection.len(), 2);
    assert!(collection.has_errors());
}

#[test]
fn test_diagnostic_display() {
    let span = SimpleSpan::from(10..15);
    let diagnostic = Diagnostic::undeclared_variable("test.cm".to_string(), "test_var", span);

    let display_string = format!("{diagnostic}");
    assert!(display_string.contains("error"));
    assert!(display_string.contains("test_var"));
    assert!(display_string.contains("10:15"));
}
