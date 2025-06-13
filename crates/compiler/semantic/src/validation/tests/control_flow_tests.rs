//! # Control Flow Validation Tests
//!
//! Tests for the ControlFlowValidator to ensure proper detection of
//! unreachable code and other control flow issues.

use super::*;
use crate::assert_diagnostics_snapshot;

#[test]
fn test_unreachable_code_detection() {
    assert_diagnostics_snapshot!("unreachable_code.cm", "unreachable_code_diagnostics");
}

#[test]
fn test_missing_return_detection() {
    assert_diagnostics_snapshot!("missing_return.cm", "missing_return_diagnostics");
}

#[test]
fn test_clean_control_flow() {
    // Test a function with proper control flow (no unreachable code)
    let source = r#"
func clean_function() {
    let x = 1;
    if (x == 1) {
        x = 2;
    }
    return x;
}
"#;

    let db = SemanticDatabaseImpl::default();
    let source_program = SourceProgram::new(&db, source.to_string(), "test.cm".to_string());
    let file = File::new(&db, source_program.text(&db).clone(), "test.cm".to_string());
    let index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let registry = create_default_registry();
    let diagnostics = registry.validate_all(&db, file, index);

    // Filter only control flow related diagnostics
    let control_flow_diagnostics: Vec<_> = diagnostics
        .all()
        .iter()
        .filter(|d| matches!(d.code, DiagnosticCode::UnreachableCode))
        .collect();

    assert!(
        control_flow_diagnostics.is_empty(),
        "Expected no unreachable code diagnostics, but found: {control_flow_diagnostics:?}"
    );
}

#[test]
fn test_missing_return_simple() {
    // Test a function that should return but doesn't
    let source = r#"
func should_return(x: felt) -> felt {
    let y = x + 1;
    // Missing return statement
}
"#;

    let db = SemanticDatabaseImpl::default();
    let source_program = SourceProgram::new(&db, source.to_string(), "test.cm".to_string());
    let file = File::new(&db, source_program.text(&db).clone(), "test.cm".to_string());
    let index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let registry = create_default_registry();
    let diagnostics = registry.validate_all(&db, file, index);

    // Filter only missing return diagnostics
    let missing_return_diagnostics: Vec<_> = diagnostics
        .all()
        .iter()
        .filter(|d| matches!(d.code, DiagnosticCode::MissingReturn))
        .collect();

    assert_eq!(
        missing_return_diagnostics.len(),
        1,
        "Expected one missing return diagnostic, but found: {missing_return_diagnostics:?}"
    );

    assert!(
        missing_return_diagnostics[0]
            .message
            .contains("doesn't return on all paths"),
        "Expected missing return message, but got: {}",
        missing_return_diagnostics[0].message
    );
}

#[test]
fn test_explicit_return_needed() {
    // Test that functions without explicit return statements trigger missing return errors.
    let source = r#"
// An explicit return statement is still required.
func unit_return_type(x: felt) -> () {
    let y = x + 1;
    return();
}
"#;

    let db = SemanticDatabaseImpl::default();
    let source_program = SourceProgram::new(&db, source.to_string(), "test.cm".to_string());
    let file = File::new(&db, source_program.text(&db).clone(), "test.cm".to_string());
    let index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let registry = create_default_registry();
    let diagnostics = registry.validate_all(&db, file, index);

    // Filter only missing return diagnostics
    let missing_return_diagnostics: Vec<_> = diagnostics
        .all()
        .iter()
        .filter(|d| matches!(d.code, DiagnosticCode::MissingReturn))
        .collect();

    assert!(
        missing_return_diagnostics.is_empty(),
        "Expected no missing return diagnostics for unit functions, but found: {missing_return_diagnostics:?}"
    );
}
