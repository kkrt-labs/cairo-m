//! Common test utilities for semantic analysis tests
//!
//! This module contains all shared test infrastructure including:
//! - Test database setup
//! - Crate creation utilities
//! - Helper functions for semantic index access
//! - Test code generation helpers
//! - Diagnostic formatting and assertion macros

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_diagnostics::{
    DiagnosticCode, DiagnosticCollection, build_diagnostic_message,
};
use cairo_m_compiler_parser::{Db as ParserDb, SourceFile, Upcast};
use cairo_m_compiler_semantic::db::{Crate, project_validate_semantics};
use cairo_m_compiler_semantic::{File, SemanticDb, SemanticIndex, project_semantic_index};
use insta::assert_snapshot;

// ===== Test Database Setup =====

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

pub fn test_db() -> TestDb {
    TestDb::default()
}

// ===== Crate Creation Utilities =====

pub fn single_file_crate(db: &dyn SemanticDb, file: File) -> Crate {
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    Crate::new(
        db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    )
}

pub fn crate_from_program(db: &dyn SemanticDb, program: &str) -> Crate {
    let file = File::new(db, program.to_string(), "test.cm".to_string());
    single_file_crate(db, file)
}

pub fn get_main_semantic_index(db: &dyn SemanticDb, crate_id: Crate) -> SemanticIndex {
    let semantic_index = project_semantic_index(db, crate_id).unwrap();
    semantic_index.modules().values().next().unwrap().clone()
}

// ===== Validation and Diagnostic Utilities =====

/// Run semantic validation on source code
pub fn run_validation(source: &str, file_name: &str) -> DiagnosticCollection {
    let db = test_db();
    let source_program = SourceFile::new(&db, source.to_string(), file_name.to_string());
    let crate_id = single_file_crate(&db, source_program);
    project_validate_semantics(&db, crate_id)
}

// Format diagnostics for snapshot testing using ariadne for beautiful error reports
pub fn format_diagnostics_for_snapshot(
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

// ===== Assertion Implementation Functions =====

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

/// Helper to wrap statement code inside a function, since most statements are not top-level.
pub fn in_function(code: &str) -> String {
    format!("fn test() {{ {code} return; }}")
}

/// Filter out unused variable warnings from diagnostics
pub fn filter_unused_variable_warnings(diagnostics: &DiagnosticCollection) -> DiagnosticCollection {
    let filtered: Vec<_> = diagnostics
        .all()
        .iter()
        .filter(|d| d.code != DiagnosticCode::UnusedVariable)
        .cloned()
        .collect();
    DiagnosticCollection::from(filtered)
}

// ===== New Parameterized Testing Infrastructure =====

/// Result of parameterized semantic tests
#[derive(Debug)]
pub struct ParameterizedSemanticResults {
    pub results: Vec<ParameterizedSemanticResult>,
}

#[derive(Debug)]
pub enum ParameterizedSemanticResult {
    Error {
        input: String,
        diagnostics: DiagnosticCollection,
    },
}

impl std::fmt::Display for ParameterizedSemanticResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, result) in self.results.iter().enumerate() {
            if i > 0 {
                writeln!(f, "\n{}\n", "=".repeat(60))?;
            }

            match result {
                ParameterizedSemanticResult::Error { input, diagnostics } => {
                    writeln!(f, "--- Input {} (ERROR) ---", i + 1)?;
                    writeln!(f, "{}", input)?;
                    writeln!(f, "--- Diagnostics ---")?;
                    for diagnostic in diagnostics.all() {
                        let message = build_diagnostic_message(input, diagnostic, false);
                        write!(f, "{}", message)?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Assert that multiple code snippets validate with expected results
#[track_caller]
pub fn assert_semantic_parameterized_impl(
    inputs: &[(&str, bool)], // (code, should_succeed)
    test_name: &str,
    show_unused_warnings: bool,
) {
    let mut results = Vec::new();

    for (code, should_succeed) in inputs {
        let diagnostics = run_validation(code, test_name);
        let filtered_diagnostics = if show_unused_warnings {
            diagnostics
        } else {
            filter_unused_variable_warnings(&diagnostics)
        };

        if *should_succeed {
            if !filtered_diagnostics.is_empty() {
                let report =
                    format_diagnostics_for_snapshot(&filtered_diagnostics, code, test_name);
                panic!(
                    "Expected successful semantic validation for input '{}', but got diagnostics:\n{}",
                    code, report
                );
            }
        } else {
            if filtered_diagnostics.is_empty() {
                panic!(
                    "Expected semantic validation to fail for input '{}', but it succeeded",
                    code
                );
            }
            results.push(ParameterizedSemanticResult::Error {
                input: code.to_string(),
                diagnostics: filtered_diagnostics,
            });
        }
    }

    let snapshot = ParameterizedSemanticResults { results };

    let base_path = "semantic_tests::";
    let local_test_name = test_name.split(base_path).nth(1).unwrap_or(test_name);

    insta::with_settings!({
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!(format!("parameterized__{}", local_test_name), snapshot);
    });
}
