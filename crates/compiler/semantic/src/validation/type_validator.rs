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

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode};
use cairo_m_compiler_parser::ParsedModule;
use cairo_m_compiler_parser::parser::{
    BinaryOp, Expression, FunctionDef, Pattern, Spanned, Statement, TopLevelItem, TypeExpr,
    UnaryOp, parse_file,
};
use chumsky::span::SimpleSpan;

use crate::db::{Project, SemanticDb};
use crate::semantic_index::ExpressionInfo;
use crate::type_resolution::{are_types_compatible, expression_semantic_type, resolve_ast_type};
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
        project: Project,
        file: File,
        index: &SemanticIndex,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let parsed_program = parse_file(db, file);
        if !parsed_program.diagnostics.is_empty() {
            panic!("Got unexpected parse errors");
        }
        let parsed_module = parsed_program.module;

        // Single pass through all expressions for type checking in this module only
        for (expr_id, expr_info) in index.all_expressions() {
            self.check_expression_types(
                db,
                project,
                file,
                index,
                expr_id,
                expr_info,
                &mut diagnostics,
            );
        }

        for (_def_idx, definition) in index.all_definitions() {
            if let DefinitionKind::Function(_) = &definition.kind {
                self.analyze_function_statement_types(
                    db,
                    project,
                    file,
                    index,
                    &parsed_module,
                    &definition.name,
                    &mut diagnostics,
                )
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "TypeValidator"
    }
}

impl TypeValidator {
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
                        "Did you mean to access the '{}' field?",
                        numeric_fields[0]
                    ))
                } else if !numeric_fields.is_empty() {
                    Some("This struct has numeric fields that could be accessed".to_string())
                } else {
                    Some("Structs cannot be used in arithmetic operations".to_string())
                }
            }
            (TypeData::Tuple(elements), TypeData::Felt) => {
                if elements.len() == 1 && matches!(elements[0].data(db), TypeData::Felt) {
                    Some("Did you mean to access the tuple element with [0]?".to_string())
                } else {
                    Some("Tuples cannot be used directly in arithmetic operations".to_string())
                }
            }
            (TypeData::Pointer(_), TypeData::Felt) => {
                Some("Dereference the pointer to access its value".to_string())
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
        project: Project,
        file: File,
        index: &SemanticIndex,
        _expr_id: ExpressionId,
        expr_info: &ExpressionInfo,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match &expr_info.ast_node {
            Expression::UnaryOp { expr, op } => {
                self.check_unary_op_types(db, project, file, index, expr, op, diagnostics);
            }
            Expression::BinaryOp { left, op, right } => {
                self.check_binary_op_types(db, project, file, index, left, op, right, diagnostics);
            }
            Expression::FunctionCall { callee, args } => {
                self.check_function_call_types(db, project, file, index, callee, args, diagnostics);
            }
            Expression::MemberAccess { object, field } => {
                self.check_member_access_types(
                    db,
                    project,
                    file,
                    index,
                    object,
                    field,
                    diagnostics,
                );
            }
            Expression::IndexAccess {
                array,
                index: index_expr,
            } => {
                self.check_index_access_types(
                    db,
                    project,
                    file,
                    index,
                    array,
                    index_expr,
                    diagnostics,
                );
            }
            Expression::StructLiteral { name, fields } => {
                self.check_struct_literal_types(
                    db,
                    project,
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
        project: Project,
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

        let left_type = expression_semantic_type(db, project, file, left_id);
        let right_type = expression_semantic_type(db, project, file, right_id);
        let felt_type = TypeId::new(db, TypeData::Felt);

        // For now, all binary operations require felt operands
        // TODO: Expand this when more numeric types are added
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                // Arithmetic operations
                if !are_types_compatible(db, left_type, felt_type) {
                    let suggestion = self.suggest_type_conversion(db, left_type, felt_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid left operand for arithmetic operator '{:?}'. Expected 'felt', found '{}'",
                            op,
                            left_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), left.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            left.span(),
                            suggestion,
                        );
                    }

                    diagnostics.push(diag);
                }
                if !are_types_compatible(db, right_type, felt_type) {
                    let suggestion = self.suggest_type_conversion(db, right_type, felt_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid right operand for arithmetic operator '{:?}'. Expected 'felt', found '{}'",
                            op,
                            right_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), right.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            right.span(),
                            suggestion,
                        );
                    }

                    diagnostics.push(diag);
                }
            }
            BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Less
            | BinaryOp::Greater
            | BinaryOp::LessEqual
            | BinaryOp::GreaterEqual => {
                // Relational comparison operations - both operands must be felt
                if !are_types_compatible(db, left_type, felt_type) {
                    let suggestion = self.suggest_type_conversion(db, left_type, felt_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid left operand for comparison operator '{:?}'. Expected 'felt', found '{}'",
                            op,
                            left_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), left.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            left.span(),
                            suggestion,
                        );
                    }

                    diagnostics.push(diag);
                }
                if !are_types_compatible(db, right_type, felt_type) {
                    let suggestion = self.suggest_type_conversion(db, right_type, felt_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid right operand for comparison operator '{:?}'. Expected 'felt', found '{}'",
                            op,
                            right_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), right.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            right.span(),
                            suggestion,
                        );
                    }

                    diagnostics.push(diag);
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
                        .with_location(file.file_path(db).to_string(), left.span()),
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
                        .with_location(file.file_path(db).to_string(), right.span()),
                    );
                }
            }
        }
    }

    /// Validate unary operation type compatibility
    fn check_unary_op_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        expr: &Spanned<Expression>,
        op: &UnaryOp,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(expr_id) = index.expression_id_by_span(expr.span()) else {
            return;
        };

        let expr_type = expression_semantic_type(db, project, file, expr_id);
        let felt_type = TypeId::new(db, TypeData::Felt);

        // For now, all unary operations require felt operands
        match op {
            UnaryOp::Neg => {
                // Arithmetic negation
                if !are_types_compatible(db, expr_type, felt_type) {
                    let suggestion = self.suggest_type_conversion(db, expr_type, felt_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::TypeMismatch,
                        format!(
                            "Invalid operand for negation operator '-'. Expected 'felt', found '{}'",
                            expr_type.data(db).display_name(db)
                        ),
                    )
                    .with_location(file.file_path(db).to_string(), expr.span());

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            expr.span(),
                            suggestion,
                        );
                    }

                    diagnostics.push(diag);
                }
            }
            UnaryOp::Not => {
                // Logical not - operand must be felt (acting as boolean)
                if !are_types_compatible(db, expr_type, felt_type) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::TypeMismatch,
                            format!(
                                "Logical not operator '!' cannot be applied to type '{}'",
                                expr_type.data(db).display_name(db)
                            ),
                        )
                        .with_location(file.file_path(db).to_string(), expr.span()),
                    );
                }
            }
        }
    }

    /// Validate function call types (arity + argument types)
    fn check_function_call_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(callee_expr_id) = index.expression_id_by_span(callee.span()) else {
            return;
        };
        let callee_type = expression_semantic_type(db, project, file, callee_expr_id);

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
                        .with_location(file.file_path(db).to_string(), callee.span()),
                    );
                    return; // Don't check argument types if arity is wrong
                }

                // Check argument types
                for (arg, (_param_name, param_type)) in args.iter().zip(params.iter()) {
                    if let Some(arg_expr_id) = index.expression_id_by_span(arg.span()) {
                        let arg_type = expression_semantic_type(db, project, file, arg_expr_id);

                        if !are_types_compatible(db, arg_type, *param_type) {
                            let suggestion =
                                self.suggest_type_conversion(db, arg_type, *param_type);
                            let mut diag = Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Argument type mismatch for parameter '{}': expected '{}', found '{}'",
                                    _param_name,
                                    param_type.data(db).display_name(db),
                                    arg_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), arg.span());

                            if let Some(suggestion) = suggestion {
                                diag = diag.with_related_span(
                                    file.file_path(db).to_string(),
                                    arg.span(),
                                    suggestion,
                                );
                            }

                            diagnostics.push(diag);
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
                    .with_location(file.file_path(db).to_string(), callee.span()),
                );
            }
        }
    }

    /// Validate member access types
    fn check_member_access_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        object: &Spanned<Expression>,
        field: &Spanned<String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(object_id) = index.expression_id_by_span(object.span()) else {
            return;
        };
        let object_type_id = expression_semantic_type(db, project, file, object_id);
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
                        .with_location(file.file_path(db).to_string(), field.span()),
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
                    .with_location(file.file_path(db).to_string(), object.span()),
                );
            }
        }
    }

    /// Validate indexing types
    fn check_index_access_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        array: &Spanned<Expression>,
        index_expr: &Spanned<Expression>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(array_id) = index.expression_id_by_span(array.span()) else {
            return;
        };
        let array_type_id = expression_semantic_type(db, project, file, array_id);
        let array_type = array_type_id.data(db);

        // Check if the array expression is indexable
        match array_type {
            TypeData::Tuple(_) | TypeData::Pointer(_) => {
                // Check if the index expression is an integer type
                let Some(index_id) = index.expression_id_by_span(index_expr.span()) else {
                    return;
                };
                let index_type_id = expression_semantic_type(db, project, file, index_id);
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
                        .with_location(file.file_path(db).to_string(), index_expr.span()),
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
                    .with_location(file.file_path(db).to_string(), array.span()),
                );
            }
        }
    }

    /// Validate struct literal types
    #[allow(clippy::too_many_arguments)]
    fn check_struct_literal_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
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
        let def_type = definition_semantic_type(db, project, def_id);

        let TypeData::Struct(struct_type) = def_type.data(db) else {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidStructLiteral,
                    format!("'{}' is not a struct type", name.value()),
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
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidStructLiteral,
                        format!(
                            "Missing field '{}' in struct literal for '{}'",
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
                    let actual_type = expression_semantic_type(db, project, file, value_expr_id);

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
                            .with_location(file.file_path(db).to_string(), field_value.span()),
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
                    .with_location(file.file_path(db).to_string(), field_name.span()),
                );
            }
        }
    }

    /// Analyze statement types in a specific function
    fn analyze_function_statement_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        parsed_module: &ParsedModule,
        function_name: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Find the function definition in the AST
        if let Some(function_def) = self.find_function_in_module(parsed_module, function_name) {
            // Analyze each statement in the function body
            for stmt in &function_def.body {
                self.check_statement_type(
                    db,
                    project,
                    file,
                    index,
                    function_def,
                    stmt,
                    diagnostics,
                );
            }
        }
    }

    /// Check types for a single statement
    fn check_statement_type(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        stmt: &Spanned<Statement>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match stmt.value() {
            Statement::Let {
                pattern,
                value,
                statement_type,
            } => {
                self.check_let_statement_types(
                    db,
                    project,
                    file,
                    index,
                    pattern,
                    value,
                    statement_type,
                    diagnostics,
                );
            }
            Statement::Local { pattern, value, ty } => {
                self.check_local_statement_types(
                    db,
                    project,
                    file,
                    index,
                    pattern,
                    value,
                    ty,
                    diagnostics,
                );
            }
            Statement::Assignment { lhs, rhs } => {
                self.check_assignment_types(db, project, file, index, lhs, rhs, diagnostics);
            }
            Statement::Return { value } => {
                self.check_return_types(
                    db,
                    project,
                    file,
                    index,
                    function_def,
                    value,
                    stmt.span(),
                    diagnostics,
                );
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_if_statement_types(
                    db,
                    project,
                    file,
                    index,
                    function_def,
                    condition,
                    then_block,
                    else_block,
                    diagnostics,
                );
            }
            Statement::Block(statements) => {
                // Recursively check statements in the block
                for stmt in statements {
                    self.check_statement_type(
                        db,
                        project,
                        file,
                        index,
                        function_def,
                        stmt,
                        diagnostics,
                    );
                }
            }
            Statement::Expression(expr) => {
                // Expression statements are handled by check_expression_types
                let _expr_id = index.expression_id_by_span(expr.span());
            }
            Statement::Const(_) => {
                // Const statements are handled during definition processing
            }
            Statement::Loop { body } => {
                self.check_statement_type(
                    db,
                    project,
                    file,
                    index,
                    function_def,
                    body,
                    diagnostics,
                );
            }
            Statement::While { condition, body } => {
                // Check condition expression
                if let Some(condition_expr_id) = index.expression_id_by_span(condition.span()) {
                    if let Some(condition_info) = index.expression(condition_expr_id) {
                        self.check_expression_types(
                            db,
                            project,
                            file,
                            index,
                            condition_expr_id,
                            condition_info,
                            diagnostics,
                        );
                    }

                    // TODO: change this to check bool type once implemented
                    // Check that condition is boolean type (felt)
                    let condition_type =
                        expression_semantic_type(db, project, file, condition_expr_id);
                    let felt_type = TypeId::new(db, TypeData::Felt);

                    if !are_types_compatible(db, condition_type, felt_type) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "While loop condition must be of type felt, found '{}'",
                                    condition_type.data(db).display_name(db)
                                ),
                            )
                            .with_location(file.file_path(db).to_string(), condition.span()),
                        );
                    }
                }

                self.check_statement_type(
                    db,
                    project,
                    file,
                    index,
                    function_def,
                    body,
                    diagnostics,
                );
            }
            Statement::For { .. } => {
                // TODO: For loops not yet supported - we need iterator/range types first
                panic!("For loops are not yet supported - need iterator/range types");
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
        project: Project,
        file: File,
        index: &SemanticIndex,
        pattern: &Pattern,
        value: &Spanned<Expression>,
        statement_type: &Option<TypeExpr>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(value_expr_id) = index.expression_id_by_span(value.span()) else {
            return;
        };
        let value_type = expression_semantic_type(db, project, file, value_expr_id);

        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier - check type if specified
                if let Some(ty) = statement_type {
                    let scope_id = index
                        .expression(value_expr_id)
                        .expect("No expression info found")
                        .scope_id;
                    let expected_type = resolve_ast_type(db, project, file, ty.clone(), scope_id);
                    if !are_types_compatible(db, value_type, expected_type) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Type mismatch for let statement '{}'. Expected '{}', found '{}'",
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
            Pattern::Tuple(names) => {
                // Tuple pattern - check that RHS is a tuple with matching arity
                match value_type.data(db) {
                    TypeData::Tuple(element_types) => {
                        if element_types.len() != names.len() {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::TypeMismatch,
                                    format!(
                                        "Tuple pattern has {} elements but value has {} elements",
                                        names.len(),
                                        element_types.len()
                                    ),
                                )
                                .with_location(file.file_path(db).to_string(), value.span()),
                            );
                        }

                        // If a type annotation is provided, it should be a tuple type
                        if let Some(ty) = statement_type {
                            let scope_id = index
                                .expression(value_expr_id)
                                .expect("No expression info found")
                                .scope_id;
                            let expected_type =
                                resolve_ast_type(db, project, file, ty.clone(), scope_id);
                            if !are_types_compatible(db, value_type, expected_type) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        DiagnosticCode::TypeMismatch,
                                        format!(
                                            "Type mismatch for tuple destructuring. Expected '{}', found '{}'",
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
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Cannot destructure non-tuple type '{}' in tuple pattern",
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

    /// Check types for local statements
    #[allow(clippy::too_many_arguments)]
    fn check_local_statement_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        pattern: &Pattern,
        value: &Spanned<Expression>,
        ty: &Option<TypeExpr>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(value_expr_id) = index.expression_id_by_span(value.span()) else {
            return;
        };
        let value_type = expression_semantic_type(db, project, file, value_expr_id);

        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier - check type if specified
                if let Some(expected_type_expr) = ty {
                    let scope_id = index
                        .expression(value_expr_id)
                        .expect("No expression info found")
                        .scope_id;
                    let expected_type =
                        resolve_ast_type(db, project, file, expected_type_expr.clone(), scope_id);
                    if !are_types_compatible(db, value_type, expected_type) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Type mismatch for local statement '{}'. Expected '{}', found '{}'",
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
            Pattern::Tuple(names) => {
                // Tuple pattern - check that RHS is a tuple with matching arity
                match value_type.data(db) {
                    TypeData::Tuple(element_types) => {
                        if element_types.len() != names.len() {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::TypeMismatch,
                                    format!(
                                        "Tuple pattern has {} elements but value has {} elements",
                                        names.len(),
                                        element_types.len()
                                    ),
                                )
                                .with_location(file.file_path(db).to_string(), value.span()),
                            );
                        }

                        // If a type annotation is provided, it should be a tuple type
                        if let Some(expected_type_expr) = ty {
                            let scope_id = index
                                .expression(value_expr_id)
                                .expect("No expression info found")
                                .scope_id;
                            let expected_type = resolve_ast_type(
                                db,
                                project,
                                file,
                                expected_type_expr.clone(),
                                scope_id,
                            );
                            if !are_types_compatible(db, value_type, expected_type) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        DiagnosticCode::TypeMismatch,
                                        format!(
                                            "Type mismatch for tuple destructuring. Expected '{}', found '{}'",
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
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::TypeMismatch,
                                format!(
                                    "Cannot destructure non-tuple type '{}' in tuple pattern",
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
        project: Project,
        file: File,
        index: &SemanticIndex,
        lhs: &Spanned<Expression>,
        rhs: &Spanned<Expression>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(lhs_expr_id) = index.expression_id_by_span(lhs.span()) else {
            return;
        };
        let Some(rhs_expr_id) = index.expression_id_by_span(rhs.span()) else {
            return;
        };

        let lhs_type = expression_semantic_type(db, project, file, lhs_expr_id);
        let rhs_type = expression_semantic_type(db, project, file, rhs_expr_id);

        // Check if LHS is assignable
        match lhs.value() {
            Expression::Identifier(_) => {
                // Check if the identifier is mutable
                if let Expression::Identifier(ident) = lhs.value() {
                    let scope_id = index
                        .expression(lhs_expr_id)
                        .expect("No expression info found")
                        .scope_id;
                    if let Some((_def_idx, _def)) =
                        index.resolve_name_to_definition(ident.value(), scope_id)
                    {
                        // TODO: Check if the definition is mutable
                    }
                }
            }
            Expression::MemberAccess {
                object: _,
                field: _,
            } => {
                // TODO: Check if the field is mutable
            }
            Expression::IndexAccess { array: _, index: _ } => {
                // TODO: Check if the array element is mutable
            }
            _ => {
                diagnostics.push(
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
            let suggestion = self.suggest_type_conversion(db, rhs_type, lhs_type);
            let mut diag = Diagnostic::error(
                DiagnosticCode::TypeMismatch,
                format!(
                    "Type mismatch in assignment. Cannot assign '{}' to variable of type '{}'",
                    rhs_type.data(db).display_name(db),
                    lhs_type.data(db).display_name(db)
                ),
            )
            .with_location(file.file_path(db).to_string(), rhs.span());

            if let Some(suggestion) = suggestion {
                diag =
                    diag.with_related_span(file.file_path(db).to_string(), rhs.span(), suggestion);
            }

            // Add context about the target variable type
            diag = diag.with_related_span(
                file.file_path(db).to_string(),
                lhs.span(),
                format!("Variable has type '{}'", lhs_type.data(db).display_name(db)),
            );

            diagnostics.push(diag);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Check types for return statements
    fn check_return_types(
        &self,
        db: &dyn SemanticDb,
        project: Project,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        value: &Option<Spanned<Expression>>,
        span: SimpleSpan<usize>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let scope_id = index.root_scope().expect("No root scope found");
        let expected_return_type =
            resolve_ast_type(db, file, function_def.return_type.clone(), scope_id);

        if matches!(expected_return_type.data(db), TypeData::Unknown) {
            panic!("Expected return type is unknown");
        }

        // Check if the function expects a non-unit return type
        let expects_value = !matches!(expected_return_type.data(db), TypeData::Tuple(ref types) if types.is_empty());

        match (value, expects_value) {
            (None, true) => {
                // Missing return value when one is expected
                diagnostics.push(
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
                let return_type = expression_semantic_type(db, file, return_expr_id);

                if !are_types_compatible(db, return_type, expected_return_type) {
                    let suggestion =
                        self.suggest_type_conversion(db, return_type, expected_return_type);

                    let error_message = if expects_value {
                        format!(
                            "Type mismatch in return statement. Function expects '{}', but returning '{}'",
                            expected_return_type.data(db).display_name(db),
                            return_type.data(db).display_name(db)
                        )
                    } else {
                        format!(
                            "Function '{}' returns no value (unit type), but found return statement with type '{}'",
                            function_def.name.value(),
                            return_type.data(db).display_name(db)
                        )
                    };

                    let mut diag = Diagnostic::error(DiagnosticCode::TypeMismatch, error_message)
                        .with_location(file.file_path(db).to_string(), span);

                    if let Some(suggestion) = suggestion {
                        diag = diag.with_related_span(
                            file.file_path(db).to_string(),
                            return_expr.span(),
                            suggestion,
                        );
                    }

                    // Add context about the function signature
                    let context_message = if expects_value {
                        format!(
                            "Function '{}' declared here with return type '{}'",
                            function_def.name.value(),
                            expected_return_type.data(db).display_name(db)
                        )
                    } else {
                        format!(
                            "Function '{}' declared here without explicit return type (implicitly returns unit)",
                            function_def.name.value()
                        )
                    };

                    diag = diag.with_related_span(
                        file.file_path(db).to_string(),
                        function_def.name.span(),
                        context_message,
                    );

                    diagnostics.push(diag);
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
        project: Project,
        file: File,
        index: &SemanticIndex,
        function_def: &FunctionDef,
        condition: &Spanned<Expression>,
        then_block: &Spanned<Statement>,
        else_block: &Option<Box<Spanned<Statement>>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check condition type
        let Some(condition_expr_id) = index.expression_id_by_span(condition.span()) else {
            return;
        };
        let condition_type = expression_semantic_type(db, project, file, condition_expr_id);
        let felt_type = TypeId::new(db, TypeData::Felt);

        if !are_types_compatible(db, condition_type, felt_type) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::TypeMismatch,
                    format!(
                        "Condition must be of type felt, found '{}'",
                        condition_type.data(db).display_name(db)
                    ),
                )
                .with_location(file.file_path(db).to_string(), condition.span()),
            );
        }

        // Check then and else block types
        self.check_statement_type(
            db,
            project,
            file,
            index,
            function_def,
            then_block,
            diagnostics,
        );
        if let Some(else_stmt) = else_block {
            self.check_statement_type(
                db,
                project,
                file,
                index,
                function_def,
                else_stmt,
                diagnostics,
            );
        }
    }

    /// Locate a function definition by name in the parsed module.
    fn find_function_in_module<'a>(
        &self,
        parsed_module: &'a ParsedModule,
        function_name: &str,
    ) -> Option<&'a FunctionDef> {
        for item in parsed_module.items() {
            match item {
                TopLevelItem::Function(func_spanned) => {
                    if func_spanned.value().name.value() == function_name {
                        return Some(func_spanned.value());
                    }
                }
                TopLevelItem::Namespace(namespace_spanned) => {
                    // Recursively search namespaces.
                    let namespace = namespace_spanned.value();
                    for namespace_item in &namespace.body {
                        if let TopLevelItem::Function(func_spanned) = namespace_item
                            && func_spanned.value().name.value() == function_name
                        {
                            return Some(func_spanned.value());
                        }
                    }
                }
                _ => {} // Ignore other top-level items.
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::db::tests::test_db;

    fn single_file_project(db: &dyn SemanticDb, file: File) -> Project {
        let mut modules = HashMap::new();
        modules.insert("main".to_string(), file);
        Project::new(db, modules, "main".to_string())
    }

    fn get_main_semantic_index(db: &dyn SemanticDb, project: Project) -> SemanticIndex {
        let semantic_index = crate::db::project_semantic_index(db, project).unwrap();
        semantic_index.modules().get("main").unwrap().clone()
    }

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
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
    fn test_let_statement_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                // Let statement tests
                let a: felt = 1;              // OK: correct type
                let b: Point = Point { x: 1, y: 2 }; // OK: correct type
                let c = 42;                   // OK: type inference
                let d = Point { x: 1, y: 2 }; // OK: type inference

                // Invalid let statements
                let e: felt = Point { x: 1, y: 2 }; // Error: type mismatch
                let f: Point = 42;            // Error: type mismatch
                let h: Point = (1, 2);        // Error: type mismatch
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
    fn test_return_type_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }

            // Valid return type functions
            func valid_return_felt() -> felt {
                return 42;                    // OK: correct return type
            }

            func valid_return_point() -> Point {
                return Point { x: 1, y: 2 };  // OK: correct return type
            }

            func valid_return_conditional() -> felt {
                if (1) {
                    return 1;                 // OK: correct return type
                } else {
                    return 2;                 // OK: correct return type
                }
            }

            // Invalid return type functions
            func invalid_return_felt() -> felt {
                return Point { x: 1, y: 2 };  // Error: wrong return type
            }

            func invalid_return_point() -> Point {
                return 42;                    // Error: wrong return type
            }

            func invalid_return_conditional() -> felt {
                if (1) {
                    return Point { x: 1, y: 2 }; // Error: wrong return type
                } else {
                    return 42;                 // OK: correct return type
                }
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
            func test() {
                // Valid if statements
                if (1) {                      // OK: felt condition
                    let a = 42;
                }

                if (1) {                      // OK: felt condition
                    return 1; // Error: return type mismatch
                } else {
                    return (); // OK: unit type
                }

                if (1 && 2) {                 // OK: logical operation on felt
                    let b = 42;
                }

                // Invalid if statements
                if (Point { x: 1, y: 2 }) {   // Error: non-felt condition
                    let c = 42;
                }

                if ((1, 2)) {                 // Error: non-felt condition
                    let e = 42;
                }

                // Nested if statements
                if (1) {
                    if (2) {                   // OK: felt condition
                        let f = 42;
                    }
                }

                if (1) {
                    if (Point { x: 1, y: 2 }) { // Error: non-felt condition
                        let g = 42;
                    }
                }
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

        // Count type mismatch errors
        let type_mismatch_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();

        assert_eq!(
            type_mismatch_errors, 4,
            "Should have 4 type mismatch errors"
        );
    }

    #[test]
    fn test_local_statement_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                // Valid local statements
                local a: felt = 1;            // OK: correct type
                local b: Point = Point { x: 1, y: 2 }; // OK: correct type
                local c = 42;                 // OK: type inference
                local d = Point { x: 1, y: 2 }; // OK: type inference

                // Invalid local statements
                local e: felt = Point { x: 1, y: 2 }; // Error: type mismatch
                local f: Point = 42;          // Error: type mismatch
                local h: Point = (1, 2);      // Error: type mismatch

                // Local statements with expressions
                local i: felt = 1 + 2;        // OK: arithmetic result
                local j: felt = 1 && 2;       // OK: logical result
                local k: Point = Point { x: 1 + 2, y: 3 + 4 }; // OK: complex initialization
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
    fn test_comparison_operators_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                // Valid comparisons - all with felt operands
                let a = 1 < 2;                // OK: felt < felt
                let b = 3 > 1;                // OK: felt > felt
                let c = 5 <= 5;               // OK: felt <= felt
                let d = 10 >= 8;              // OK: felt >= felt
                let e = 1 == 1;               // OK: felt == felt
                let f = 2 != 3;               // OK: felt != felt

                // Invalid comparisons - struct with felt
                let point = Point { x: 1, y: 2 };
                let invalid_1 = point < 1;     // Error: struct < felt
                let invalid_2 = 1 > point;     // Error: felt > struct
                let invalid_3 = point <= 5;    // Error: struct <= felt
                let invalid_4 = 10 >= point;   // Error: felt >= struct

                // Invalid comparisons - tuple with felt
                let tuple = (1, 2);
                let invalid_5 = tuple < 3;     // Error: tuple < felt
                let invalid_6 = 4 > tuple;     // Error: felt > tuple

                // Invalid equality/inequality with non-felt types
                let eq_1 = point == Point { x: 1, y: 2 }; // Error: struct == struct
                let neq_1 = tuple != (3, 4);   // Error: tuple != tuple

                // Valid equality/inequality with felt types
                let eq_2 = 5 == 5;             // OK: felt == felt
                let neq_2 = 3 != 4;            // OK: felt != felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

        // Count type mismatch errors
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .collect();

        assert_eq!(
            type_errors.len(),
            10,
            "Should have 10 type mismatch errors for invalid comparisons"
        );

        // Check that errors mention comparison operators
        let comparison_errors = type_errors
            .iter()
            .filter(|e| e.message.contains("comparison operator"))
            .count();

        assert_eq!(
            comparison_errors, 10,
            "All 10 errors should be for comparison operators"
        );
    }

    #[test]
    fn test_comparison_in_conditionals() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let a = 5;
                let b = 10;

                // Valid comparisons in if conditions
                if (a < b) {                  // OK: felt < felt
                    let x = 1;
                }

                if (a > b) {                  // OK: felt > felt
                    let y = 2;
                } else if (a <= b) {          // OK: felt <= felt
                    let z = 3;
                }

                while (a >= 0) {              // OK: felt >= felt
                    let w = 4;
                }

                // Invalid comparisons in conditions
                let point = Point { x: 1, y: 2 };
                if (point < 5) {              // Error: struct < felt
                    let invalid = 1;
                }

                if (point > 3) {              // Error: struct > felt
                    let invalid2 = 2;
                }

                // Complex valid expressions
                if (a + 1 < b - 1) {          // OK: arithmetic results are felt
                    let valid = 1;
                }

                if ((a < b) && (b > 0)) {     // OK: comparison results used in logical ops
                    let valid2 = 2;
                }
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

        // Count type mismatch errors
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .collect();

        // We expect 2 errors total
        assert_eq!(type_errors.len(), 2, "Should have 2 type mismatch errors");

        // All errors should be about comparison operators with structs
        assert!(type_errors.iter().all(|e|
            e.message.contains("comparison operator") &&
            e.message.contains("'Point'")
        ), "All errors should be about comparing structs");
    }

    #[test]
    fn test_mixed_operators_with_comparisons() {
        let db = test_db();
        let program = r#"
            func test() {
                let a = 5;
                let b = 10;
                let c = 15;

                // Valid: arithmetic with comparison
                let result1 = (a + b) < c;           // OK: (felt + felt) < felt
                let result2 = a < (b * 2);           // OK: felt < (felt * felt)
                let result3 = (a - 1) <= (b + 1);    // OK: (felt - felt) <= (felt + felt)

                // Valid: comparison results in logical operations
                let result4 = (a < b) && (b < c);    // OK: comparison results are felt
                let result5 = (a > 0) || (b >= 20);  // OK: comparison results are felt

                // Valid: nested comparisons and arithmetic
                let result6 = ((a + b) > c) && ((c - a) < b); // OK

                // Invalid: trying to do arithmetic on comparison results (which return felt but semantically are booleans)
                // This actually passes type checking since comparisons return felt,
                // but it's semantically questionable
                let weird_but_valid = (a < b) + 1;   // OK: felt + felt (though semantically odd)

                // Test equality/inequality mixed with other comparisons
                let result7 = (a == b) || (a < b);   // OK: both return felt
                let result8 = (a != b) && (a > 0);   // OK: both return felt
            }
        "#;
        let file = crate::File::new(&db, program.to_string(), "test.cm".to_string());
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

        // Should have no type errors - all operations are valid
        let type_errors = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::TypeMismatch)
            .count();

        assert_eq!(type_errors, 0, "Should have no type mismatch errors");
    }

    #[test]
    fn test_assignment_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                // Valid assignments
                let a: felt = 1;
                a = 2;                        // OK: same type
                a = 1 + 2;                    // OK: arithmetic result
                a = 1 && 2;                   // OK: logical result

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
        let project = single_file_project(&db, file);
        let semantic_index = get_main_semantic_index(&db, project);

        let validator = TypeValidator;
        let diagnostics = validator.validate(&db, project, file, &semantic_index);

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
