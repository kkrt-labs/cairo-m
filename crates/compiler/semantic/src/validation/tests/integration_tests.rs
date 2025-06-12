//! # Integration Tests
//!
//! This module contains integration tests that verify the validation system
//! works correctly end-to-end with real Cairo-M programs. All tests use
//! fixture files from the test_data directory and snapshot testing.

use crate::{assert_diagnostics_snapshot, test_fixture_clean};

#[test]
fn test_fib_program_is_clean() {
    // The fibonacci program should have no validation errors
    test_fixture_clean!("fib.cm");
}

#[test]
fn test_clean_program_fixture() {
    // The clean program fixture should have no validation errors
    test_fixture_clean!("clean_program.cm");
}

#[test]
fn test_scope_errors_snapshot() {
    // Generate a snapshot for the scope errors fixture
    // This provides a visual representation of all diagnostics
    assert_diagnostics_snapshot!("scope_errors.cm", "scope_errors_diagnostics");
}

#[test]
fn test_fib_program_snapshot() {
    // Generate a snapshot for the fib program (should show no diagnostics)
    assert_diagnostics_snapshot!("fib.cm", "fib_program_diagnostics");
}

#[test]
fn test_clean_program_snapshot() {
    // Generate a snapshot for the clean program (should show no diagnostics)
    assert_diagnostics_snapshot!("clean_program.cm", "clean_program_diagnostics");
}

#[test]
fn test_namespace_scoping_errors() {
    assert_diagnostics_snapshot!(
        "namespace_scoping_errors.cm",
        "namespace_scoping_errors_diagnostics"
    );
}

#[test]
fn test_parameter_vs_local_scoping() {
    assert_diagnostics_snapshot!(
        "parameter_vs_local_scoping.cm",
        "parameter_vs_local_scoping_diagnostics"
    );
}

#[test]
fn test_function_call_validation() {
    assert_diagnostics_snapshot!(
        "function_call_validation.cm",
        "function_call_validation_diagnostics"
    );
}

#[test]
fn test_struct_field_validation() {
    assert_diagnostics_snapshot!(
        "struct_field_validation.cm",
        "struct_field_validation_diagnostics"
    );
}

#[test]
fn test_const_validation() {
    assert_diagnostics_snapshot!("const_validation.cm", "const_validation_diagnostics");
}

#[test]
fn test_control_flow_scoping() {
    assert_diagnostics_snapshot!(
        "control_flow_scoping.cm",
        "control_flow_scoping_diagnostics"
    );
}

#[test]
fn test_deeply_nested_scopes() {
    assert_diagnostics_snapshot!(
        "deeply_nested_scopes.cm",
        "deeply_nested_scopes_diagnostics"
    );
}

// ===== Scope Validation Tests (migrated from scope_validation_tests.rs) =====

#[test]
fn test_simple_undeclared_variable() {
    assert_diagnostics_snapshot!(
        "simple_undeclared_variable.cm",
        "simple_undeclared_variable_diagnostics"
    );
}

#[test]
fn test_undeclared_in_expressions() {
    assert_diagnostics_snapshot!(
        "undeclared_in_expressions.cm",
        "undeclared_in_expressions_diagnostics"
    );
}

#[test]
fn test_basic_scope_visibility() {
    assert_diagnostics_snapshot!(
        "basic_scope_visibility.cm",
        "basic_scope_visibility_diagnostics"
    );
}

#[test]
fn test_unused_variables() {
    assert_diagnostics_snapshot!("unused_variables.cm", "unused_variables_diagnostics");
}

#[test]
fn test_duplicate_definitions() {
    assert_diagnostics_snapshot!(
        "duplicate_definitions.cm",
        "duplicate_definitions_diagnostics"
    );
}

#[test]
fn test_nested_scope_access() {
    assert_diagnostics_snapshot!("nested_scope_access.cm", "nested_scope_access_diagnostics");
}

#[test]
fn test_struct_and_const_usage() {
    assert_diagnostics_snapshot!(
        "struct_and_const_usage.cm",
        "struct_and_const_usage_diagnostics"
    );
}

#[test]
fn test_edge_cases() {
    assert_diagnostics_snapshot!("edge_cases.cm", "edge_cases_diagnostics");
}

#[test]
fn test_missing_validation_cases() {
    assert_diagnostics_snapshot!(
        "missing_validation_cases.cm",
        "missing_validation_cases_diagnostics"
    );
}

#[test]
fn test_function_call_comprehensive() {
    assert_diagnostics_snapshot!(
        "function_call_comprehensive.cm",
        "function_call_comprehensive_diagnostics"
    );
}

#[test]
fn test_indexing_validation() {
    assert_diagnostics_snapshot!("indexing_validation.cm", "indexing_validation_diagnostics");
}

#[test]
fn test_let_statement_validation() {
    assert_diagnostics_snapshot!(
        "let_statement_validation.cm",
        "let_statement_validation_diagnostics"
    );
}

#[test]
fn test_return_type_validation() {
    assert_diagnostics_snapshot!(
        "return_type_validation.cm",
        "return_type_validation_diagnostics"
    );
}

#[test]
fn test_if_statement_validation() {
    assert_diagnostics_snapshot!(
        "if_statement_validation.cm",
        "if_statement_validation_diagnostics"
    );
}

#[test]
fn test_local_statement_validation() {
    assert_diagnostics_snapshot!(
        "local_statement_validation.cm",
        "local_statement_validation_diagnostics"
    );
}

#[test]
fn test_assignment_validation() {
    assert_diagnostics_snapshot!(
        "assignment_validation.cm",
        "assignment_validation_diagnostics"
    );
}
