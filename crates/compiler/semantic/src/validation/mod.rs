//! # Semantic Validation Framework
//!
//! This module implements validation rules for Cairo-M semantic analysis.
//! It provides a diagnostic system and validator trait pattern for extensible
//! semantic checking.

pub mod diagnostics;
pub mod scope_check;
pub mod validator;

pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCollection, DiagnosticSeverity};
pub use scope_check::ScopeValidator;
pub use validator::Validator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::validate_semantics;
    use cairo_m_compiler_parser::{parse_program, SourceProgram};

    #[test]
    fn test_validation_framework_integration() {
        let db = test_db();

        // Test program with multiple validation issues
        let source = SourceProgram::new(
            &db,
            r#"
            func test() -> felt {
                let unused_var = 42;  // Warning: unused variable
                let used_var = 24;
                return used_var;
            }
        "#
            .to_string(),
        );

        // Run validation
        let parsed_module = parse_program(&db, source);
        let diagnostics = validate_semantics(&db, parsed_module, source);

        // Should find the unused variable
        assert!(!diagnostics.is_empty());

        let unused_warnings: Vec<_> = diagnostics
            .all()
            .iter()
            .filter(|d| d.code == DiagnosticCode::UnusedVariable)
            .collect();

        // Debug output to see what we found
        println!("Found {} unused variable warnings:", unused_warnings.len());
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
