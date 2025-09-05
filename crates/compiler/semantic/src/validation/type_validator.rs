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

#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink};
use cairo_m_compiler_parser::parser::{
    parse_file, BinaryOp, Expression, FunctionDef, Pattern, Spanned, Statement, TopLevelItem,
    TypeExpr, UnaryOp,
};
use cairo_m_compiler_parser::ParsedModule;
use chumsky::span::SimpleSpan;

use crate::builtins::{is_builtin_function_name, BuiltinFn};
use crate::db::{Crate, SemanticDb};
use crate::semantic_index::ExpressionInfo;
use crate::type_resolution::{
    are_types_compatible, expression_semantic_type, get_binary_op_signatures,
    get_unary_op_signatures, resolve_ast_type,
};
use crate::types::{TypeData, TypeId};
use crate::validation::Validator;
use crate::{DefinitionKind, ExpressionId, File, SemanticIndex};

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
    fn validate(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        sink: &dyn cairo_m_compiler_diagnostics::DiagnosticSink,
    ) {
        let parsed_program = parse_file(db, file);
        if !parsed_program.diagnostics.is_empty() {
            panic!("Got unexpected parse errors");
        }
        let parsed_module = parsed_program.module;

        // Single pass through all expressions for type checking in this module only
        for (expr_id, expr_info) in index.all_expressions() {
            self.check_expression_types(db, crate_id, file, index, expr_id, expr_info, sink);
        }

        // Check all function definitions for nested arrays in their signatures
        for item in parsed_module.items() {
            if let TopLevelItem::Function(func_spanned) = item {
                let func_def = func_spanned.value();
                // Check return type for nested arrays
                Self::check_for_nested_arrays(db, file, &func_def.return_type, sink);

                // Check parameter types for nested arrays
                for param in &func_def.params {
                    Self::check_for_nested_arrays(db, file, &param.type_expr, sink);
                }
            }
        }

        for (_def_idx, definition) in index.all_definitions() {
            if let DefinitionKind::Function(_) = &definition.kind {
                self.analyze_function_statement_types(
                    db,
                    crate_id,
                    file,
                    index,
                    &parsed_module,
                    &definition.name,
                    sink,
                )
            }
        }
    }

    fn name(&self) -> &'static str {
        "TypeValidator"
    }
}

impl TypeValidator {
    fn check_builtin_assert(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        _callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        sink: &dyn DiagnosticSink,
    ) {
        // Only validate that the inner expression (if provided) evaluates to a boolean.
        // Delegate all other typing rules to the general validators.
        if let Some(cond) = args.first() {
            if let Some(cond_expr_id) = index.expression_id_by_span(cond.span()) {
                let cond_type = expression_semantic_type(db, crate_id, file, cond_expr_id, None);
                match cond_type.data(db) {
                    TypeData::Bool => {}
                    TypeData::Error | TypeData::Unknown => {
                        // Avoid cascading diagnostics for already-invalid expressions
                    }
                    other => {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!("expected bool, found `{}`", other.display_name(db)),
                            )
                            .with_location(file.file_path(db).to_string(), cond.span()),
                        );
                    }
                }
            }
        }
    }

    /// Check if a type expression contains nested arrays
    fn check_for_nested_arrays(
        db: &dyn SemanticDb,
        file: File,
        type_expr: &Spanned<TypeExpr>,
        sink: &dyn DiagnosticSink,
    ) {
        match type_expr.value() {
            TypeExpr::FixedArray { element_type, .. } => {
                // Check if the element type is also an array
                if matches!(element_type.value(), TypeExpr::FixedArray { .. }) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidTypeDefinition,
                            "Nested arrays are not supported yet".to_string(),
                        )
                        .with_location(file.file_path(db).to_string(), type_expr.span()),
                    );
                }
            }
            TypeExpr::Tuple(elements) => {
                // Recursively check tuple elements
                for elem in elements {
                    Self::check_for_nested_arrays(db, file, elem, sink);
                }
            }
            _ => {}
        }
    }

    /// Suggest possible type conversions or fixes for type mismatches
    fn suggest_type_conversion(
        &self,
        db: &dyn SemanticDb,
        from_type: TypeId,
        to_type: TypeId,
    ) -> Option<String> {
        let from_data = from_type.data(db);
        let to_data = to_type.data(db);

        match (from_data, to_data) {
            (TypeData::Struct(struct_type), TypeData::Felt) => {
                // Check if struct has a numeric field that could be used
                let fields = struct_type.fields(db);
                let numeric_fields: Vec<_> = fields
                    .iter()
                    .filter(|(_, field_type)| matches!(field_type.data(db), TypeData::Felt))
                    .map(|(name, _)| name)
                    .collect();

                if numeric_fields.len() == 1 {
                    Some(format!(
                        "Did you mean to access the `{}` field?",
                        numeric_fields[0]
                    ))
                } else if !numeric_fields.is_empty() {
                    Some("This struct has numeric fields that could be accessed".to_string())
                } else {
                    Some("Structs cannot be used in arithmetic operations".to_string())
                }
            }
            (TypeData::Tuple(elements), TypeData::Felt) => {
                if elements.len() == 1 && elements[0].data(db).is_numeric() {
                    Some("Did you mean to access the tuple element with `.0`?".to_string())
                } else {
                    Some("Tuples cannot be used directly in arithmetic operations".to_string())
                }
            }
            (TypeData::Bool, TypeData::Felt) => {
                Some("Cannot use bool in arithmetic operations. Consider using logical operators (&&, ||) instead.".to_string())
            }
            (TypeData::Function(_), _) => {
                Some("Did you forget to call the function with parentheses?".to_string())
            }
            _ => None,
        }
    }
    /// Check type constraints for a single expression
    fn check_expression_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        _expr_id: ExpressionId,
        expr_info: &ExpressionInfo,
        sink: &dyn DiagnosticSink,
    ) {
        match &expr_info.ast_node {
            Expression::UnaryOp { expr, op } => {
                self.check_unary_op_types(db, crate_id, file, index, expr, op, sink);
            }
            Expression::BinaryOp { left, op, right } => {
                self.check_binary_op_types(db, crate_id, file, index, left, op, right, sink);
            }
            Expression::FunctionCall { callee, args } => {
                // Handle built-in assert() semantics
                if let Expression::Identifier(ident) = callee.value()
                    && is_builtin_function_name(ident.value()) == Some(BuiltinFn::Assert)
                {
                    self.check_builtin_assert(db, crate_id, file, index, callee, args, sink);
                    return;
                }
                self.check_function_call_types(db, crate_id, file, index, callee, args, sink);
            }
            Expression::MemberAccess { object, field } => {
                self.check_member_access_types(db, crate_id, file, index, object, field, sink);
            }
            Expression::IndexAccess {
                array,
                index: index_expr,
            } => {
                self.check_index_access_types(db, crate_id, file, index, array, index_expr, sink);
            }
            Expression::StructLiteral { name, fields } => {
                self.check_struct_literal_types(
                    db,
                    crate_id,
                    file,
                    index,
                    expr_info.scope_id,
                    name,
                    fields,
                    sink,
                );
            }
            Expression::TupleIndex {
                tuple,
                index: tuple_index,
            } => {
                self.check_tuple_index_types(db, crate_id, file, index, tuple, *tuple_index, sink);
            }
            Expression::ArrayLiteral(elements) => {
                self.check_array_literal_types(
                    db, crate_id, file, index, elements, expr_info, sink,
                );
            }
            Expression::ArrayRepeat { element, count: _ } => {
                self.check_array_repeat_types(db, crate_id, file, index, element, expr_info, sink);
            }
            Expression::Cast { expr, target_type } => {
                self.check_cast_types(
                    db,
                    crate_id,
                    file,
                    index,
                    expr,
                    target_type,
                    expr_info,
                    sink,
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
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        left: &Spanned<Expression>,
        op: &BinaryOp,
        right: &Spanned<Expression>,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(left_id) = index.expression_id_by_span(left.span()) else {
            return;
        };
        let Some(right_id) = index.expression_id_by_span(right.span()) else {
            return;
        };

        // For commutative operators with literal operands, we need special handling
        let is_commutative = matches!(
            op,
            BinaryOp::Add | BinaryOp::Mul | BinaryOp::Eq | BinaryOp::Neq
        );

        let (left_type, right_type) = if is_commutative {
            // Check if operands are literals
            let left_expr = &index.expression(left_id).unwrap().ast_node;
            let right_expr = &index.expression(right_id).unwrap().ast_node;

            let left_is_unsuffixed_literal = matches!(left_expr, Expression::Literal(_, None));
            let right_is_unsuffixed_literal = matches!(right_expr, Expression::Literal(_, None));

            if left_is_unsuffixed_literal && !right_is_unsuffixed_literal {
                // Infer right first, then use it as context for left
                let right_type = expression_semantic_type(db, crate_id, file, right_id, None);
                let left_type =
                    expression_semantic_type(db, crate_id, file, left_id, Some(right_type));
                (left_type, right_type)
            } else {
                // Default: left first, then right with left as context
                let left_type = expression_semantic_type(db, crate_id, file, left_id, None);
                let right_type =
                    expression_semantic_type(db, crate_id, file, right_id, Some(left_type));
                (left_type, right_type)
            }
        } else {
            // Non-commutative operators
            let left_type = expression_semantic_type(db, crate_id, file, left_id, None);
            let right_type =
                expression_semantic_type(db, crate_id, file, right_id, Some(left_type));
            (left_type, right_type)
        };

        // Any un-resolved type must not trigger a type mismatch error here.
        if left_type.data(db) == TypeData::Error || right_type.data(db) == TypeData::Error {
            return;
        }

        let op_signatures = get_binary_op_signatures(db);
        let mut binary_op_on_left_type = op_signatures
            .iter()
            .filter(|op_signature| op_signature.op == *op && op_signature.left == left_type);
        if binary_op_on_left_type.clone().count() == 0 {
            let suggestion = self.suggest_type_conversion(db, left_type, right_type);
            let mut diag = Diagnostic::error(
                DiagnosticCode::TypeMismatch,
                format!(
                    "Operator `{}` is not supported for type `{}`",
                    op,
                    left_type.data(db).display_name(db)
                ),
            )
            .with_location(file.file_path(db).to_string(), left.span());
            if let Some(suggestion) = suggestion {
                diag =
                    diag.with_related_span(file.file_path(db).to_string(), left.span(), suggestion);
            }
            sink.push(diag);
            return;
        }

        let valid_signature =
            binary_op_on_left_type.find(|op_signature| op_signature.right == right_type);
        if valid_signature.is_none() {
            let suggestion = self.suggest_type_conversion(db, right_type, left_type);
            let mut diag = Diagnostic::error(
                DiagnosticCode::TypeMismatch,
                format!(
                    "Invalid right operand for arithmetic operator `{}`. Expected `{}`, found `{}`",
                    op,
                    left_type.data(db).display_name(db),
                    right_type.data(db).display_name(db)
                ),
            )
            .with_location(file.file_path(db).to_string(), right.span());
            if let Some(suggestion) = suggestion {
                diag =
                    diag.with_related_span(file.file_path(db).to_string(), left.span(), suggestion);
            }
            sink.push(diag);
        }
    }

    /// Validate unary operation type compatibility
    fn check_unary_op_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        expr: &Spanned<Expression>,
        op: &UnaryOp,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(expr_id) = index.expression_id_by_span(expr.span()) else {
            return;
        };

        let expr_type = expression_semantic_type(db, crate_id, file, expr_id, None);

        let unary_op_signatures = get_unary_op_signatures(db);
        let unary_op_on_expr_type = unary_op_signatures
            .iter()
            .find(|op_signature| op_signature.op == *op && op_signature.operand == expr_type);

        if unary_op_on_expr_type.is_none() {
            let suggestion =
                self.suggest_type_conversion(db, expr_type, TypeId::new(db, TypeData::Felt));
            let mut diag = Diagnostic::error(
                DiagnosticCode::TypeMismatch,
                format!(
                    "Operator `{}` is not supported for type `{}`",
                    op,
                    expr_type.data(db).display_name(db)
                ),
            )
            .with_location(file.file_path(db).to_string(), expr.span());
            if let Some(suggestion) = suggestion {
                diag =
                    diag.with_related_span(file.file_path(db).to_string(), expr.span(), suggestion);
            }
            sink.push(diag);
        }
    }

    /// Validate function call types (arity + argument types)
    fn check_function_call_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        sink: &dyn DiagnosticSink,
    ) {
        let Some(callee_expr_id) = index.expression_id_by_span(callee.span()) else {
            return;
        };
        let callee_type = expression_semantic_type(db, crate_id, file, callee_expr_id, None);

        match callee_type.data(db) {
            TypeData::Function(signature_id) => {
                let params = signature_id.params(db);

                // Check arity
                if args.len() != params.len() {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidFunctionCall,
                            format!(
                                "Function expects {} argument(s), but {} were provided",
                                params.len(),
                                args.len()
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), callee.span()),
                    );
                    return; // Don't check argument types if arity is wrong
                }

                // Check argument types
                for (arg_idx, (arg, (param_name, param_type))) in
                    args.iter().zip(params.iter()).enumerate()
                {
                    if let Some(arg_expr_id) = index.expression_id_by_span(arg.span()) {
                        // Pass the expected parameter type as context for literal inference
                        let arg_type = expression_semantic_type(
                            db,
                            crate_id,
                            file,
                            arg_expr_id,
                            Some(*param_type),
                        );

                        if !are_types_compatible(db, arg_type, *param_type) {
                            // Find the parameter's AST to get its span
                            let func_def_id = signature_id.definition_id(db);
                            let func_def = index.definition(func_def_id.id_in_file(db)).unwrap();
                            let param_type_span =
                                if let DefinitionKind::Function(func_ref) = &func_def.kind {
                                    func_ref.params_ast.get(arg_idx).map(|p| p.1.span())
                                } else {
                                    None
                                };

                            let mut diag = Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "argument type mismatch for parameter `{}`: expected `{}`, got `{}`",
                                    param_name,
                                    param_type.data(db).display_name(db),
                                    arg_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), arg.span());

                            if let Some(span) = param_type_span {
                                diag = diag.with_related_span(
                                    file.file_path(db).to_string(),
                                    span,
                                    format!(
                                        "parameter `{}` declared here with type `{}`",
                                        param_name,
                                        param_type.data(db).display_name(db)
                                    ),
                                );
                            }

                            sink.push(diag);
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
                if let Expression::Identifier(_ident) = callee.value()
                    && index
                        .definition_for_identifier_expr(callee_expr_id)
                        .is_none()
                {
                    // This is an undeclared identifier, let ScopeValidator handle it
                    return;
                }

                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFunctionCall,
                        format!(
                            "Cannot call value of type {} as a function",
                            callee_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), callee.span()),
                );
            }
        }
    }

    /// Validate member access types
    fn check_member_access_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        object: &Spanned<Expression>,
        field: &Spanned<String>,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(object_id) = index.expression_id_by_span(object.span()) else {
            return;
        };
        let object_type_id = expression_semantic_type(db, crate_id, file, object_id, None);
        let object_type = object_type_id.data(db);

        match object_type {
            TypeData::Struct(struct_type) => {
                let fields = struct_type.fields(db);
                if !fields.iter().any(|(name, _)| name == field.value()) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidFieldAccess,
                            format!(
                                "Field `{}` does not exist in struct `{}`",
                                field.value(),
                                struct_type.name(db)
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), field.span()),
                    );
                }
            }
            TypeData::Error => {
                // Skip validation for error types
            }
            _ => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldAccess,
                        format!(
                            "Expected struct type, found `{}`",
                            object_type_id.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), object.span()),
                );
            }
        }
    }

    /// Validate indexing types
    fn check_index_access_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        array: &Spanned<Expression>,
        index_expr: &Spanned<Expression>,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(array_id) = index.expression_id_by_span(array.span()) else {
            return;
        };
        let array_type_id = expression_semantic_type(db, crate_id, file, array_id, None);
        let array_type = array_type_id.data(db);

        // Check if the array expression is indexable
        match array_type {
            TypeData::FixedArray { size, .. } => {
                // Check if the index expression is an integer type
                let Some(index_id) = index.expression_id_by_span(index_expr.span()) else {
                    return;
                };
                let index_type_id = expression_semantic_type(db, crate_id, file, index_id, None);
                let index_type = index_type_id.data(db);

                if !matches!(index_type, TypeData::Felt) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidIndexType,
                            format!(
                                "Array index must be of type felt, found `{}`",
                                index_type_id.data(db).display_name(db)
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), index_expr.span()),
                    );
                }

                // Compile-time bounds checking for constant indices
                if let Expression::Literal(value, _) = index_expr.value() {
                    let index_value = *value as usize;
                    if index_value >= size {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::IndexOutOfBounds,
                                format!(
                                    "Index {} out of bounds for array of size {}",
                                    index_value, size
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), index_expr.span())
                            .with_related_span(
                                file.file_path(db).to_string(),
                                array.span(),
                                format!("Array has size {}", size),
                            ),
                        );
                    }
                }
                // TODO: Add bounds checking for compile-time constant expressions
            }
            TypeData::Tuple(_) => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidTupleIndexAccess,
                        "tuples must be accessed using `.index` syntax (e.g., `tup.0`), not `[]`"
                            .to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), array.span()),
                );
            }
            TypeData::Error => {
                // Skip validation for error types
            }
            _ => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidIndexAccess,
                        format!(
                            "Type `{}` cannot be indexed",
                            array_type_id.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), array.span()),
                );
            }
        }
    }

    /// Check array literal for mixed types and nested arrays
    fn check_array_literal_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        elements: &[Spanned<Expression>],
        expr_info: &ExpressionInfo,
        sink: &dyn DiagnosticSink,
    ) {
        if elements.is_empty() {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::TypeInferenceError,
                    "Empty arrays are not allowed.".to_string(),
                )
                .with_location(file.file_path(db).to_string(), expr_info.ast_span),
            );
            return;
        }

        // Check for nested array literals
        for elem in elements {
            if matches!(
                elem.value(),
                Expression::ArrayLiteral(_) | Expression::ArrayRepeat { .. }
            ) {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidTypeDefinition,
                        "Nested arrays are not supported yet".to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), expr_info.ast_span),
                );
                return; // No need to check further
            }
        }

        // Get the type of the first element
        let Some(first_elem_id) = index.expression_id_by_span(elements[0].span()) else {
            return;
        };
        let first_type = expression_semantic_type(db, crate_id, file, first_elem_id, None);

        // Check if all elements have the same type
        for (idx, elem) in elements.iter().enumerate().skip(1) {
            if let Some(elem_id) = index.expression_id_by_span(elem.span()) {
                let elem_type =
                    expression_semantic_type(db, crate_id, file, elem_id, Some(first_type));

                if !are_types_compatible(db, elem_type, first_type) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Array element at index {} has type `{}`, but expected `{}` to match first element",
                                idx,
                                elem_type.data(db).display_name(db),
                                first_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), elem.span()),
                    );
                }
            }
        }
    }

    /// Check array repeat literal: ensure element type is valid and no nesting
    fn check_array_repeat_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        element: &Spanned<Expression>,
        expr_info: &ExpressionInfo,
        sink: &dyn DiagnosticSink,
    ) {
        // Nested arrays not supported
        if matches!(
            element.value(),
            Expression::ArrayLiteral(_) | Expression::ArrayRepeat { .. }
        ) {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidTypeDefinition,
                    "Nested arrays are not supported yet".to_string(),
                )
                .with_location(file.file_path(db).to_string(), expr_info.ast_span),
            );
            return;
        }

        // If we have an expected array type, ensure element type is compatible to improve diagnostics
        if let Some(expected_ast) = &expr_info.expected_type_ast {
            if let TypeExpr::FixedArray { element_type, .. } = expected_ast.value() {
                if let Some(elem_id) = index.expression_id_by_span(element.span()) {
                    let expected_elem_ty = crate::type_resolution::resolve_ast_type(
                        db,
                        crate_id,
                        file,
                        element_type.as_ref().clone(),
                        expr_info.scope_id,
                    );
                    let actual_elem_ty = crate::type_resolution::expression_semantic_type(
                        db,
                        crate_id,
                        file,
                        elem_id,
                        Some(expected_elem_ty),
                    );
                    if !crate::type_resolution::are_types_compatible(
                        db,
                        actual_elem_ty,
                        expected_elem_ty,
                    ) {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Array element has type `{}`, but expected `{}`",
                                    actual_elem_ty.data(db).display_name(db),
                                    expected_elem_ty.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), element.span()),
                        );
                    }
                }
            }
        }
    }

    /// Validate tuple index access
    fn check_tuple_index_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        tuple: &Spanned<Expression>,
        tuple_index: usize,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(tuple_id) = index.expression_id_by_span(tuple.span()) else {
            return;
        };
        let tuple_type_id = expression_semantic_type(db, crate_id, file, tuple_id, None);

        match tuple_type_id.data(db) {
            TypeData::Tuple(elements) => {
                if tuple_index >= elements.len() {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::TupleIndexOutOfBounds,
                            format!(
                                "no field `{}` on type `{}`",
                                tuple_index,
                                tuple_type_id.data(db).display_name(db)
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), tuple.span()),
                    );
                }
            }
            TypeData::Error => {
                // Skip validation for error types
            }
            _ => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidTupleIndexAccess,
                        format!(
                            "Cannot use tuple index on type `{}`",
                            tuple_type_id.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), tuple.span()),
                );
            }
        }
    }

    /// Validate struct literal types
    #[allow(clippy::too_many_arguments)]
    fn check_struct_literal_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        scope_id: crate::place::FileScopeId,
        name: &Spanned<String>,
        fields: &[(Spanned<String>, Spanned<Expression>)],
        sink: &dyn DiagnosticSink,
    ) {
        // Resolve the struct type
        let Some((def_idx, _)) = index.resolve_name_at_position(name.value(), scope_id, name.span()) else {
            // Undeclared struct type - let ScopeValidator handle this
            return;
        };

        use crate::semantic_index::DefinitionId;
        use crate::type_resolution::definition_semantic_type;

        let def_id = DefinitionId::new(db, file, def_idx);
        let def_type = definition_semantic_type(db, crate_id, def_id);

        let TypeData::Struct(struct_type) = def_type.data(db) else {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidStructLiteral,
                    format!("`{}` is not a struct type", name.value()),
                )
                .with_location(file.file_path(db).to_string(), name.span()),
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
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidStructLiteral,
                        format!(
                            "Missing field `{}` in struct literal for `{}`",
                            field_name,
                            struct_type.name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), name.span()),
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
                    let actual_type =
                        expression_semantic_type(db, crate_id, file, value_expr_id, None);

                    if !are_types_compatible(db, actual_type, *expected_type) {
                        // Get the expression info to check the origin
                        if let Some(expr_info) = index.expression(value_expr_id) {
                            self.emit_type_mismatch_diagnostic(
                                db,
                                file,
                                expr_info,
                                expected_type.data(db).display_name(db),
                                actual_type.data(db).display_name(db),
                                sink,
                            );
                        } else {
                            // Fallback to the original diagnostic
                            sink.push(
                                Diagnostic::error(
                                    DiagnosticCode::TypeMismatch,
                                    format!(
                                        "Type mismatch for field `{}`. Expected `{}`, found `{}`",
                                        field_name.value(),
                                        expected_type.data(db).display_name(db),
                                        actual_type.data(db).display_name(db)
                                    ),
                                )
                                .with_location(file.file_path(db).to_string(), field_value.span()),
                            );
                        }
                    }
                }
            } else {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldAccess,
                        format!(
                            "Field `{}` does not exist in struct `{}`",
                            field_name.value(),
                            struct_type.name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), field_name.span()),
                );
            }
        }
    }

    /// Validate cast type compatibility
    #[allow(clippy::too_many_arguments)]
    fn check_cast_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        expr: &Spanned<Expression>,
        target_type: &Spanned<TypeExpr>,
        expr_info: &ExpressionInfo,
        sink: &dyn DiagnosticSink,
    ) {
        // Get the source expression type
        if let Some(source_expr_id) = index.expression_id_by_span(expr.span()) {
            let source_type = expression_semantic_type(db, crate_id, file, source_expr_id, None);

            // Resolve the target type
            let target_type_id =
                resolve_ast_type(db, crate_id, file, target_type.clone(), expr_info.scope_id);

            // Check if the cast is valid
            let is_valid = match (source_type.data(db), target_type_id.data(db)) {
                // Allow u32 to felt casting
                (TypeData::U32, TypeData::Felt) => true,
                // All other casts are invalid
                _ => false,
            };

            if !is_valid {
                let source_name = source_type.data(db).display_name(db);
                let target_name = target_type_id.data(db).display_name(db);

                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid cast from '{}' to '{}'. Only u32 to felt casting is currently supported.",
                            source_name,
                            target_name
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr.span()),
                );
            }
        }
    }

    /// Emit a type mismatch diagnostic with context-aware messaging based on expression origin
    fn emit_type_mismatch_diagnostic(
        &self,
        db: &dyn SemanticDb,
        file: File,
        expr_info: &ExpressionInfo,
        expected_type: String,
        actual_type: String,
        sink: &dyn DiagnosticSink,
    ) {
        use crate::semantic_index::Origin;

        match &expr_info.origin {
            Origin::StructField {
                field, field_span, ..
            } => {
                let diagnostic = Diagnostic::error(
                    DiagnosticCode::TypeMismatch,
                    format!(
                        "type mismatch for field `{}`: expected `{}`, got `{}`",
                        field, expected_type, actual_type
                    ),
                )
                .with_location(file.file_path(db).to_string(), expr_info.ast_span)
                .with_related_span(
                    file.file_path(db).to_string(),
                    *field_span,
                    "field declared here".to_string(),
                );

                sink.push(diagnostic);
            }
            Origin::TupleElem { index, .. } => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "type mismatch for tuple element #{}: expected `{}`, got `{}`",
                            index, expected_type, actual_type
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr_info.ast_span),
                );
            }
            Origin::ArrayElem { index, .. } => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "type mismatch for array element #{}: expected `{}`, got `{}`",
                            index, expected_type, actual_type
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr_info.ast_span),
                );
            }
            Origin::Arg { .. } => {
                // Function argument type mismatches are handled by check_function_call_types
                // with more detailed context, so we can skip here
            }
            Origin::AssignmentRhs { .. } => {
                // Assignment type mismatches are handled by check_assignment_types
                // with more detailed context, so we can skip here
            }
            Origin::ReturnExpr => {
                // Return type mismatches are handled by check_return_types
                // with more detailed context, so we can skip here
            }
            Origin::Condition { kind } => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "`{}` condition must be of type `bool`, but found `{}`",
                            kind, actual_type
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr_info.ast_span),
                );
            }
            Origin::Plain => {
                // Fallback to generic type mismatch message
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "type mismatch: expected `{}`, got `{}`",
                            expected_type, actual_type
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr_info.ast_span),
                );
            }
        }
    }

    /// Analyze statement types in a specific function
    fn analyze_function_statement_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        parsed_module: &ParsedModule,
        function_name: &str,
        sink: &dyn DiagnosticSink,
    ) {
        // Find the function definition in the AST
        if let Some(function_def) = self.find_function_in_module(parsed_module, function_name) {
            // Analyze each statement in the function body
            for stmt in &function_def.body {
                self.check_statement_type(db, crate_id, file, index, function_def, stmt, sink);
            }
        }
    }

    /// Check types for a single statement
    fn check_statement_type(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        stmt: &Spanned<Statement>,
        sink: &dyn DiagnosticSink,
    ) {
        match stmt.value() {
            Statement::Let {
                pattern,
                value,
                statement_type,
            } => {
                self.check_let_statement_types(
                    db,
                    crate_id,
                    file,
                    index,
                    pattern,
                    value,
                    statement_type,
                    sink,
                );
            }
            Statement::Assignment { lhs, rhs } => {
                self.check_assignment_types(db, crate_id, file, index, lhs, rhs, sink);
            }
            Statement::Return { value } => {
                self.check_return_types(
                    db,
                    crate_id,
                    file,
                    index,
                    function_def,
                    value,
                    stmt.span(),
                    sink,
                );
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_if_statement_types(
                    db,
                    crate_id,
                    file,
                    index,
                    function_def,
                    condition,
                    then_block,
                    else_block,
                    sink,
                );
            }
            Statement::Block(statements) => {
                // Recursively check statements in the block
                for stmt in statements {
                    self.check_statement_type(db, crate_id, file, index, function_def, stmt, sink);
                }
            }
            Statement::Expression(expr) => {
                // Expression statements are handled by check_expression_types
                let _expr_id = index.expression_id_by_span(expr.span());
            }
            Statement::Const(const_def) => {
                // Validate that the const value type matches the declared type
                if let Some(ref type_ast) = const_def.ty {
                    if let Some(value_expr_id) = index.expression_id_by_span(const_def.value.span())
                    {
                        // Get the declared type
                        let declared_type = resolve_ast_type(
                            db,
                            crate_id,
                            file,
                            type_ast.clone(),
                            index.scope_for_span(stmt.span()).unwrap(),
                        );

                        // Get the actual value type
                        let value_type = expression_semantic_type(
                            db,
                            crate_id,
                            file,
                            value_expr_id,
                            Some(declared_type),
                        );

                        // Check compatibility
                        if !are_types_compatible(db, value_type, declared_type) {
                            sink.push(
                                Diagnostic::error(
                                    DiagnosticCode::TypeMismatch,
                                    format!(
                                        "Type mismatch in const declaration: expected `{}`, got `{}`",
                                        declared_type.data(db).display_name(db),
                                        value_type.data(db).display_name(db)
                                    ),
                                )
                                .with_location(file.file_path(db).to_string(), const_def.value.span()),
                            );
                        }
                    }
                }
            }
            Statement::Loop { body } => {
                self.check_statement_type(db, crate_id, file, index, function_def, body, sink);
            }
            Statement::While { condition, body } => {
                // Check condition expression
                if let Some(condition_expr_id) = index.expression_id_by_span(condition.span()) {
                    if let Some(condition_info) = index.expression(condition_expr_id) {
                        self.check_expression_types(
                            db,
                            crate_id,
                            file,
                            index,
                            condition_expr_id,
                            condition_info,
                            sink,
                        );
                    }

                    // Check that condition is boolean type
                    let bool_type = TypeId::new(db, TypeData::Bool);
                    let condition_type = expression_semantic_type(
                        db,
                        crate_id,
                        file,
                        condition_expr_id,
                        Some(bool_type),
                    );

                    if !are_types_compatible(db, condition_type, bool_type) {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "While loop condition must be of type 'bool', found `{}`",
                                    condition_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), condition.span()),
                        );
                    }
                }

                self.check_statement_type(db, crate_id, file, index, function_def, body, sink);
            }
            Statement::For {
                init,
                condition,
                step,
                body,
            } => {
                // 1. Check the init statement
                self.check_statement_type(db, crate_id, file, index, function_def, init, sink);

                // 2. Condition must be bool
                if let Some(cond_expr_id) = index.expression_id_by_span(condition.span()) {
                    // Re-run expression-level checks for good diagnostics
                    if let Some(cond_info) = index.expression(cond_expr_id) {
                        self.check_expression_types(
                            db,
                            crate_id,
                            file,
                            index,
                            cond_expr_id,
                            cond_info,
                            sink,
                        );
                    }

                    let bool_ty = TypeId::new(db, TypeData::Bool);
                    let cond_ty =
                        expression_semantic_type(db, crate_id, file, cond_expr_id, Some(bool_ty));
                    if !are_types_compatible(db, cond_ty, bool_ty) {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "for loop condition must be of type 'bool', found `{}`",
                                    cond_ty.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), condition.span()),
                        );
                    }
                }

                // 3. Body
                self.check_statement_type(db, crate_id, file, index, function_def, body, sink);

                // 4. Step statement
                self.check_statement_type(db, crate_id, file, index, function_def, step, sink);
            }
            Statement::Break | Statement::Continue => {
                // No types to check for break/continue
            }
        }
    }

    /// Check types for let statements
    #[allow(clippy::too_many_arguments)]
    fn check_let_statement_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        pattern: &Pattern,
        value: &Spanned<Expression>,
        statement_type: &Option<Spanned<TypeExpr>>,
        sink: &dyn DiagnosticSink,
    ) {
        // Check for nested arrays in type annotation
        if let Some(ty) = statement_type {
            Self::check_for_nested_arrays(db, file, ty, sink);
        }
        let Some(value_expr_id) = index.expression_id_by_span(value.span()) else {
            return;
        };
        let value_type = expression_semantic_type(db, crate_id, file, value_expr_id, None);

        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier - check type if specified
                if let Some(ty) = statement_type {
                    let scope_id = index
                        .expression(value_expr_id)
                        .expect("No expression info found")
                        .scope_id;
                    let expected_type = resolve_ast_type(db, crate_id, file, ty.clone(), scope_id);
                    if !are_types_compatible(db, value_type, expected_type) {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Type mismatch for let statement `{}`. Expected `{}`, found `{}`",
                                    name.value(),
                                    expected_type.data(db).display_name(db),
                                    value_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), value.span()),
                        );
                    }
                }
            }
            Pattern::Tuple(patterns) => {
                // Tuple pattern - check that RHS is a tuple with matching structure
                match value_type.data(db) {
                    TypeData::Tuple(element_types) => {
                        if element_types.len() != patterns.len() {
                            sink.push(
                                Diagnostic::error(
                                    DiagnosticCode::TypeMismatch,
                                    format!(
                                        "Tuple pattern has {} elements but value has {} elements",
                                        patterns.len(),
                                        element_types.len()
                                    ),
                                )
                                .with_location(file.file_path(db).to_string(), value.span()),
                            );
                        }

                        // Recursively check nested patterns
                        for (pattern, elem_type) in patterns.iter().zip(element_types.iter()) {
                            if let Pattern::Tuple(nested_patterns) = pattern {
                                // Check nested tuple pattern matches nested tuple type
                                match elem_type.data(db) {
                                    TypeData::Tuple(nested_types) => {
                                        if nested_types.len() != nested_patterns.len() {
                                            sink.push(
                                                Diagnostic::error(
                                                    DiagnosticCode::TypeMismatch,
                                                    format!(
                                                        "Nested tuple pattern has {} elements but value has {} elements",
                                                        nested_patterns.len(),
                                                        nested_types.len()
                                                    ),
                                                )
                                                .with_location(file.file_path(db).to_string(), value.span()),
                                            );
                                        }
                                    }
                                    _ => {
                                        sink.push(
                                            Diagnostic::error(
                                                DiagnosticCode::TypeMismatch,
                                                format!(
                                                    "Expected tuple type for nested tuple pattern, found `{}`",
                                                    elem_type.data(db).display_name(db)
                                                ),
                                            )
                                            .with_location(file.file_path(db).to_string(), value.span()),
                                        );
                                    }
                                }
                            }
                        }

                        // If a type annotation is provided, it should be a tuple type
                        if let Some(ty) = statement_type {
                            let scope_id = index
                                .expression(value_expr_id)
                                .expect("No expression info found")
                                .scope_id;
                            let expected_type =
                                resolve_ast_type(db, crate_id, file, ty.clone(), scope_id);
                            if !are_types_compatible(db, value_type, expected_type) {
                                sink.push(
                                    Diagnostic::error(
                                        DiagnosticCode::TypeMismatch,
                                        format!(
                                            "Type mismatch for tuple destructuring. Expected `{}`, found `{}`",
                                            expected_type.data(db).display_name(db),
                                            value_type.data(db).display_name(db)
                                        ),
                                    )
                                    .with_location(file.file_path(db).to_string(), value.span()),
                                );
                            }
                        }
                    }
                    _ => {
                        sink.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Cannot destructure non-tuple type `{}` in tuple pattern",
                                    value_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), value.span()),
                        );
                    }
                }
            }
        }
    }

    /// Check types for assignment statements
    fn check_assignment_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        lhs: &Spanned<Expression>,
        rhs: &Spanned<Expression>,
        sink: &dyn DiagnosticSink,
    ) {
        let Some(lhs_expr_id) = index.expression_id_by_span(lhs.span()) else {
            return;
        };
        let Some(rhs_expr_id) = index.expression_id_by_span(rhs.span()) else {
            return;
        };

        let lhs_type = expression_semantic_type(db, crate_id, file, lhs_expr_id, None);
        // Pass LHS type as context for RHS to support literal type inference
        let rhs_type = expression_semantic_type(db, crate_id, file, rhs_expr_id, Some(lhs_type));

        // Helper: detect if an lvalue ultimately refers to a const definition
        fn lvalue_resolves_to_const(
            index: &SemanticIndex,
            expr: &Spanned<Expression>,
        ) -> Option<(String, crate::Definition)> {
            match expr.value() {
                Expression::Identifier(name) => {
                    if let Some(id) = index.expression_id_by_span(name.span()) {
                        if let Some((_def_idx, def)) = index.definition_for_identifier_expr(id) {
                            if matches!(def.kind, crate::definition::DefinitionKind::Const(_)) {
                                return Some((name.value().clone(), def.clone()));
                            }
                        } else {
                            // Fallback: attempt positional resolution (handles some edge cases)
                            let scope_id = index
                                .expression(id)
                                .expect("No expression info found")
                                .scope_id;
                            if let Some((_def_idx, def)) =
                                index.resolve_name_at_position(name.value(), scope_id, name.span())
                            {
                                if matches!(def.kind, crate::definition::DefinitionKind::Const(_)) {
                                    return Some((name.value().clone(), def.clone()));
                                }
                            }
                        }
                    }
                    None
                }
                Expression::MemberAccess { object, .. } => lvalue_resolves_to_const(index, object),
                Expression::TupleIndex { tuple, .. } => lvalue_resolves_to_const(index, tuple),
                Expression::IndexAccess { array, .. } => lvalue_resolves_to_const(index, array),
                Expression::Cast { expr, .. } => lvalue_resolves_to_const(index, expr),
                Expression::Parenthesized(inner) => lvalue_resolves_to_const(index, inner),
                _ => None,
            }
        }

        // Check if LHS is assignable
        match lhs.value() {
            Expression::Identifier(_) => {
                // Check if the identifier is mutable
                if let Expression::Identifier(ident) = lhs.value() {
                    let scope_id = index
                        .expression(lhs_expr_id)
                        .expect("No expression info found")
                        .scope_id;
                    if let Some((_def_idx, def)) =
                        index.resolve_name_at_position(ident.value(), scope_id, ident.span())
                    {
                        // Check const via definition kind
                        if matches!(def.kind, crate::definition::DefinitionKind::Const(_)) {
                            sink.push(
                                Diagnostic::error(
                                    DiagnosticCode::AssignmentToConst,
                                    format!("cannot assign to const variable `{}`", ident.value()),
                                )
                                .with_location(file.file_path(db).to_string(), lhs.span())
                                .with_related_span(
                                    file.file_path(db).to_string(),
                                    def.name_span,
                                    "const variable defined here".to_string(),
                                ),
                            );
                            return;
                        }
                    }
                }
            }
            Expression::MemberAccess {
                object: _,
                field: _,
            } => {
                // Member access (struct fields) are valid assignment targets unless rooted in const
                if let Some((name, def)) = lvalue_resolves_to_const(index, lhs) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::AssignmentToConst,
                            format!("cannot assign to field of const variable `{}`", name),
                        )
                        .with_location(file.file_path(db).to_string(), lhs.span())
                        .with_related_span(
                            file.file_path(db).to_string(),
                            def.name_span,
                            "const variable defined here".to_string(),
                        ),
                    );
                    return;
                }
            }
            Expression::TupleIndex { .. } => {
                // Tuple element is valid assignment target unless rooted in const
                if let Some((name, def)) = lvalue_resolves_to_const(index, lhs) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::AssignmentToConst,
                            format!("cannot assign to element of const variable `{}`", name),
                        )
                        .with_location(file.file_path(db).to_string(), lhs.span())
                        .with_related_span(
                            file.file_path(db).to_string(),
                            def.name_span,
                            "const variable defined here".to_string(),
                        ),
                    );
                    return;
                }
            }
            Expression::IndexAccess { array: _, index: _ } => {
                // Array element is valid assignment target unless rooted in const
                if let Some((name, def)) = lvalue_resolves_to_const(index, lhs) {
                    sink.push(
                        Diagnostic::error(
                            DiagnosticCode::AssignmentToConst,
                            format!("cannot assign to element of const variable `{}`", name),
                        )
                        .with_location(file.file_path(db).to_string(), lhs.span())
                        .with_related_span(
                            file.file_path(db).to_string(),
                            def.name_span,
                            "const variable defined here".to_string(),
                        ),
                    );
                    return;
                }
            }
            _ => {
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidAssignmentTarget,
                        "Invalid assignment target - must be a variable, field, or array element"
                            .to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), lhs.span()),
                );
                return;
            }
        }

        // Check type compatibility
        if !are_types_compatible(db, lhs_type, rhs_type) {
            let error_message = format!(
                "type mismatch in assignment: expected `{}`, got `{}`",
                lhs_type.data(db).display_name(db),
                rhs_type.data(db).display_name(db)
            );

            let mut diag = Diagnostic::error(DiagnosticCode::TypeMismatch, error_message)
                .with_location(file.file_path(db).to_string(), rhs.span());

            if let Some(suggestion) = self.suggest_type_conversion(db, rhs_type, lhs_type) {
                diag =
                    diag.with_related_span(file.file_path(db).to_string(), rhs.span(), suggestion);
            }

            diag = diag.with_related_span(
                file.file_path(db).to_string(),
                lhs.span(),
                format!(
                    "variable declared with type `{}`",
                    lhs_type.data(db).display_name(db)
                ),
            );

            sink.push(diag);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Check types for return statements
    fn check_return_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        value: &Option<Spanned<Expression>>,
        span: SimpleSpan<usize>,
        sink: &dyn DiagnosticSink,
    ) {
        let scope_id = index.root_scope().expect("No root scope found");
        let expected_return_type = resolve_ast_type(
            db,
            crate_id,
            file,
            function_def.return_type.clone(),
            scope_id,
        );

        if matches!(expected_return_type.data(db), TypeData::Unknown) {
            panic!("Expected return type is unknown");
        }

        // Check if the function expects a non-unit return type
        let expects_value = !matches!(expected_return_type.data(db), TypeData::Tuple(ref types) if types.is_empty());

        match (value, expects_value) {
            (None, true) => {
                // Missing return value when one is expected
                sink.push(
                    Diagnostic::error(
                        DiagnosticCode::MissingReturnValue,
                        "Function with return type must return a value".to_string(),
                    )
                    .with_location(file.file_path(db).to_string(), span),
                );
            }
            (Some(return_expr), _) => {
                // Check type compatibility
                let return_expr_id = index
                    .expression_id_by_span(return_expr.span())
                    .expect("Return expression not found");
                let return_type =
                    expression_semantic_type(db, crate_id, file, return_expr_id, None);

                if !are_types_compatible(db, return_type, expected_return_type) {
                    let suggestion =
                        self.suggest_type_conversion(db, return_type, expected_return_type);

                    let error_message = format!(
                        "type mismatch in return statement: expected `{}`, got `{}`",
                        expected_return_type.data(db).display_name(db),
                        return_type.data(db).display_name(db)
                    );

                    let mut diag = Diagnostic::error(DiagnosticCode::TypeMismatch, error_message)
                        .with_location(file.file_path(db).to_string(), return_expr.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            return_expr.span(),
                            suggestion,
                        );
                    }

                    // Add context about the function signature with return type span
                    diag = diag.with_related_span(
                        file.file_path(db).to_string(),
                        function_def.return_type.span(),
                        format!(
                            "function `{}` declared here to return `{}`",
                            function_def.name.value(),
                            expected_return_type.data(db).display_name(db)
                        ),
                    );

                    sink.push(diag);
                }
            }
            (None, false) => {
                // No return value for unit type - this is fine
            }
        }
    }

    /// Check types for if statements
    #[allow(clippy::too_many_arguments)]
    fn check_if_statement_types(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        condition: &Spanned<Expression>,
        then_block: &Spanned<Statement>,
        else_block: &Option<Box<Spanned<Statement>>>,
        sink: &dyn DiagnosticSink,
    ) {
        // Check condition type
        let Some(condition_expr_id) = index.expression_id_by_span(condition.span()) else {
            return;
        };
        let bool_type = TypeId::new(db, TypeData::Bool);
        let condition_type =
            expression_semantic_type(db, crate_id, file, condition_expr_id, Some(bool_type));

        if !are_types_compatible(db, condition_type, bool_type) {
            sink.push(
                Diagnostic::error(
                    DiagnosticCode::TypeMismatch,
                    format!(
                        "If condition must be of type 'bool', found `{}`",
                        condition_type.data(db).display_name(db)
                    ),
                )
                .with_location(file.file_path(db).to_string(), condition.span()),
            );
        }

        // Check then and else block types
        self.check_statement_type(db, crate_id, file, index, function_def, then_block, sink);
        if let Some(else_stmt) = else_block {
            self.check_statement_type(db, crate_id, file, index, function_def, else_stmt, sink);
        }
    }

    /// Locate a function definition by name in the parsed module.
    fn find_function_in_module<'a>(
        &self,
        parsed_module: &'a ParsedModule,
        function_name: &str,
    ) -> Option<&'a FunctionDef> {
        for item in parsed_module.items() {
            if let TopLevelItem::Function(func_spanned) = item {
                if func_spanned.value().name.value() == function_name {
                    return Some(func_spanned.value());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_diagnostics::DiagnosticCollection;

    use super::*;
    use crate::db::tests::test_db;
    use crate::module_semantic_index;

    // TODO For tests only - ideally not present there
    fn single_file_crate(db: &dyn SemanticDb, file: File) -> Crate {
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

    fn get_main_semantic_index(db: &dyn SemanticDb, crate_id: Crate) -> SemanticIndex {
        module_semantic_index(db, crate_id, "main".to_string()).unwrap()
    }

    #[test]
    fn test_binary_op_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            fn returns_felt() -> felt { return 0; }
            fn test() {
                let valid = 1 + 2;              // OK: felt + felt
                let point = Point { x: 1, y: 2 };
                let invalid_1 = point + 1;        // Error: struct + felt
                let valid_2 = returns_felt() + 1; // OK: felt + felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = sink.into_diagnostics();

        // Should have one error for the invalid binary operation
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .collect();

        assert_eq!(type_errors.len(), 1, "Should have one type mismatch error");
        assert!(type_errors[0].message.contains("`+`"));
    }

    #[test]
    fn test_function_call_type_validation() {
        let db = test_db();
        let program = r#"
            fn add(x: felt, y: felt) -> felt { return x + y; }
            struct Point { x: felt, y: felt }
            fn test() {
                let valid = add(1, 2);          // OK: correct types
                let point = Point { x: 1, y: 2 };
                let invalid = add(point, 1);    // Error: struct instead of felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = DiagnosticCollection::new(sink.into_diagnostics());

        // Should have one error for the invalid argument type
        let type_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();

        assert_eq!(type_errors, 1, "Should have argument type mismatch errors");
    }

    #[test]
    fn test_comprehensive_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            fn test() {
                let p = Point { x: 1, y: 2 };
                let valid_access = p.x;         // OK: valid field
                let invalid_access = p.z;      // Error: invalid field

                let tuple = (1, 2, 3);
                let valid_index = tuple[0];     // OK: valid indexing
                let invalid_index = tuple[p];  // Error: struct as index

                let invalid_index2 = 42[0];    // Error: indexing non-indexable
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = sink.into_diagnostics();

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

    #[test]
    fn test_return_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }

            // Valid return type functions
            fn valid_return_felt() -> felt {
                return 42;                    // OK: correct return type
            }

            fn valid_return_point() -> Point {
                return Point { x: 1, y: 2 };  // OK: correct return type
            }

            fn valid_return_conditional() -> felt {
                if (true) {
                    return 1;                 // OK: correct return type
                } else {
                    return 2;                 // OK: correct return type
                }
            }

            // Invalid return type functions
            fn invalid_return_felt() -> felt {
                return Point { x: 1, y: 2 };  // Error: wrong return type
            }

            fn invalid_return_point() -> Point {
                return 42;                    // Error: wrong return type
            }

            fn invalid_return_conditional() -> felt {
                if (false) {
                    return Point { x: 1, y: 2 }; // Error: wrong return type
                } else {
                    return 42;                 // OK: correct return type
                }
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = sink.into_diagnostics();

        // Count type mismatch errors
        let type_mismatch_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();

        assert_eq!(
            type_mismatch_errors, 3,
            "Should have 3 type mismatch errors"
        );
    }

    #[test]
    fn test_if_statement_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            fn test() {
                // Valid if statements
                if (1) {                      // Error: felt condition
                    let a = 42;
                }

                if (true) {                      // OK: bool condition
                    return 1; // Error: return type mismatch
                } else {
                    return (); // OK: unit type
                }

                if (true && false) {                 // OK: logical operation on bool
                    let b = 42;
                }

                // Invalid if statements
                if (Point { x: 1, y: 2 }) {   // Error: non-bool condition
                    let c = 42;
                }

                if ((1, 2)) {                 // Error: non-bool condition
                    let e = 42;
                }

                // Nested if statements
                if (true) {
                    if (false) {                   // OK: bool condition
                        let f = 42;
                    }
                }

                if (true) {
                    if (Point { x: 1, y: 2 }) { // Error: non-felt condition
                        let g = 42;
                    }
                }
                return;
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = DiagnosticCollection::new(sink.into_diagnostics());

        // Count type mismatch errors
        let type_mismatch_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();

        assert_eq!(
            type_mismatch_errors, 5,
            "Should have 5 type mismatch errors"
        );
    }

    #[test]
    fn test_comparison_in_conditionals() {
        let db = test_db();
        let program = r#"
            struct Point { x: u32, y: u32 }
            fn test() {
                let a = 5u32;
                let b = 10u32;

                // Valid comparisons in if conditions
                if (a < b) {                  // OK: felt < felt
                    let x = 1u32;
                }

                if (a > b) {                  // OK: felt > felt
                    let y = 2u32;
                } else if (a <= b) {          // OK: felt <= felt
                    let z = 3u32;
                }

                while a >= 0u32 {              // OK: felt >= felt
                    let w = 4u32;
                }

                // Invalid comparisons in conditions
                let point = Point { x: 1u32, y: 2u32 };
                if (point < 5u32) {              // Error: struct < felt
                    let invalid = 1u32;
                }

                if (point > 3u32) {              // Error: struct > felt
                    let invalid2 = 2u32;
                }

                // Complex valid expressions
                if (a + 1u32 < b - 1u32) {          // OK: arithmetic results are felt
                    let valid = 1u32;
                }

                if ((a < b) && (b > 0u32)) {     // OK: comparison results used in logical ops
                    let valid2 = 2u32;
                }
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = sink.into_diagnostics();

        // Count type mismatch errors
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .collect();

        // We expect 2 errors total
        assert_eq!(type_errors.len(), 2, "Should have 2 type mismatch errors");

        // All errors should be about comparison operators with structs
        assert!(
            type_errors.iter().all(|e| e.message.contains("`Point`")),
            "All errors should be about comparing structs"
        );
    }

    #[test]
    fn test_assignment_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            fn test() {
                // Valid assignments
                let a: bool = true;
                a = false;                        // OK: same type
                a = true || false;                    // OK: logical result
                a = true && false;                   // OK: logical result

                let b: Point = Point { x: 1, y: 2 };
                b = Point { x: 3, y: 4 };     // OK: same type
                b = Point { x: 1 + 2, y: 3 + 4 }; // OK: complex initialization

                // Invalid assignments
                a = Point { x: 1, y: 2 };     // Error: type mismatch
                a = (1, 2);                   // Error: type mismatch
                b = 42;                       // Error: type mismatch

                // Field assignments
                let c: Point = Point { x: 1, y: 2 };
                c.x = 3;                      // OK: felt to felt
                c.y = 4;                      // OK: felt to felt
                c.x = Point { x: 1, y: 2 };   // Error: type mismatch

                // Invalid assignment targets
                42 = 1;                       // Error: invalid target
                (1, 2) = 1;                   // Error: invalid target
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let crate_id = single_file_crate(&db, file);
        let semantic_index = get_main_semantic_index(&db, crate_id);

        let validator = TypeValidator;
        let sink = cairo_m_compiler_diagnostics::VecSink::new();
        validator.validate(&db, crate_id, file, &semantic_index, &sink);
        let diagnostics = DiagnosticCollection::new(sink.into_diagnostics());

        // Count different types of errors
        let type_mismatch_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();
        let invalid_target_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::InvalidAssignmentTarget)
            .count();

        assert_eq!(
            type_mismatch_errors, 4,
            "Should have 4 type mismatch errors"
        );
        assert_eq!(
            invalid_target_errors, 2,
            "Should have 2 invalid target errors"
        );
    }
}
