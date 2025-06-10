//! # Validation Testing Framework
//!
//! This module provides a unified testing infrastructure for semantic validation
//! based on snapshot testing. All tests follow a simple pattern:
//!
//! 1. Create a `.cm` file in `test_data/` containing the code to be tested
//! 2. Add a test that calls `assert_diagnostics_snapshot!("your_file.cm", "snapshot_name")`
//! 3. Run `cargo insta review` to generate and review the snapshot
//! 4. Commit both the `.cm` file and the generated `.snap` file
//!
//! ## Examples
//!
//! **Clean program (no diagnostics expected):**
//! ```rust,ignore
//! test_fixture_clean!("clean_program.cm");
//! ```
//!
//! **Program with expected diagnostics:**
//! ```rust,ignore
//! assert_diagnostics_snapshot!("scope_errors.cm", "scope_errors_diagnostics");
//! ```

use crate::db::SemanticDatabaseImpl;
use crate::validation::diagnostics::{DiagnosticCode, DiagnosticCollection, DiagnosticSeverity};
use crate::{semantic_index::semantic_index, File};
use cairo_m_compiler_parser::{parse_program, SourceProgram};
use std::fs;
use std::path::PathBuf;

use super::validator::create_default_registry;

pub mod diagnostic_tests;
pub mod integration_tests;

/// Path to the test data directory relative to the workspace root
const TEST_DATA_DIR: &str = "src/validation/tests/test_data";

/// Test a fixture file and generate a snapshot of all diagnostics
pub fn assert_diagnostics_snapshot(fixture_name: &str, snapshot_name: &str) {
    let source = load_fixture(fixture_name);
    let diagnostics = run_validation(&source);
    let snapshot_content = format_diagnostics_for_snapshot(&diagnostics, &source, fixture_name);
    insta::assert_snapshot!(snapshot_name, snapshot_content);
}

/// Test a fixture file expecting no diagnostics
pub fn test_fixture_clean(fixture_name: &str) {
    let source = load_fixture(fixture_name);
    let diagnostics = run_validation(&source);
    if !diagnostics.is_empty() {
        let report = format_diagnostics_for_snapshot(&diagnostics, &source, fixture_name);
        panic!("Expected clean validation for {fixture_name}, but found diagnostics:\n{report}");
    }
}

/// Helper macro for snapshot testing
#[macro_export]
macro_rules! assert_diagnostics_snapshot {
    ($fixture:expr, $snapshot_name:expr) => {
        $crate::validation::tests::assert_diagnostics_snapshot($fixture, $snapshot_name);
    };
}

/// Helper macro for clean fixture testing
#[macro_export]
macro_rules! test_fixture_clean {
    ($fixture:expr) => {
        $crate::validation::tests::test_fixture_clean($fixture);
    };
}

/// Load a fixture file from the test_data directory
fn load_fixture(fixture_name: &str) -> String {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push(TEST_DATA_DIR);
    fixture_path.push(fixture_name);

    fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", fixture_path.display(), e))
}

/// Check if a fixture file exists
pub fn fixture_exists(fixture_name: &str) -> bool {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push(TEST_DATA_DIR);
    fixture_path.push(fixture_name);
    fixture_path.exists()
}

/// List all available fixture files
pub fn list_fixtures() -> Vec<String> {
    let mut test_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_data_path.push(TEST_DATA_DIR);

    if !test_data_path.exists() {
        return vec![];
    }

    fs::read_dir(test_data_path)
        .unwrap_or_else(|_| panic!("Failed to read test_data directory"))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "cm") {
                path.file_name()?.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

// ===== Core validation and snapshot logic =====

/// Run semantic validation on source code
fn run_validation(source: &str) -> DiagnosticCollection {
    let db = SemanticDatabaseImpl::default();
    let source_program = SourceProgram::new(&db, source.to_string());

    // Create a file handle
    let file = File::new(&db, source_program.text(&db).clone());

    // Build semantic index
    let index = semantic_index(&db, file);

    // Run validation
    let registry = create_default_registry();
    registry.validate_all(&db, file, index)
}

/// Format diagnostics for snapshot testing using ariadne for beautiful error reports
fn format_diagnostics_for_snapshot(
    diagnostics: &DiagnosticCollection,
    source: &str,
    fixture_name: &str,
) -> String {
    use ariadne::{Label, Report, ReportKind, Source};

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
        let mut write_buffer = Vec::new();

        let report_kind = match diagnostic.severity {
            crate::validation::diagnostics::DiagnosticSeverity::Error => ReportKind::Error,
            crate::validation::diagnostics::DiagnosticSeverity::Warning => ReportKind::Warning,
            crate::validation::diagnostics::DiagnosticSeverity::Info => ReportKind::Advice,
            crate::validation::diagnostics::DiagnosticSeverity::Hint => ReportKind::Advice,
        };

        let mut report_builder = Report::build(report_kind, ((), diagnostic.span.into_range()))
            .with_config(
                ariadne::Config::new()
                    .with_index_type(ariadne::IndexType::Byte)
                    .with_color(false), // No color for snapshots
            )
            .with_code(diagnostic_code_to_u32(diagnostic.code))
            .with_message(&diagnostic.message)
            .with_label(
                Label::new(((), diagnostic.span.into_range())).with_message(&diagnostic.message),
            );

        // Add related spans as additional labels
        for (related_span, related_message) in &diagnostic.related_spans {
            report_builder = report_builder.with_label(
                Label::new(((), related_span.into_range()))
                    .with_message(related_message)
                    .with_color(ariadne::Color::Blue),
            );
        }

        report_builder
            .finish()
            .write(Source::from(source), &mut write_buffer)
            .unwrap();

        let report_string = String::from_utf8_lossy(&write_buffer);
        result.push_str(&format!("--- Diagnostic {} ---\n", i + 1));
        result.push_str(&report_string);

        if i < diagnostics.len() - 1 {
            result.push('\n');
        }
    }

    result
}

// ===== Utility functions =====

/// Convert DiagnosticCode to u32 for ariadne reporting
fn diagnostic_code_to_u32(code: DiagnosticCode) -> u32 {
    match code {
        DiagnosticCode::UndeclaredVariable => 1001,
        DiagnosticCode::UnusedVariable => 1002,
        DiagnosticCode::DuplicateDefinition => 1003,
        DiagnosticCode::UseBeforeDefinition => 1004,
        DiagnosticCode::TypeMismatch => 2001,
        DiagnosticCode::InvalidFieldAccess => 2002,
        DiagnosticCode::InvalidIndexAccess => 2003,
        DiagnosticCode::InvalidStructLiteral => 2004,
        DiagnosticCode::InvalidFunctionCall => 2005,
        DiagnosticCode::InvalidAssignment => 2006,
        DiagnosticCode::InvalidReturnType => 2007,
        DiagnosticCode::InvalidTypeDefinition => 2008,
        DiagnosticCode::UnreachableCode => 3001,
    }
}

#[cfg(test)]
mod tests_inner {
    use crate::db::tests::test_db;
    use crate::validate_semantics;

    use super::*;

    #[test]
    fn test_fixture_loading() {
        // Test that we can load the existing fib.cm fixture
        let source = load_fixture("fib.cm");
        assert!(!source.is_empty());
        assert!(source.contains("func fib"));
        assert!(source.contains("func add"));
    }

    #[test]
    fn test_list_fixtures() {
        let fixtures = list_fixtures();
        assert!(!fixtures.is_empty());
        assert!(fixtures.contains(&"fib.cm".to_string()));
    }

    #[test]
    fn test_fixture_exists() {
        assert!(fixture_exists("fib.cm"));
        assert!(!fixture_exists("nonexistent.cm"));
    }

    #[test]
    fn test_fib_fixture_validation() {
        // The fib.cm fixture should be a clean program with no validation errors
        test_fixture_clean("fib.cm");
    }

    #[test]
    fn test_validation_framework_integration() {
        let db = test_db();

        // Test program with multiple validation issues
        let source = SourceProgram::new(
            &db,
            r#"
                func test() -> felt {
                    let unused_var = 42;  // Warning: Unused variable
                    let used_var = 24;
                    return used_var;
                }
            "#
            .to_string(),
        );

        // Run validation
        let parsed_module = parse_program(&db, source);
        let diagnostics = validate_semantics(&db, parsed_module, source);

        // Should find the Unused variable
        assert!(!diagnostics.is_empty());

        let unused_warnings: Vec<_> = diagnostics
            .all()
            .iter()
            .filter(|d| d.code == DiagnosticCode::UnusedVariable)
            .collect();

        // Debug output to see what we found
        println!("Found {} Unused variable warnings:", unused_warnings.len());
        for warning in &unused_warnings {
            println!("  - {}", warning.message);
        }

        assert_eq!(unused_warnings.len(), 1);
        assert!(unused_warnings[0].message.contains("unused_var"));

        // Verify the validation system works end-to-end
        println!("Validation found {} diagnostics:", diagnostics.len());
        for diagnostic in diagnostics.all() {
            println!("  {diagnostic}");
        }
    }

    #[test]
    fn test_duplicate_definition_validation() {
        let db = test_db();

        // Test program with duplicate definitions
        let source = SourceProgram::new(
            &db,
            r#"
                func test() {
                    let var = 1;
                    let var = 2;  // Error: duplicate definition
                }
            "#
            .to_string(),
        );

        let parsed_module = parse_program(&db, source);
        let diagnostics = validate_semantics(&db, parsed_module, source);

        let duplicate_errors: Vec<_> = diagnostics
            .all()
            .iter()
            .filter(|d| d.code == DiagnosticCode::DuplicateDefinition)
            .collect();

        assert_eq!(duplicate_errors.len(), 1);
        assert!(duplicate_errors[0].message.contains("var"));
        assert_eq!(duplicate_errors[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_undeclared_variable_detection() {
        let db = test_db();

        // Test program with undeclared variable usage
        let source = SourceProgram::new(
            &db,
            r#"
                func test() -> felt {
                    let local_var = 42;
                    return local_var + undeclared_var;  // Error: undeclared variable
                }
            "#
            .to_string(),
        );

        let parsed_module = parse_program(&db, source);
        let diagnostics = validate_semantics(&db, parsed_module, source);

        let undeclared_errors: Vec<_> = diagnostics
            .all()
            .iter()
            .filter(|d| d.code == DiagnosticCode::UndeclaredVariable)
            .collect();

        assert_eq!(undeclared_errors.len(), 1);
        assert!(undeclared_errors[0].message.contains("undeclared_var"));
        assert_eq!(undeclared_errors[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_clean_program_validation() {
        let db = test_db();

        // Test program with no validation issues
        let source = SourceProgram::new(
            &db,
            r#"
                func add(a: felt, b: felt) -> felt {
                    return a + b;
                }

                func main() -> felt {
                    return add(1, 2);
                }
            "#
            .to_string(),
        );

        let parsed_module = parse_program(&db, source);
        let diagnostics = validate_semantics(&db, parsed_module, source);

        // Should have no errors
        let errors: Vec<_> = diagnostics.errors();
        assert_eq!(errors.len(), 0);

        // Should have no warnings either (all variables are used)
        let warnings: Vec<_> = diagnostics.warnings();
        assert_eq!(warnings.len(), 0);
    }
}
