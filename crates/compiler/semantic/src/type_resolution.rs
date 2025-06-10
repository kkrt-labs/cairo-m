//! # Type Resolution and Inference
//!
//! This module implements the core type resolution and inference logic using Salsa queries.
//! It provides the implementations for all type-related database queries defined in `SemanticDb`.
//!
//! ## Key Functions
//!
//! - `resolve_ast_type`: Converts AST type expressions to semantic types
//! - `definition_semantic_type`: Determines the type of a definition
//! - `expression_semantic_type`: Infers the type of an expression
//! - `struct_semantic_data`: Resolves struct type information
//! - `function_semantic_signature`: Resolves function signature information
//! - `are_types_compatible`: Checks type compatibility

use crate::db::SemanticDb;
use crate::definition::{
    DefinitionKind, FunctionDefRef, LocalDefRef, ParameterDefRef, StructDefRef,
};
use crate::place::FileScopeId;
use crate::semantic_index::{semantic_index, DefinitionId, ExpressionId};
use crate::types::{FunctionSignatureId, StructTypeId, TypeData, TypeId};
use crate::File;
use cairo_m_compiler_parser::parser::{
    Expression, Spanned, Statement, TopLevelItem, TypeExpr as AstTypeExpr,
};
use cairo_m_compiler_parser::{parse_program, ParsedModule};
use chumsky::span::SimpleSpan;

/// Resolves an AST type expression to a `TypeId`
#[salsa::tracked]
pub fn resolve_ast_type<'db>(
    db: &'db dyn SemanticDb,
    file: File,
    ast_type_expr: AstTypeExpr,
    context_scope_id: FileScopeId,
) -> TypeId<'db> {
    let semantic_index = semantic_index(db, file);

    match ast_type_expr {
        AstTypeExpr::Named(name_str) => {
            if name_str == "felt" {
                return TypeId::new(db, TypeData::Felt);
            }

            // Try to resolve as a struct type
            if let Some((def_idx, _)) =
                semantic_index.resolve_name_to_definition(&name_str, context_scope_id)
            {
                let def_id = DefinitionId::new(db, file, def_idx);
                let def_type = definition_semantic_type(db, def_id);

                // Ensure it's a struct type, not just any definition
                if let TypeData::Struct(_) = def_type.data(db) {
                    def_type
                } else {
                    TypeId::new(db, TypeData::Error) // Found a name, but it's not a type
                }
            } else {
                TypeId::new(db, TypeData::Error) // Type not found
            }
        }
        AstTypeExpr::Pointer(inner_ast_expr) => {
            let inner_type_id = resolve_ast_type(db, file, *inner_ast_expr, context_scope_id);
            TypeId::new(db, TypeData::Pointer(inner_type_id))
        }
        AstTypeExpr::Tuple(inner_ast_exprs) => {
            let collected_type_ids: Vec<TypeId> = inner_ast_exprs
                .into_iter()
                .map(|expr| resolve_ast_type(db, file, expr, context_scope_id))
                .collect();
            TypeId::new(db, TypeData::Tuple(collected_type_ids))
        }
    }
}

/// Determines the semantic type of a definition
#[salsa::tracked]
pub fn definition_semantic_type<'db>(
    db: &'db dyn SemanticDb,
    definition_id: DefinitionId<'db>,
) -> TypeId<'db> {
    let file = definition_id.file(db);
    let def_index = definition_id.id_in_file(db);
    let semantic_index = semantic_index(db, file);

    let Some(definition) = semantic_index.definition(def_index) else {
        return TypeId::new(db, TypeData::Error);
    };

    match &definition.kind {
        DefinitionKind::Struct(_) => {
            if let Some(struct_type_id) = struct_semantic_data(db, definition_id) {
                TypeId::new(db, TypeData::Struct(struct_type_id))
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Function(_) => {
            if let Some(signature_id) = function_semantic_signature(db, definition_id) {
                TypeId::new(db, TypeData::Function(signature_id))
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Parameter(ParameterDefRef {
            name: _name,
            type_ast,
        }) => resolve_ast_type(db, file, type_ast.clone(), definition.scope_id),
        DefinitionKind::Local(LocalDefRef {
            explicit_type_ast, ..
        }) => resolve_ast_type(
            db,
            file,
            explicit_type_ast.clone().unwrap(),
            definition.scope_id,
        ),
        // For `let` and `local` without type hints, we need to infer from the value.
        // This requires finding the expression associated with the definition.
        // This part of the semantic index is not yet fully implemented.
        DefinitionKind::Let(_) | DefinitionKind::Const(_) => {
            // For variables without explicit type annotations, infer from their value expression
            // First, try to find the expression associated with this definition
            if let Some((value_expr_id, _)) =
                semantic_index.all_expressions().find(|(_, expr_info)| {
                    // Look for expressions in the same scope that might be the initializer
                    expr_info.scope_id == definition.scope_id
                })
            {
                // If we found a potential value expression, infer its type
                expression_semantic_type(db, file, value_expr_id)
            } else {
                // Fallback to Unknown if we can't find the associated expression
                TypeId::new(db, TypeData::Unknown)
            }
        }
        DefinitionKind::Import(_) | DefinitionKind::Namespace(_) => {
            // These don't have a "type" in the traditional sense.
            TypeId::new(db, TypeData::Error)
        }
    }
}

/// Infers the semantic type of an expression
#[salsa::tracked]
pub fn expression_semantic_type<'db>(
    db: &'db dyn SemanticDb,
    file: File,
    expression_id: ExpressionId,
) -> TypeId<'db> {
    let semantic_index = semantic_index(db, file);

    let Some(expr_info) = semantic_index.expression(expression_id) else {
        return TypeId::new(db, TypeData::Error);
    };

    // To get the AST node, we need to re-access the parsed module and search by span.
    // Even if cached, this is inefficient but necessary without an AST ID map.
    // TODO: make this more efficient!
    let parsed_module = parse_program(db, file);
    let Some(ast_expr) = find_expression_in_module(parsed_module, expr_info.ast_node_text_range)
    else {
        return TypeId::new(db, TypeData::Error);
    };

    match ast_expr.value() {
        Expression::Literal(_) => TypeId::new(db, TypeData::Felt),
        Expression::Identifier(name) => {
            if let Some((def_idx, _)) =
                semantic_index.resolve_name_to_definition(name.value(), expr_info.scope_id)
            {
                let def_id = DefinitionId::new(db, file, def_idx);
                definition_semantic_type(db, def_id)
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        Expression::BinaryOp { left, op: _, right } => {
            // TODO For now, assume all binary ops are on felts and return felt.
            // A real implementation would check left/right types.
            let left_id = semantic_index.expression_id_by_span(left.span()).unwrap();
            let right_id = semantic_index.expression_id_by_span(right.span()).unwrap();

            let left_type = expression_semantic_type(db, file, left_id);
            let right_type = expression_semantic_type(db, file, right_id);

            // Basic type check - both should be felt, or fail.
            // TODO: add test for this.
            if are_types_compatible(db, left_type, TypeId::new(db, TypeData::Felt))
                && are_types_compatible(db, right_type, TypeId::new(db, TypeData::Felt))
            {
                TypeId::new(db, TypeData::Felt)
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        Expression::MemberAccess { object, field } => {
            let object_id = semantic_index.expression_id_by_span(object.span()).unwrap();
            let object_type = expression_semantic_type(db, file, object_id);

            // TODO: add test for this.
            if let TypeData::Struct(struct_id) = object_type.data(db) {
                struct_id
                    .field_type(db, field.value())
                    .unwrap_or_else(|| TypeId::new(db, TypeData::Error))
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        _ => TypeId::new(db, TypeData::Unknown),
    }
}

/// Retrieves the semantic data for a struct definition
#[salsa::tracked]
pub fn struct_semantic_data<'db>(
    db: &'db dyn SemanticDb,
    struct_definition_id: DefinitionId<'db>,
) -> Option<StructTypeId<'db>> {
    let file = struct_definition_id.file(db);
    let def_index = struct_definition_id.id_in_file(db);
    let semantic_index = semantic_index(db, file);

    let definition = semantic_index.definition(def_index)?;

    if let DefinitionKind::Struct(StructDefRef { fields_ast, name }) = &definition.kind {
        let mut fields = Vec::new();
        for field_def in fields_ast {
            let field_type = resolve_ast_type(db, file, field_def.1.clone(), definition.scope_id);
            fields.push((field_def.0.clone(), field_type));
        }

        Some(StructTypeId::new(
            db,
            struct_definition_id,
            name.clone(),
            fields,
            definition.scope_id,
        ))
    } else {
        None
    }
}

/// Retrieves the semantic signature for a function definition
#[salsa::tracked]
pub fn function_semantic_signature<'db>(
    db: &'db dyn SemanticDb,
    func_definition_id: DefinitionId<'db>,
) -> Option<FunctionSignatureId<'db>> {
    let file = func_definition_id.file(db);
    let def_index = func_definition_id.id_in_file(db);
    let semantic_index = semantic_index(db, file);

    let definition = semantic_index.definition(def_index)?;

    if let DefinitionKind::Function(FunctionDefRef {
        params_ast,
        return_type_ast,
        ..
    }) = &definition.kind
    {
        let mut params = Vec::new();
        for (param_name, param_type_ast) in params_ast {
            let param_type =
                resolve_ast_type(db, file, param_type_ast.clone(), definition.scope_id);
            params.push((param_name.clone(), param_type));
        }

        let return_type = match return_type_ast.clone() {
            Some(ty) => resolve_ast_type(db, file, ty, definition.scope_id),
            // Assuming no return type means returns a tuple `()`
            None => TypeId::new(db, TypeData::Tuple(vec![])),
        };

        Some(FunctionSignatureId::new(
            db,
            func_definition_id,
            params,
            return_type,
        ))
    } else {
        None
    }
}

/// Checks if two types are compatible
#[salsa::tracked]
pub fn are_types_compatible<'db>(
    db: &'db dyn SemanticDb,
    actual_type: TypeId<'db>,
    expected_type: TypeId<'db>,
) -> bool {
    // For now, implement simple equality-based compatibility
    if actual_type == expected_type {
        return true;
    }

    let actual_data = actual_type.data(db);
    let expected_data = expected_type.data(db);

    match (actual_data, expected_data) {
        // Error and Unknown types are compatible with anything to prevent cascading errors
        (TypeData::Error, _) | (_, TypeData::Error) => true,
        (TypeData::Unknown, _) | (_, TypeData::Unknown) => true,

        // Tuple compatibility (recursive check)
        (TypeData::Tuple(actual_types), TypeData::Tuple(expected_types)) => {
            actual_types.len() == expected_types.len()
                && actual_types
                    .iter()
                    .zip(expected_types.iter())
                    .all(|(a, e)| are_types_compatible(db, *a, *e))
        }

        // Pointer compatibility
        (TypeData::Pointer(actual_inner), TypeData::Pointer(expected_inner)) => {
            are_types_compatible(db, actual_inner, expected_inner)
        }

        // All other combinations are incompatible if not caught by direct equality
        _ => false,
    }
}

// Helper to find an expression in the AST by span. This is inefficient and should be
// replaced with a more direct mapping if possible in the future.
fn find_expression_in_module(
    module: &ParsedModule,
    span_to_find: SimpleSpan<usize>,
) -> Option<&Spanned<Expression>> {
    for item in module.items() {
        if let Some(expr) = find_expression_in_toplevel(item, span_to_find) {
            return Some(expr);
        }
    }
    None
}

fn find_expression_in_toplevel(
    item: &TopLevelItem,
    span_to_find: SimpleSpan<usize>,
) -> Option<&Spanned<Expression>> {
    match item {
        TopLevelItem::Function(func) => {
            for stmt in &func.value().body {
                if let Some(expr) = find_expression_in_statement(stmt, span_to_find) {
                    return Some(expr);
                }
            }
        }
        TopLevelItem::Const(c) => {
            if let Some(expr) = find_expression_in_expr(&c.value().value, span_to_find) {
                return Some(expr);
            }
        }
        TopLevelItem::Namespace(ns) => {
            for inner_item in &ns.value().body {
                if let Some(expr) = find_expression_in_toplevel(inner_item, span_to_find) {
                    return Some(expr);
                }
            }
        }
        _ => {}
    }
    None
}

fn find_expression_in_statement(
    stmt: &Spanned<Statement>,
    span_to_find: SimpleSpan<usize>,
) -> Option<&Spanned<Expression>> {
    if stmt.span() == span_to_find
        && let Statement::Expression(expr) = stmt.value()
    {
        return Some(expr);
    }
    match stmt.value() {
        Statement::Let { value, .. } => find_expression_in_expr(value, span_to_find),
        Statement::Local { value, .. } => find_expression_in_expr(value, span_to_find),
        Statement::Const(c) => find_expression_in_expr(&c.value, span_to_find),
        Statement::Assignment { lhs, rhs } => find_expression_in_expr(lhs, span_to_find)
            .or_else(|| find_expression_in_expr(rhs, span_to_find)),
        Statement::Return { value: Some(v), .. } => find_expression_in_expr(v, span_to_find),
        Statement::If {
            condition,
            then_block,
            else_block,
        } => find_expression_in_expr(condition, span_to_find)
            .or_else(|| find_expression_in_statement(then_block, span_to_find))
            .or_else(|| {
                else_block
                    .as_ref()
                    .and_then(|eb| find_expression_in_statement(eb, span_to_find))
            }),
        Statement::Expression(expr) => find_expression_in_expr(expr, span_to_find),
        Statement::Block(stmts) => stmts
            .iter()
            .find_map(|s| find_expression_in_statement(s, span_to_find)),
        _ => None,
    }
}

fn find_expression_in_expr(
    expr: &Spanned<Expression>,
    span_to_find: SimpleSpan<usize>,
) -> Option<&Spanned<Expression>> {
    if expr.span() == span_to_find {
        return Some(expr);
    }
    match expr.value() {
        Expression::BinaryOp { left, right, .. } => find_expression_in_expr(left, span_to_find)
            .or_else(|| find_expression_in_expr(right, span_to_find)),
        Expression::FunctionCall { callee, args } => find_expression_in_expr(callee, span_to_find)
            .or_else(|| {
                args.iter()
                    .find_map(|arg| find_expression_in_expr(arg, span_to_find))
            }),
        Expression::MemberAccess { object, .. } => find_expression_in_expr(object, span_to_find),
        Expression::IndexAccess { array, index } => find_expression_in_expr(array, span_to_find)
            .or_else(|| find_expression_in_expr(index, span_to_find)),
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .find_map(|(_, val)| find_expression_in_expr(val, span_to_find)),
        Expression::Tuple(exprs) => exprs
            .iter()
            .find_map(|e| find_expression_in_expr(e, span_to_find)),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::place::FileScopeId;
    use crate::semantic_index::DefinitionIndex;

    #[test]
    fn test_resolve_felt_type() {
        let db = test_db();
        let file = crate::File::new(&db, "".to_string());
        let scope_id = FileScopeId::new(0);

        let felt_type =
            resolve_ast_type(&db, file, AstTypeExpr::Named("felt".to_string()), scope_id);
        let felt_data = felt_type.data(&db);

        assert!(matches!(felt_data, TypeData::Felt));
    }

    #[test]
    fn test_resolve_pointer_type() {
        let db = test_db();
        let file = crate::File::new(&db, "".to_string());
        let scope_id = FileScopeId::new(0);

        let pointer_type = resolve_ast_type(
            &db,
            file,
            AstTypeExpr::Pointer(Box::new(AstTypeExpr::Named("felt".to_string()))),
            scope_id,
        );
        let pointer_data = pointer_type.data(&db);

        match pointer_data {
            TypeData::Pointer(inner) => {
                let inner_data = inner.data(&db);
                assert!(matches!(inner_data, TypeData::Felt));
            }
            _ => panic!("Expected pointer type"),
        }
    }

    #[test]
    fn test_resolve_tuple_type() {
        let db = test_db();
        let file = crate::File::new(&db, "".to_string());
        let scope_id = FileScopeId::new(0);

        let tuple_type = resolve_ast_type(
            &db,
            file,
            AstTypeExpr::Tuple(vec![
                AstTypeExpr::Named("felt".to_string()),
                AstTypeExpr::Named("felt".to_string()),
            ]),
            scope_id,
        );
        let tuple_data = tuple_type.data(&db);

        match tuple_data {
            TypeData::Tuple(types) => {
                assert_eq!(types.len(), 2);
                for type_id in types {
                    let inner_data = type_id.data(&db);
                    assert!(matches!(inner_data, TypeData::Felt));
                }
            }
            _ => panic!("Expected tuple type"),
        }
    }

    #[test]
    fn test_type_compatibility() {
        let db = test_db();
        let file = crate::File::new(&db, "".to_string());

        let felt1 = TypeId::new(&db, TypeData::Felt);
        let felt2 = TypeId::new(&db, TypeData::Felt);
        let error_type = TypeId::new(&db, TypeData::Error);
        let unknown_type = TypeId::new(&db, TypeData::Unknown);

        // Same types should be compatible
        assert!(are_types_compatible(&db, felt1, felt2));

        // Error and Unknown types should be compatible with anything
        assert!(are_types_compatible(&db, felt1, error_type));
        assert!(are_types_compatible(&db, error_type, felt1));
        assert!(are_types_compatible(&db, felt1, unknown_type));
        assert!(are_types_compatible(&db, unknown_type, felt1));

        // Structs should be compatible if they have the same definitions.
        let def_id_1 = DefinitionId::new(&db, file, DefinitionIndex::from(0));
        let def_id_2 = DefinitionId::new(&db, file, DefinitionIndex::from(1));
        let scope_id = FileScopeId::new(0);

        let struct_type_id1 =
            StructTypeId::new(&db, def_id_1, "struct1".to_string(), vec![], scope_id);
        let struct_type_id1_dup =
            StructTypeId::new(&db, def_id_1, "struct1".to_string(), vec![], scope_id);
        let struct_type_id2 =
            StructTypeId::new(&db, def_id_2, "struct2".to_string(), vec![], scope_id);

        let instance_def_1_1 = TypeId::new(&db, TypeData::Struct(struct_type_id1));
        let instance_def_1_2 = TypeId::new(&db, TypeData::Struct(struct_type_id1_dup));
        let instance_def_2_1 = TypeId::new(&db, TypeData::Struct(struct_type_id2));

        assert!(are_types_compatible(
            &db,
            instance_def_1_1,
            instance_def_1_2
        ));
        assert!(!are_types_compatible(
            &db,
            instance_def_1_1,
            instance_def_2_1
        ));

        // Tuples should be compatible if they have the same length and compatible elements
        let tuple1 = TypeId::new(&db, TypeData::Tuple(vec![felt1, felt2]));
        let tuple2 = TypeId::new(&db, TypeData::Tuple(vec![felt1, felt2]));
        assert!(are_types_compatible(&db, tuple1, tuple2));
    }
}
