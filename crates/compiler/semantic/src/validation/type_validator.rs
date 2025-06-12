//! # Type Validator
//!
//! This validator handles all type-related semantic validation:
//! - Binary operation type compatibility
//! - Function call argument types
//! - Struct field access and literals
//! - Array/tuple indexing types
//! - Assignment type compatibility
//! - Return type matching
//! - Conditional expression types

use crate::db::SemanticDb;
use crate::semantic_index::ExpressionInfo;
use crate::type_resolution::{are_types_compatible, expression_semantic_type};
use crate::types::{TypeData, TypeId};
use crate::validation::Validator;
use crate::{ExpressionId, File, SemanticIndex};
use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode};
use cairo_m_compiler_parser::parser::{BinaryOp, Expression, Spanned};
use std::collections::HashSet;

/// Unified validator for all type-related semantic checks
///
/// This validator ensures type safety across all language constructs by checking:
/// - Expression types are compatible with their usage context
/// - Operations are applied to appropriate types
/// - Assignments and returns match expected types
///
/// # Architecture
///
/// The validator makes a single pass through all expressions, computing types
/// once and applying all relevant type rules. This is more efficient than
/// multiple specialized validators that each re-compute types.
#[derive(Debug, Default)]
pub struct TypeValidator;

impl Validator for TypeValidator {
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Single pass through all expressions for type checking
        for (expr_id, expr_info) in index.all_expressions() {
            self.check_expression_types(db, file, index, expr_id, expr_info, &mut diagnostics);
        }

        // Check statement-level type constraints
        self.check_statement_types(db, file, index, &diagnostics);

        diagnostics
    }

    fn name(&self) -> &'static str {
        "TypeValidator"
    }
}

impl TypeValidator {
    /// Check type constraints for a single expression
    fn check_expression_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        _expr_id: ExpressionId,
        expr_info: &ExpressionInfo,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match &expr_info.ast_node {
            Expression::BinaryOp { left, op, right } => {
                self.check_binary_op_types(db, file, index, left, op, right, diagnostics);
            }
            Expression::FunctionCall { callee, args } => {
                self.check_function_call_types(db, file, index, callee, args, diagnostics);
            }
            Expression::MemberAccess { object, field } => {
                self.check_member_access_types(db, file, index, object, field, diagnostics);
            }
            Expression::IndexAccess {
                array,
                index: index_expr,
            } => {
                self.check_index_access_types(db, file, index, array, index_expr, diagnostics);
            }
            Expression::StructLiteral { name, fields } => {
                self.check_struct_literal_types(
                    db,
                    file,
                    index,
                    expr_info.scope_id,
                    name,
                    fields,
                    diagnostics,
                );
            }
            // Literals, identifiers, and tuples don't need additional type validation
            // beyond what's already done in type_resolution.rs
            _ => {}
        }
    }

    /// Validate binary operation type compatibility
    #[allow(clippy::too_many_arguments)]
    fn check_binary_op_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        left: &Spanned<Expression>,
        op: &BinaryOp,
        right: &Spanned<Expression>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(left_id) = index.expression_id_by_span(left.span()) else {
            return;
        };
        let Some(right_id) = index.expression_id_by_span(right.span()) else {
            return;
        };

        let left_type = expression_semantic_type(db, file, left_id);
        let right_type = expression_semantic_type(db, file, right_id);
        let felt_type = TypeId::new(db, TypeData::Felt);

        // For now, all binary operations require felt operands
        // TODO: Expand this when more numeric types are added
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                // Arithmetic operations
                if !are_types_compatible(db, left_type, felt_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Invalid left operand for arithmetic operator '{:?}'. Expected 'felt', found '{}'",
                                op,
                                left_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(left.span()),
                    );
                }
                if !are_types_compatible(db, right_type, felt_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Invalid right operand for arithmetic operator '{:?}'. Expected 'felt', found '{}'",
                                op,
                                right_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(right.span()),
                    );
                }
            }
            BinaryOp::Eq | BinaryOp::Neq => {
                // Comparison operations - operands must be same type
                if !are_types_compatible(db, left_type, right_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Type mismatch in comparison. Cannot compare '{}' with '{}'",
                                left_type.data(db).display_name(db),
                                right_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(right.span()),
                    );
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                // Logical operations - both operands must be felt (acting as boolean)
                if !are_types_compatible(db, left_type, felt_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Logical operator '{:?}' cannot be applied to type '{}'",
                                op,
                                left_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(left.span()),
                    );
                }
                if !are_types_compatible(db, right_type, felt_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Logical operator '{:?}' cannot be applied to type '{}'",
                                op,
                                right_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(right.span()),
                    );
                }
            }
        }
    }

    /// Validate function call types (arity + argument types)
    fn check_function_call_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(callee_expr_id) = index.expression_id_by_span(callee.span()) else {
            return;
        };
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
                    return; // Don't check argument types if arity is wrong
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
                                        "Argument type mismatch: expected '{}', found '{}'",
                                        param_type.data(db).display_name(db),
                                        arg_type.data(db).display_name(db)
                                    ),
                                )
                                .with_location(arg.span()),
                            );
                        }
                    }
                }
            }
            TypeData::Error => {
                // Skip validation for error types to avoid cascading diagnostics
                // The underlying error (e.g., undeclared function) will be reported by ScopeValidator
            }
            _ => {
                // Attempting to call a non-function type
                // But first check if this is an undeclared identifier to avoid duplicate errors
                if let Expression::Identifier(ident) = callee.value()
                    && index
                        .resolve_name_to_definition(
                            ident.value(),
                            index.expression(callee_expr_id).unwrap().scope_id,
                        )
                        .is_none()
                {
                    // This is an undeclared identifier, let ScopeValidator handle it
                    return;
                }

                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFunctionCall,
                        format!(
                            "Cannot call value of type {} as a function",
                            callee_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(callee.span()),
                );
            }
        }
    }

    /// Validate member access types
    fn check_member_access_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        object: &Spanned<Expression>,
        field: &Spanned<String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(object_id) = index.expression_id_by_span(object.span()) else {
            return;
        };
        let object_type_id = expression_semantic_type(db, file, object_id);
        let object_type = object_type_id.data(db);

        match object_type {
            TypeData::Struct(struct_type) => {
                let fields = struct_type.fields(db);
                if !fields.iter().any(|(name, _)| name == field.value()) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidFieldAccess,
                            format!(
                                "Field '{}' does not exist in struct {}",
                                field.value(),
                                struct_type.name(db)
                            ),
                        )
                        .with_location(field.span()),
                    );
                }
            }
            TypeData::Error => {
                // Skip validation for error types
            }
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldAccess,
                        format!(
                            "Expected struct type, found {}",
                            object_type_id.data(db).display_name(db)
                        ),
                    )
                    .with_location(object.span()),
                );
            }
        }
    }

    /// Validate indexing types
    fn check_index_access_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        array: &Spanned<Expression>,
        index_expr: &Spanned<Expression>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(array_id) = index.expression_id_by_span(array.span()) else {
            return;
        };
        let array_type_id = expression_semantic_type(db, file, array_id);
        let array_type = array_type_id.data(db);

        // Check if the array expression is indexable
        match array_type {
            TypeData::Tuple(_) | TypeData::Pointer(_) => {
                // Check if the index expression is an integer type
                let Some(index_id) = index.expression_id_by_span(index_expr.span()) else {
                    return;
                };
                let index_type_id = expression_semantic_type(db, file, index_id);
                let index_type = index_type_id.data(db);

                if !matches!(index_type, TypeData::Felt) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidIndexType,
                            format!(
                                "Index expression must be of type felt, found {}",
                                index_type_id.data(db).display_name(db)
                            ),
                        )
                        .with_location(index_expr.span()),
                    );
                }
            }
            TypeData::Error => {
                // Skip validation for error types
            }
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidIndexAccess,
                        format!(
                            "Type '{}' cannot be indexed",
                            array_type_id.data(db).display_name(db)
                        ),
                    )
                    .with_location(array.span()),
                );
            }
        }
    }

    /// Validate struct literal types
    #[allow(clippy::too_many_arguments)]
    fn check_struct_literal_types(
        &self,
        db: &dyn SemanticDb,
        file: File,
        index: &SemanticIndex,
        scope_id: crate::place::FileScopeId,
        name: &Spanned<String>,
        fields: &[(Spanned<String>, Spanned<Expression>)],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Resolve the struct type
        let Some((def_idx, _)) = index.resolve_name_to_definition(name.value(), scope_id) else {
            // Undeclared struct type - let ScopeValidator handle this
            return;
        };

        use crate::semantic_index::DefinitionId;
        use crate::type_resolution::definition_semantic_type;

        let def_id = DefinitionId::new(db, file, def_idx);
        let def_type = definition_semantic_type(db, def_id);

        let TypeData::Struct(struct_type) = def_type.data(db) else {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidStructLiteral,
                    format!("'{}' is not a struct type", name.value()),
                )
                .with_location(name.span()),
            );
            return;
        };

        let struct_fields = struct_type.fields(db);
        let provided_fields: HashSet<String> = fields
            .iter()
            .map(|(field_name, _)| field_name.value().clone())
            .collect();

        // Check for missing fields
        for (field_name, _field_type) in &struct_fields {
            if !provided_fields.contains(field_name) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidStructLiteral,
                        format!(
                            "Missing field '{}' in struct literal for '{}'",
                            field_name,
                            struct_type.name(db)
                        ),
                    )
                    .with_location(name.span()),
                );
            }
        }

        // Check for unknown fields and type compatibility
        for (field_name, field_value) in fields {
            if let Some((_, expected_type)) = struct_fields
                .iter()
                .find(|(name, _)| name == field_name.value())
            {
                // Check field value type compatibility
                if let Some(value_expr_id) = index.expression_id_by_span(field_value.span()) {
                    let actual_type = expression_semantic_type(db, file, value_expr_id);

                    if !are_types_compatible(db, actual_type, *expected_type) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Type mismatch for field '{}'. Expected '{}', found '{}'",
                                    field_name.value(),
                                    expected_type.data(db).display_name(db),
                                    actual_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(field_value.span()),
                        );
                    }
                }
            } else {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldAccess,
                        format!(
                            "Field '{}' does not exist in struct '{}'",
                            field_name.value(),
                            struct_type.name(db)
                        ),
                    )
                    .with_location(field_name.span()),
                );
            }
        }
    }

    /// Check statement-level type constraints
    fn check_statement_types(
        &self,
        _db: &dyn SemanticDb,
        _file: File,
        _index: &SemanticIndex,
        _diagnostics: &[Diagnostic],
    ) {
        // TODO: Implement statement-level type checking:
        // - Let statement type compatibility
        // - Assignment type compatibility
        // - Return statement type matching
        // - If condition type checking

        // This would require iterating through function bodies and checking statements
        // For now, basic expression type checking covers most cases
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::semantic_index::semantic_index;

    #[test]
    fn test_binary_op_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func returns_felt() -> felt { return 0; }
            func test() {
                let valid = 1 + 2;              // OK: felt + felt
                let point = Point { x: 1, y: 2 };
                let invalid_1 = point + 1;        // Error: struct + felt
                let valid_2 = returns_felt() + 1; // OK: felt + felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should have one error for the invalid binary operation
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .collect();

        assert_eq!(type_errors.len(), 1, "Should have one type mismatch error");
        assert!(type_errors[0].message.contains(
            "Invalid left operand for arithmetic operator 'Add'. Expected 'felt', found 'Point'"
        ));
    }

    #[test]
    fn test_function_call_type_validation() {
        let db = test_db();
        let program = r#"
            func add(x: felt, y: felt) -> felt { return x + y; }
            struct Point { x: felt, y: felt }
            func test() {
                let valid = add(1, 2);          // OK: correct types
                let point = Point { x: 1, y: 2 };
                let invalid = add(point, 1);    // Error: struct instead of felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should have one error for the invalid argument type
        let type_errors = diagnostics
            .iter()
            .filter(|d| {
                d.code == DiagnosticCode::TypeMismatch
                    && d.message.contains("Argument type mismatch")
            })
            .count();

        assert_eq!(type_errors, 1, "Should have argument type mismatch errors");
    }

    #[test]
    fn test_comprehensive_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1, y: 2 };
                let valid_access = p.x;         // OK: valid field
                let invalid_access = p.z;      // Error: invalid field

                let tuple = (1, 2, 3);
                let valid_index = tuple[0];     // OK: valid indexing
                let invalid_index = tuple[p];  // Error: struct as index

                let invalid_index2 = 42[0];    // Error: indexing non-indexable
            }
        "#;
        let file = crate::File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should catch multiple type errors
        assert!(
            diagnostics.len() >= 3,
            "Should have multiple type validation errors"
        );

        // Check for specific error types
        let field_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::InvalidFieldAccess)
            .count();
        let index_errors = diagnostics
            .iter()
            .filter(|d| {
                d.code == DiagnosticCode::InvalidIndexType
                    || d.code == DiagnosticCode::InvalidIndexAccess
            })
            .count();

        assert!(field_errors > 0, "Should have field access errors");
        assert!(index_errors > 0, "Should have indexing errors");
    }
}
