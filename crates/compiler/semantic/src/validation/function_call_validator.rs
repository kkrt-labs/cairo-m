//! # Function Call Validator
//!
//! This validator checks that function calls are valid:
//! - Function exists and is callable
//! - Correct number of arguments provided
//! - Argument types match parameter types

use crate::db::SemanticDb;
use crate::type_resolution::{are_types_compatible, expression_semantic_type};
use crate::types::TypeData;
use crate::validation::diagnostics::{Diagnostic, DiagnosticCode};
use crate::validation::Validator;
use crate::File;
use cairo_m_compiler_parser::parser::Expression;

/// Validator for function call expressions
///
/// This validator ensures that function calls (e.g., `foo(arg1, arg2)`)
/// are semantically valid by checking:
/// - The function exists and is accessible
/// - The correct number of arguments is provided
/// - Argument types match the function's parameter types
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// func add(x: felt, y: felt) -> felt { return x + y; }
///
/// let result1 = add(1, 2, 3); // Error: too many arguments
/// let result2 = add(1); // Error: too few arguments
/// let result3 = undefined_func(1); // Error: function doesn't exist
/// ```
pub struct FunctionCallValidator;

impl Validator for FunctionCallValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &crate::SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Find all function call expressions
        for (_expr_id, expr_info) in index.all_expressions() {
            if let Expression::FunctionCall { callee, args } = &expr_info.ast_node {
                // Get callee's type
                if let Some(callee_expr_id) = index.expression_id_by_span(callee.span()) {
                    let callee_type = expression_semantic_type(db, file, callee_expr_id);

                    match callee_type.data(db) {
                        TypeData::Function(signature_id) => {
                            let params = signature_id.params(db);

                            // Check arity
                            if args.len() != params.len() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        DiagnosticCode::InvalidFunctionCall,
                                        format!(
                                            "Function expects {} argument(s), but {} were provided",
                                            params.len(),
                                            args.len()
                                        ),
                                    )
                                    .with_location(callee.span()),
                                );
                                continue;
                            }

                            // Check argument types
                            for (arg, (_param_name, param_type)) in args.iter().zip(params.iter()) {
                                if let Some(arg_expr_id) = index.expression_id_by_span(arg.span()) {
                                    let arg_type = expression_semantic_type(db, file, arg_expr_id);

                                    if !are_types_compatible(db, arg_type, *param_type) {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                DiagnosticCode::TypeMismatch,
                                                format!(
                                                    "Argument type mismatch: expected '{:?}', found '{:?}'",
                                                    param_type.data(db),
                                                    arg_type.data(db)
                                                ),
                                            )
                                            .with_location(arg.span()),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            // TODO: this is a temporary fix to avoid reporting two diagnostics for the same issue; where
                            // this error is caught by both Validators. TODO: design a better diagnostic reported that
                            // is able to only report the "most important" diagnostic.

                            // Before reporting an error, check if this is a call to an undeclared function.
                            // The ScopeValidator will provide a more accurate diagnostic in this case.
                            let callee_expr_info = index
                                .expression(callee_expr_id)
                                .expect("Expression info should exist for a valid expression ID");

                            // Check if the callee is an identifier expression.
                            if let Expression::Identifier(ident) = &callee_expr_info.ast_node {
                                // Check if the identifier is unresolved in the current scope.
                                if index
                                    .resolve_name_to_definition(
                                        ident.value(),
                                        callee_expr_info.scope_id,
                                    )
                                    .is_none()
                                {
                                    // This is an undeclared identifier. ScopeValidator handles it.
                                    // Skip adding a diagnostic from this validator and move to the next expression.
                                    continue;
                                }
                            }

                            // If we are here, the callee is not an unresolved identifier, but it's still
                            // not a function. This is a valid error to report (e.g., trying to call a
                            // variable of type `felt`).
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::InvalidFunctionCall,
                                    format!(
                                        "Cannot call value of type '{:?}' as a function",
                                        callee_type.data(db)
                                    ),
                                )
                                .with_location(callee.span()),
                            );
                        }
                    }
                }
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "FunctionCallValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::semantic_index::semantic_index;

    #[test]
    fn test_valid_function_call() {
        let db = test_db();
        let program = r#"
            func add(x: felt, y: felt) -> felt { return x + y; }
            func test() {
                let result = add(1, 2);
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = FunctionCallValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid function call
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_arity_mismatch_too_many_args() {
        let db = test_db();
        let program = r#"
            func add(x: felt, y: felt) -> felt { return x + y; }
            func test() {
                let result = add(1, 2, 3);
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = FunctionCallValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidFunctionCall);
        assert!(diagnostics[0]
            .message
            .contains("expects 2 argument(s), but 3 were provided"));
    }

    #[test]
    fn test_arity_mismatch_too_few_args() {
        let db = test_db();
        let program = r#"
            func add(x: felt, y: felt) -> felt { return x + y; }
            func test() {
                let result = add(1);
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = FunctionCallValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidFunctionCall);
        assert!(diagnostics[0]
            .message
            .contains("expects 2 argument(s), but 1 were provided"));
    }

    #[test]
    fn test_non_callable_type() {
        let db = test_db();
        let program = r#"
            func test() {
                let x = 42;
                let result = x(1, 2);
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = FunctionCallValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidFunctionCall);
        assert!(diagnostics[0].message.contains("Cannot call value of type"));
    }

    #[test]
    fn test_undeclared_function_suppressed() {
        let db = test_db();
        let program = r#"
            func test() {
                let result = undefined_function(42);
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = FunctionCallValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics because the undeclared function
        // diagnostic should be suppressed (handled by ScopeValidator)
        assert!(diagnostics.is_empty());
    }
}
