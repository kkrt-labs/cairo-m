//! # Semantic Validation Tests
//!
//! This module contains comprehensive tests for semantic validation organized by concern.
//! Tests are structured to provide clear visibility into what semantic features are
//! implemented and validated.
//!
//! ## Test Organization
//!
//! - `scoping/` - Variable scoping, visibility, and declaration tests
//! - `types/` - Type checking, type resolution, and type validation tests
//! - `control_flow/` - Control flow analysis, unreachable code, missing returns
//! - `functions/` - Function calls, parameter validation, return type checking
//! - `structures/` - Struct definition, field access, and struct validation
//! - `expressions/` - Expression validation, operator usage, literal validation
//! - `statements/` - Statement validation (let, assignment, etc.)
//! - `integration/` - End-to-end integration tests with complex scenarios
//!
//! ## Test Utilities
//!
//! This module re-exports the core testing utilities:
//! - `assert_semantic_ok!(code)` - Assert code validates without errors
//! - `assert_semantic_err!(code)` - Assert code produces validation errors
//! - `assert_diagnostics_snapshot!(file, name)` - Snapshot test for .cm files

use cairo_m_compiler_diagnostics::{
    build_diagnostic_message, DiagnosticCode, DiagnosticCollection,
};
use cairo_m_compiler_parser::{Db as ParserDb, SourceProgram, Upcast};
use cairo_m_compiler_semantic::validation::validator::create_default_registry;
use cairo_m_compiler_semantic::{semantic_index::semantic_index, SemanticDb};
use insta::assert_snapshot;

#[salsa::db]
#[derive(Clone, Default)]
pub struct TestDb {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TestDb {}
#[salsa::db]
impl ParserDb for TestDb {}
#[salsa::db]
impl SemanticDb for TestDb {}

impl Upcast<dyn ParserDb> for TestDb {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

fn test_db() -> TestDb {
    TestDb::default()
}

/// Run semantic validation on source code
fn run_validation(source: &str, file_name: &str) -> DiagnosticCollection {
    let db = test_db();
    let source_program = SourceProgram::new(&db, source.to_string(), file_name.to_string());

    // Build semantic index
    let index = semantic_index(&db, source_program)
        .as_ref()
        .expect("Got unexpected parse errors");

    // Run validation
    let registry = create_default_registry();
    registry.validate_all(&db, source_program, index)
}

// Format diagnostics for snapshot testing using ariadne for beautiful error reports
fn format_diagnostics_for_snapshot(
    diagnostics: &DiagnosticCollection,
    source: &str,
    fixture_name: &str,
) -> String {
    let mut result = String::new();

    // Add header
    result.push_str(&format!("Fixture: {fixture_name}\n"));
    result.push_str(&"=".repeat(60));
    result.push('\n');
    result.push_str(&format!("Source code:\n{source}\n"));
    result.push_str(&"=".repeat(60));
    result.push('\n');

    if diagnostics.is_empty() {
        result.push_str("No diagnostics found.\n");
        return result;
    }

    result.push_str(&format!("Found {} diagnostic(s):\n\n", diagnostics.len()));

    // Create ariadne reports for each diagnostic
    for (i, diagnostic) in diagnostics.all().iter().enumerate() {
        let message = build_diagnostic_message(source, diagnostic, false);
        result.push_str(&format!("--- Diagnostic {} ---\n", i + 1));
        result.push_str(&message);
        if i < diagnostics.len() - 1 {
            result.push('\n');
        }
    }

    result
}

/// Assert that inline code validates successfully without any diagnostics
/// Similar to parser's assert_parses_ok! but for semantic validation
#[track_caller]
pub fn assert_semantic_ok_impl(code: &str, test_name: &str) {
    assert_semantic_ok_impl_with_options(code, test_name, false)
}

/// Assert that inline code validates successfully without any diagnostics
/// with option to mute unused variable warnings
#[track_caller]
pub fn assert_semantic_ok_impl_with_options(
    code: &str,
    test_name: &str,
    show_unused_warnings: bool,
) {
    let diagnostics = run_validation(code, test_name);
    let filtered_diagnostics = if show_unused_warnings {
        diagnostics
    } else {
        filter_unused_variable_warnings(&diagnostics)
    };

    if !filtered_diagnostics.is_empty() {
        let report = format_diagnostics_for_snapshot(&filtered_diagnostics, code, test_name);
        panic!("Expected successful semantic validation, but got diagnostics:\n{report}");
    }
}

/// Assert that inline code fails semantic validation and produces diagnostics
/// Similar to parser's assert_parses_err! but for semantic validation
#[track_caller]
pub fn assert_semantic_err_impl(code: &str, test_name: &str) {
    assert_semantic_err_impl_with_options(code, test_name, false)
}

/// Assert that inline code fails semantic validation and produces diagnostics
/// with option to mute unused variable warnings
#[track_caller]
pub fn assert_semantic_err_impl_with_options(
    code: &str,
    test_name: &str,
    show_unused_warnings: bool,
) {
    let diagnostics = run_validation(code, test_name);
    let filtered_diagnostics = if show_unused_warnings {
        diagnostics
    } else {
        filter_unused_variable_warnings(&diagnostics)
    };

    if filtered_diagnostics.is_empty() {
        panic!("Expected semantic validation to fail, but it succeeded without diagnostics.");
    }

    let snapshot_content = format_diagnostics_for_snapshot(&filtered_diagnostics, code, test_name);

    // Extract local test name for better snapshot organization
    let base_path = "semantic_tests::";
    let local_test_name = test_name.split(base_path).nth(1).unwrap_or(test_name);

    insta::with_settings!({
        description => format!("Inline semantic validation error test: {}", local_test_name),
        omit_expression => true,
        sort_maps => true,
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!(format!("diagnostics__{}", local_test_name), snapshot_content);
    });
}

/// Helper macro for snapshot testing
#[macro_export]
macro_rules! assert_diagnostics_snapshot {
    ($fixture:expr, $snapshot_name:expr) => {
        $crate::assert_diagnostics_snapshot($fixture, $snapshot_name);
    };
    ($fixture:expr, $snapshot_name:expr, $description:expr) => {
        $crate::validation::tests::assert_diagnostics_snapshot_with_context(
            $fixture,
            $snapshot_name,
            Some($description),
            None::<&dyn std::fmt::Debug>,
        );
    };
    ($fixture:expr, $snapshot_name:expr, $description:expr, $context:expr) => {
        $crate::validation::tests::assert_diagnostics_snapshot_with_context(
            $fixture,
            $snapshot_name,
            Some($description),
            Some($context),
        );
    };
}

/// Helper macro for clean fixture testing
#[macro_export]
macro_rules! test_fixture_clean {
    ($fixture:expr) => {
        $crate::test_fixture_clean($fixture);
    };
}

/// Macro to assert that inline code validates successfully
#[macro_export]
macro_rules! assert_semantic_ok {
    ($code:expr) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_ok_impl($code, function_name)
    }};
    ($code:expr, show_unused) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_ok_impl_with_options($code, function_name, true)
    }};
}

/// Macro to assert that inline code fails semantic validation
#[macro_export]
macro_rules! assert_semantic_err {
    ($code:expr) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_err_impl($code, function_name)
    }};
    ($code:expr, show_unused) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_err_impl_with_options($code, function_name, true)
    }};
}

// ===== Helper functions for test code generation =====

/// Helper to wrap statement code inside a function, since most statements are not top-level.
pub fn in_function(code: &str) -> String {
    format!("func test() {{ {code} }}")
}

/// Helper to wrap code in a function with a return type
pub fn in_function_with_return(code: &str, return_type: &str) -> String {
    format!("func test() -> {return_type} {{ {code} }}")
}

/// Helper to wrap code in a function with parameters
pub fn in_function_with_params(code: &str, params: &str) -> String {
    format!("func test({params}) {{ {code} }}")
}

/// Helper to wrap code in a function with both parameters and return type
pub fn in_function_with_params_and_return(code: &str, params: &str, return_type: &str) -> String {
    format!("func test({params}) -> {return_type} {{ {code} }}")
}

/// Helper to create a struct definition with the given fields
pub fn with_struct(struct_name: &str, fields: &str, code: &str) -> String {
    format!("struct {struct_name} {{ {fields} }}\n\n{code}")
}

/// Helper to create multiple function definitions
pub fn with_functions(functions: &str, main_code: &str) -> String {
    format!("{functions}\n\n{main_code}")
}

// ===== Diagnostic filtering functions =====

/// Filter out unused variable warnings from diagnostics
fn filter_unused_variable_warnings(diagnostics: &DiagnosticCollection) -> DiagnosticCollection {
    let filtered: Vec<_> = diagnostics
        .all()
        .iter()
        .filter(|d| d.code != DiagnosticCode::UnusedVariable)
        .cloned()
        .collect();
    DiagnosticCollection::from(filtered)
}

// Test modules organized by concern
pub mod control_flow;
pub mod expressions;
pub mod functions;
pub mod integration;
pub mod scoping;
pub mod statements;
pub mod structures;
pub mod types;
