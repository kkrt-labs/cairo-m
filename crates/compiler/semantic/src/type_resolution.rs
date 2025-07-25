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

use cairo_m_compiler_parser::parser::{
    BinaryOp, Expression, NamedType, Spanned, TypeExpr as AstTypeExpr, UnaryOp,
};

use crate::File;
use crate::db::{Crate, SemanticDb, module_name_for_file, module_semantic_index};
use crate::definition::{DefinitionKind, FunctionDefRef, ParameterDefRef, StructDefRef};
use crate::place::FileScopeId;
use crate::semantic_index::{DefinitionId, ExpressionId};
use crate::types::{FunctionSignatureId, StructTypeId, TypeData, TypeId};

/// Resolves an AST type expression to a `TypeId`
#[salsa::tracked]
pub fn resolve_ast_type<'db>(
    db: &'db dyn SemanticDb,
    crate_id: Crate,
    file: File,
    ast_type_expr: Spanned<AstTypeExpr>,
    context_scope_id: FileScopeId,
) -> TypeId<'db> {
    let module_name = match module_name_for_file(db, crate_id, file) {
        Some(name) => name,
        None => {
            // File not found in project - this can happen during rapid edits
            // Return error type instead of panicking
            tracing::debug!(
                "Could not find module name for file: {:?}",
                file.file_path(db)
            );
            return TypeId::new(db, TypeData::Error);
        }
    };
    let semantic_index = module_semantic_index(db, crate_id, module_name)
        .expect("Failed to resolve index for module");

    match ast_type_expr.value() {
        AstTypeExpr::Named(name) => {
            match name.value() {
                NamedType::Felt => TypeId::new(db, TypeData::Felt),
                NamedType::Bool => TypeId::new(db, TypeData::Bool),
                NamedType::U32 => TypeId::new(db, TypeData::U32),
                NamedType::Custom(name_str) => {
                    // Try to resolve as a struct type
                    semantic_index
                        .resolve_name_to_definition(name_str, context_scope_id)
                        .map(|(def_idx, _)| {
                            let def_id = DefinitionId::new(db, file, def_idx);
                            let def_type = definition_semantic_type(db, crate_id, def_id);

                            // Ensure it's a struct type, not just any definition
                            match def_type.data(db) {
                                TypeData::Struct(_) => def_type,
                                _ => TypeId::new(db, TypeData::Error), // Found a name, but it's not a type
                            }
                        })
                        .unwrap_or_else(|| TypeId::new(db, TypeData::Error)) // Type not found
                }
            }
        }
        AstTypeExpr::Pointer(inner_ast_expr) => {
            let inner_type_id = resolve_ast_type(
                db,
                crate_id,
                file,
                (**inner_ast_expr).clone(),
                context_scope_id,
            );
            TypeId::new(db, TypeData::Pointer(inner_type_id))
        }
        AstTypeExpr::Tuple(inner_ast_exprs) => {
            let collected_type_ids: Vec<TypeId> = inner_ast_exprs
                .iter()
                .map(|expr| resolve_ast_type(db, crate_id, file, expr.clone(), context_scope_id))
                .collect();
            TypeId::new(db, TypeData::Tuple(collected_type_ids))
        }
    }
}

/// Helper function to resolve variable types (for Let definitions)
fn resolve_variable_type<'db>(
    db: &'db dyn SemanticDb,
    crate_id: Crate,
    file: File,
    explicit_type_ast: &Option<Spanned<AstTypeExpr>>,
    value_expr_id: Option<ExpressionId>,
    scope_id: FileScopeId,
) -> TypeId<'db> {
    // Prioritize explicit type annotation if available
    if let Some(type_ast) = explicit_type_ast {
        resolve_ast_type(db, crate_id, file, type_ast.clone(), scope_id)
    } else if let Some(value_expr_id) = value_expr_id {
        // Infer from the stored value expression ID
        expression_semantic_type(db, crate_id, file, value_expr_id, None)
    } else {
        // No type annotation and no value to infer from
        TypeId::new(db, TypeData::Unknown)
    }
}

/// Determines the semantic type of a definition
#[salsa::tracked]
pub fn definition_semantic_type<'db>(
    db: &'db dyn SemanticDb,
    crate_id: Crate,
    definition_id: DefinitionId<'db>,
) -> TypeId<'db> {
    let file = definition_id.file(db);
    let def_index = definition_id.id_in_file(db);

    let module_name = match module_name_for_file(db, crate_id, file) {
        Some(name) => name,
        None => {
            // File not found in project - this can happen during rapid edits
            // Return error type instead of panicking
            tracing::debug!(
                "Could not find module name for file: {:?}",
                file.file_path(db)
            );
            return TypeId::new(db, TypeData::Error);
        }
    };
    let semantic_index = module_semantic_index(db, crate_id, module_name)
        .expect("Failed to resolve index for module");

    let Some(definition) = semantic_index.definition(def_index) else {
        return TypeId::new(db, TypeData::Error);
    };

    match &definition.kind {
        DefinitionKind::Struct(_) => {
            if let Some(struct_type_id) = struct_semantic_data(db, crate_id, definition_id) {
                TypeId::new(db, TypeData::Struct(struct_type_id))
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Function(_) => {
            if let Some(signature_id) = function_semantic_signature(db, crate_id, definition_id) {
                TypeId::new(db, TypeData::Function(signature_id))
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Parameter(ParameterDefRef {
            name: _name,
            type_ast,
        }) => resolve_ast_type(db, crate_id, file, type_ast.clone(), definition.scope_id),
        DefinitionKind::Let(let_ref) => {
            // Check if this is from tuple destructuring
            if let Some((value_expr_id, index)) = let_ref.destructuring_info {
                // Get the type of the RHS tuple expression
                let tuple_type = expression_semantic_type(db, crate_id, file, value_expr_id, None);
                // Extract the type of the element at the given index
                match tuple_type.data(db) {
                    TypeData::Tuple(element_types) => {
                        if index < element_types.len() {
                            element_types[index]
                        } else {
                            TypeId::new(db, TypeData::Error)
                        }
                    }
                    _ => TypeId::new(db, TypeData::Error),
                }
            } else {
                // Regular let variable
                resolve_variable_type(
                    db,
                    crate_id,
                    file,
                    &let_ref.explicit_type_ast,
                    let_ref.value_expr_id,
                    definition.scope_id,
                )
            }
        }
        DefinitionKind::Const(const_ref) => {
            // Constants must be initialized, so we infer from the value expression
            if let Some(value_expr_id) = const_ref.value_expr_id {
                expression_semantic_type(db, crate_id, file, value_expr_id, None)
            } else {
                // Constants without initialization is an error in the language
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Use(use_ref) => {
            // Check if the imported module exists in the project
            if !crate_id
                .modules(db)
                .contains_key(use_ref.imported_module.value())
            {
                return TypeId::new(db, TypeData::Error);
            }

            let imported_module = use_ref.imported_module.clone();
            let imported_index =
                module_semantic_index(db, crate_id, imported_module.value().clone())
                    .expect("Failed to resolve index for imported module");
            let imported_root = imported_index
                .root_scope()
                .expect("Imported module should have root scope");

            if let Some((imported_def_idx, _)) =
                imported_index.resolve_name_to_definition(use_ref.item.value(), imported_root)
            {
                let imported_file = *crate_id
                    .modules(db)
                    .get(use_ref.imported_module.value())
                    .expect("Imported file should exist");
                let imported_def_id = DefinitionId::new(db, imported_file, imported_def_idx);
                definition_semantic_type(db, crate_id, imported_def_id)
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        DefinitionKind::Namespace(_) => {
            // Namespaces don't have a type
            TypeId::new(db, TypeData::Error)
        }
        DefinitionKind::LoopVariable(_) => {
            // TODO: For now, loop variables are untyped (future: infer from iterable)
            // In the future, this should infer the type from the iterable expression
            TypeId::new(db, TypeData::Felt)
        }
    }
}

/// Infers the semantic type of an expression
#[salsa::tracked]
pub fn expression_semantic_type<'db>(
    db: &'db dyn SemanticDb,
    crate_id: Crate,
    file: File,
    expression_id: ExpressionId,
    context_expected: Option<TypeId<'db>>,
) -> TypeId<'db> {
    let module_name = match module_name_for_file(db, crate_id, file) {
        Some(name) => name,
        None => {
            // File not found in project - this can happen during rapid edits
            // Return error type instead of panicking
            tracing::debug!(
                "Could not find module name for file: {:?}",
                file.file_path(db)
            );
            return TypeId::new(db, TypeData::Error);
        }
    };
    let semantic_index = module_semantic_index(db, crate_id, module_name)
        .expect("Failed to resolve index for module");

    let Some(expr_info) = semantic_index.expression(expression_id) else {
        return TypeId::new(db, TypeData::Error);
    };

    // Access the AST node directly from ExpressionInfo - no lookup needed!
    match &expr_info.ast_node {
        Expression::Literal(_value, literal_suffix) => {
            // Priority 1: Check for explicit suffix (e.g., 42u32)
            if let Some(literal_suffix) = literal_suffix {
                let expected_type = TypeData::from(literal_suffix);
                if matches!(expected_type, TypeData::U32) || matches!(expected_type, TypeData::Felt)
                {
                    return TypeId::new(db, expected_type);
                }
            }

            // Priority 2: Check for expected type from AST (e.g., let x: u32 = 42)
            if let Some(type_ast) = expr_info.expected_type_ast.clone() {
                // We found an explicit type annotation. Resolve it.
                let expected_type =
                    resolve_ast_type(db, crate_id, file, type_ast, expr_info.scope_id);

                // If the context expects a numeric type, infer the literal as that type.
                if matches!(expected_type.data(db), TypeData::U32)
                    || matches!(expected_type.data(db), TypeData::Felt)
                {
                    return expected_type;
                }
            }

            // Priority 3: Check for context from propagated type (e.g., in x + 1 where x is u32)
            if let Some(context_type) = context_expected {
                // If the context expects a numeric primitive, use it
                match context_type.data(db) {
                    TypeData::U32 | TypeData::Felt => {
                        return context_type;
                    }
                    _ => {}
                }
            }

            // Default: If no specific context is found, default to `felt`.
            TypeId::new(db, TypeData::Felt)
        }
        Expression::BooleanLiteral(_) => TypeId::new(db, TypeData::Bool),
        Expression::Identifier(name) => {
            if let Some((def_idx, _)) =
                semantic_index.resolve_name_to_definition(name.value(), expr_info.scope_id)
            {
                let def_id = DefinitionId::new(db, file, def_idx);
                definition_semantic_type(db, crate_id, def_id)
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        Expression::UnaryOp { expr, op } => {
            let expr_id = semantic_index.expression_id_by_span(expr.span()).unwrap();
            // Propagate the context_expected to the operand
            let expr_type = expression_semantic_type(db, crate_id, file, expr_id, context_expected);

            for signature in get_unary_op_signatures(db) {
                if signature.op == *op && are_types_compatible(db, expr_type, signature.operand) {
                    return signature.result;
                }
            }

            TypeId::new(db, TypeData::Error)
        }
        Expression::BinaryOp { left, op, right } => {
            let left_id = semantic_index.expression_id_by_span(left.span()).unwrap();
            let right_id = semantic_index.expression_id_by_span(right.span()).unwrap();

            // For commutative operators, we need to be smarter about type inference
            // First, check if either operand is a literal - if so, we want to infer its type from the other operand
            let is_commutative = matches!(
                op,
                BinaryOp::Add | BinaryOp::Mul | BinaryOp::Eq | BinaryOp::Neq
            );

            let (left_type, right_type) = if is_commutative {
                // Check if left is a literal and right has a concrete type
                let left_expr = &semantic_index.expression(left_id).unwrap().ast_node;
                let right_expr = &semantic_index.expression(right_id).unwrap().ast_node;

                let left_is_unsuffixed_literal = matches!(left_expr, Expression::Literal(_, None));
                let right_is_unsuffixed_literal =
                    matches!(right_expr, Expression::Literal(_, None));

                if left_is_unsuffixed_literal && !right_is_unsuffixed_literal {
                    // Infer right first, then use it as context for left
                    let right_type =
                        expression_semantic_type(db, crate_id, file, right_id, context_expected);
                    let left_type =
                        expression_semantic_type(db, crate_id, file, left_id, Some(right_type));
                    (left_type, right_type)
                } else {
                    // Default behavior: left first, then right with left as context
                    let left_type =
                        expression_semantic_type(db, crate_id, file, left_id, context_expected);
                    let right_type =
                        expression_semantic_type(db, crate_id, file, right_id, Some(left_type));
                    (left_type, right_type)
                }
            } else {
                // Non-commutative operators: always left first, then right
                let left_type =
                    expression_semantic_type(db, crate_id, file, left_id, context_expected);
                let right_type =
                    expression_semantic_type(db, crate_id, file, right_id, Some(left_type));
                (left_type, right_type)
            };

            for signature in get_binary_op_signatures(db) {
                if signature.op == *op
                    && are_types_compatible(db, left_type, signature.left)
                    && are_types_compatible(db, right_type, signature.right)
                {
                    return signature.result;
                }
            }

            TypeId::new(db, TypeData::Error)
        }
        Expression::MemberAccess { object, field } => {
            // Handle potential missing expression ID gracefully
            let object_id = match semantic_index.expression_id_by_span(object.span()) {
                Some(id) => id,
                None => return TypeId::new(db, TypeData::Error),
            };

            let object_type = expression_semantic_type(db, crate_id, file, object_id, None);

            match object_type.data(db) {
                TypeData::Struct(struct_id) => {
                    // Direct struct field access
                    struct_id
                        .field_type(db, field.value())
                        .unwrap_or_else(|| TypeId::new(db, TypeData::Error))
                }
                TypeData::Pointer(inner_type) => {
                    // Pointer to struct field access - automatic dereference
                    if let TypeData::Struct(struct_id) = inner_type.data(db) {
                        struct_id
                            .field_type(db, field.value())
                            .unwrap_or_else(|| TypeId::new(db, TypeData::Error))
                    } else {
                        // Pointer to non-struct type
                        TypeId::new(db, TypeData::Error)
                    }
                }
                _ => {
                    // Field access on non-struct, non-pointer type
                    TypeId::new(db, TypeData::Error)
                }
            }
        }
        Expression::FunctionCall { callee, args: _ } => {
            // Get ExpressionId for the callee
            if let Some(callee_expr_id) = semantic_index.expression_id_by_span(callee.span()) {
                // Infer callee's type recursively
                let callee_type =
                    expression_semantic_type(db, crate_id, file, callee_expr_id, None);
                // If it's a function type, return the return type
                match callee_type.data(db) {
                    TypeData::Function(signature_id) => signature_id.return_type(db),
                    _ => TypeId::new(db, TypeData::Error),
                }
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        Expression::StructLiteral { name, fields: _ } => {
            // Resolve the struct name to a definition
            if let Some((def_idx, _)) =
                semantic_index.resolve_name_to_definition(name.value(), expr_info.scope_id)
            {
                let def_id = DefinitionId::new(db, file, def_idx);
                let def_type = definition_semantic_type(db, crate_id, def_id);

                // Ensure it's a struct type
                if let TypeData::Struct(_) = def_type.data(db) {
                    def_type
                } else {
                    TypeId::new(db, TypeData::Error) // Found a name, but it's not a type
                }
            } else {
                TypeId::new(db, TypeData::Error) // Struct type not found
            }
        }
        Expression::IndexAccess { array, index: _ } => {
            // Infer the array/pointer type
            if let Some(array_expr_id) = semantic_index.expression_id_by_span(array.span()) {
                let array_type = expression_semantic_type(db, crate_id, file, array_expr_id, None);

                match array_type.data(db) {
                    // For pointer types, return the dereferenced type
                    TypeData::Pointer(inner_type) => inner_type,
                    // TODO: For tuple types, we could return the element type if all elements are the same
                    // For now, return error for index access on non-pointer types
                    _ => TypeId::new(db, TypeData::Error),
                }
            } else {
                TypeId::new(db, TypeData::Error)
            }
        }
        Expression::Tuple(elements) => {
            // If we have a context expected type that's a tuple, use it to infer element types
            let element_contexts: Vec<Option<TypeId>> = if let Some(context_type) = context_expected
            {
                match context_type.data(db) {
                    TypeData::Tuple(expected_element_types)
                        if expected_element_types.len() == elements.len() =>
                    {
                        // We have a matching tuple type, use element types as context
                        expected_element_types.iter().map(|&t| Some(t)).collect()
                    }
                    _ => vec![None; elements.len()],
                }
            } else {
                vec![None; elements.len()]
            };

            // Infer types of all elements with their respective contexts
            let element_types: Vec<TypeId> = elements
                .iter()
                .enumerate()
                .filter_map(|(i, element)| {
                    semantic_index
                        .expression_id_by_span(element.span())
                        .map(|element_id| {
                            let context = element_contexts.get(i).copied().flatten();
                            expression_semantic_type(db, crate_id, file, element_id, context)
                        })
                })
                .collect();

            // If we successfully resolved all element types, create a tuple type
            if element_types.len() == elements.len() {
                TypeId::new(db, TypeData::Tuple(element_types))
            } else {
                // Some elements couldn't be resolved
                TypeId::new(db, TypeData::Error)
            }
        }
    }
}

/// Retrieves the semantic data for a struct definition
#[salsa::tracked]
pub fn struct_semantic_data<'db>(
    db: &'db dyn SemanticDb,
    crate_id: Crate,
    struct_definition_id: DefinitionId<'db>,
) -> Option<StructTypeId<'db>> {
    let file = struct_definition_id.file(db);
    let def_index = struct_definition_id.id_in_file(db);

    let module_name = module_name_for_file(db, crate_id, file)?;
    let semantic_index = module_semantic_index(db, crate_id, module_name)
        .expect("Failed to resolve index for module");

    let definition = semantic_index.definition(def_index)?;

    if let DefinitionKind::Struct(StructDefRef { fields_ast, name }) = &definition.kind {
        let mut fields = Vec::new();
        for field_def in fields_ast {
            let field_type =
                resolve_ast_type(db, crate_id, file, field_def.1.clone(), definition.scope_id);
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
    crate_id: Crate,
    func_definition_id: DefinitionId<'db>,
) -> Option<FunctionSignatureId<'db>> {
    let file = func_definition_id.file(db);
    let def_index = func_definition_id.id_in_file(db);

    let module_name = module_name_for_file(db, crate_id, file)?;
    let semantic_index = module_semantic_index(db, crate_id, module_name)
        .expect("Failed to resolve index for module");

    let definition = semantic_index.definition(def_index)?;

    if let DefinitionKind::Function(FunctionDefRef {
        params_ast,
        return_type_ast,
        ..
    }) = &definition.kind
    {
        let mut params = Vec::new();
        for (param_name, param_type_ast) in params_ast {
            let param_type = resolve_ast_type(
                db,
                crate_id,
                file,
                param_type_ast.clone(),
                definition.scope_id,
            );
            params.push((param_name.clone(), param_type));
        }

        let return_type = resolve_ast_type(
            db,
            crate_id,
            file,
            return_type_ast.clone(),
            definition.scope_id,
        );

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

        // Bool is only compatible with Bool (not with Felt or U32)
        (TypeData::Bool, TypeData::Bool) => true,
        (TypeData::Bool, _) | (_, TypeData::Bool) => false,

        // U32 is only compatible with U32 (not with Felt)
        (TypeData::U32, TypeData::U32) => true,
        (TypeData::U32, _) | (_, TypeData::U32) => false,

        // All other combinations are incompatible if not caught by direct equality
        _ => false,
    }
}

#[derive(Debug)]
pub struct UnaryOpSignature<'db> {
    pub op: UnaryOp,
    pub operand: TypeId<'db>,
    pub result: TypeId<'db>,
}

pub fn get_unary_op_signatures<'db>(db: &'db dyn SemanticDb) -> Vec<UnaryOpSignature<'db>> {
    let felt = TypeId::new(db, TypeData::Felt);
    let u32 = TypeId::new(db, TypeData::U32);
    let bool = TypeId::new(db, TypeData::Bool);

    vec![
        // Neg
        UnaryOpSignature {
            op: UnaryOp::Neg,
            operand: felt,
            result: felt,
        },
        UnaryOpSignature {
            op: UnaryOp::Neg,
            operand: u32,
            result: u32,
        },
        // Not
        UnaryOpSignature {
            op: UnaryOp::Not,
            operand: bool,
            result: bool,
        },
    ]
}

// A simple representation of a valid operation
#[derive(Debug)]
pub struct OperatorSignature<'db> {
    pub op: BinaryOp,
    pub left: TypeId<'db>,
    pub right: TypeId<'db>,
    pub result: TypeId<'db>,
}

// A function to get all valid signatures
pub fn get_binary_op_signatures<'db>(db: &'db dyn SemanticDb) -> Vec<OperatorSignature<'db>> {
    let felt = TypeId::new(db, TypeData::Felt);
    let u32 = TypeId::new(db, TypeData::U32);
    let bool = TypeId::new(db, TypeData::Bool);

    vec![
        // Arithmetic

        // Add
        OperatorSignature {
            op: BinaryOp::Add,
            left: felt,
            right: felt,
            result: felt,
        },
        OperatorSignature {
            op: BinaryOp::Add,
            left: u32,
            right: u32,
            result: u32,
        },
        // Sub
        OperatorSignature {
            op: BinaryOp::Sub,
            left: felt,
            right: felt,
            result: felt,
        },
        OperatorSignature {
            op: BinaryOp::Sub,
            left: u32,
            right: u32,
            result: u32,
        },
        // Mul
        OperatorSignature {
            op: BinaryOp::Mul,
            left: felt,
            right: felt,
            result: felt,
        },
        OperatorSignature {
            op: BinaryOp::Mul,
            left: u32,
            right: u32,
            result: u32,
        },
        // Div
        OperatorSignature {
            op: BinaryOp::Div,
            left: felt,
            right: felt,
            result: felt,
        },
        OperatorSignature {
            op: BinaryOp::Div,
            left: u32,
            right: u32,
            result: u32,
        },
        // Eq
        OperatorSignature {
            op: BinaryOp::Eq,
            left: felt,
            right: felt,
            result: bool,
        },
        OperatorSignature {
            op: BinaryOp::Eq,
            left: u32,
            right: u32,
            result: bool,
        },
        OperatorSignature {
            op: BinaryOp::Eq,
            left: bool,
            right: bool,
            result: bool,
        },
        // Neq
        OperatorSignature {
            op: BinaryOp::Neq,
            left: felt,
            right: felt,
            result: bool,
        },
        OperatorSignature {
            op: BinaryOp::Neq,
            left: u32,
            right: u32,
            result: bool,
        },
        OperatorSignature {
            op: BinaryOp::Neq,
            left: bool,
            right: bool,
            result: bool,
        },
        // Less
        OperatorSignature {
            op: BinaryOp::Less,
            left: u32,
            right: u32,
            result: bool,
        },
        // Greater
        OperatorSignature {
            op: BinaryOp::Greater,
            left: u32,
            right: u32,
            result: bool,
        },
        // LessEqual
        OperatorSignature {
            op: BinaryOp::LessEqual,
            left: u32,
            right: u32,
            result: bool,
        },
        // GreaterEqual
        OperatorSignature {
            op: BinaryOp::GreaterEqual,
            left: u32,
            right: u32,
            result: bool,
        },
        // And
        OperatorSignature {
            op: BinaryOp::And,
            left: bool,
            right: bool,
            result: bool,
        },
        // Or
        OperatorSignature {
            op: BinaryOp::Or,
            left: bool,
            right: bool,
            result: bool,
        },
    ]
}

#[cfg(test)]
mod tests {
    use chumsky::span::SimpleSpan;

    use super::*;
    use crate::db::tests::{crate_from_program, test_db};
    use crate::module_semantic_index;
    use crate::place::FileScopeId;
    use crate::semantic_index::DefinitionIndex;

    // Helper functions for tests
    fn spanned<T>(value: T) -> Spanned<T> {
        Spanned::new(value, SimpleSpan::from(0..0))
    }

    fn named_type(name: NamedType) -> Spanned<AstTypeExpr> {
        spanned(AstTypeExpr::Named(spanned(name)))
    }

    fn pointer_type(inner: Spanned<AstTypeExpr>) -> Spanned<AstTypeExpr> {
        spanned(AstTypeExpr::Pointer(Box::new(inner)))
    }

    fn tuple_type(elements: Vec<Spanned<AstTypeExpr>>) -> Spanned<AstTypeExpr> {
        spanned(AstTypeExpr::Tuple(elements))
    }

    #[test]
    fn test_resolve_felt_type() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "");
        let file = *crate_id.modules(&db).values().next().unwrap();
        let scope_id = FileScopeId::new(0);

        let felt_type =
            resolve_ast_type(&db, crate_id, file, named_type(NamedType::Felt), scope_id);
        let felt_data = felt_type.data(&db);

        assert!(matches!(felt_data, TypeData::Felt));
    }

    #[test]
    fn test_resolve_u32_type() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "");
        let file = *crate_id.modules(&db).values().next().unwrap();
        let scope_id = FileScopeId::new(0);

        let u32_type = resolve_ast_type(&db, crate_id, file, named_type(NamedType::U32), scope_id);
        let u32_data = u32_type.data(&db);

        assert!(matches!(u32_data, TypeData::U32));
    }

    #[test]
    fn test_resolve_pointer_type() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "");
        let file = *crate_id.modules(&db).values().next().unwrap();
        let scope_id = FileScopeId::new(0);

        let pointer_type = resolve_ast_type(
            &db,
            crate_id,
            file,
            pointer_type(named_type(NamedType::Felt)),
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
        let crate_id = crate_from_program(&db, "");
        let file = *crate_id.modules(&db).values().next().unwrap();
        let scope_id = FileScopeId::new(0);

        let tuple_type = resolve_ast_type(
            &db,
            crate_id,
            file,
            tuple_type(vec![
                named_type(NamedType::Felt),
                named_type(NamedType::Felt),
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
        let crate_id = crate_from_program(&db, "");
        let file = *crate_id.modules(&db).values().next().unwrap();

        let felt1 = TypeId::new(&db, TypeData::Felt);
        let felt2 = TypeId::new(&db, TypeData::Felt);
        let u32_1 = TypeId::new(&db, TypeData::U32);
        let u32_2 = TypeId::new(&db, TypeData::U32);
        let bool_type = TypeId::new(&db, TypeData::Bool);
        let error_type = TypeId::new(&db, TypeData::Error);
        let unknown_type = TypeId::new(&db, TypeData::Unknown);

        // Same types should be compatible
        assert!(are_types_compatible(&db, felt1, felt2));
        assert!(are_types_compatible(&db, u32_1, u32_2));
        assert!(are_types_compatible(&db, bool_type, bool_type));

        // Different primitive types should NOT be compatible
        assert!(!are_types_compatible(&db, felt1, u32_1));
        assert!(!are_types_compatible(&db, u32_1, felt1));
        assert!(!are_types_compatible(&db, felt1, bool_type));
        assert!(!are_types_compatible(&db, u32_1, bool_type));

        // Error and Unknown types should be compatible with anything
        assert!(are_types_compatible(&db, felt1, error_type));
        assert!(are_types_compatible(&db, error_type, felt1));
        assert!(are_types_compatible(&db, felt1, unknown_type));
        assert!(are_types_compatible(&db, unknown_type, felt1));
        assert!(are_types_compatible(&db, u32_1, error_type));
        assert!(are_types_compatible(&db, error_type, u32_1));
        assert!(are_types_compatible(&db, u32_1, unknown_type));
        assert!(are_types_compatible(&db, unknown_type, u32_1));

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

    #[test]
    fn test_direct_ast_node_access() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "fn test() { let x = 42; }");
        let file = *crate_id.modules(&db).values().next().unwrap();
        let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        // Find any expression in the index
        let all_expressions: Vec<_> = semantic_index.all_expressions().collect();
        assert!(
            !all_expressions.is_empty(),
            "Should have at least one expression"
        );

        for (expr_id, expr_info) in all_expressions {
            // Verify that we can access the AST node directly without lookup
            match &expr_info.ast_node {
                Expression::Literal(value, _) => {
                    // Test that we can access literal values directly
                    assert_eq!(*value, 42);

                    // Verify the expression type can be resolved efficiently
                    let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);
                    assert!(matches!(expr_type.data(&db), TypeData::Felt));
                }
                Expression::Identifier(name) => {
                    // Test that we can access identifier names directly
                    assert_eq!(name.value(), "x");
                }
                _ => {
                    panic!("Test data does not contain this expr")
                }
            }

            // Verify that span information is still available for diagnostics
            assert!(expr_info.ast_span.start < expr_info.ast_span.end);
        }
    }

    #[test]
    fn test_expression_type_coverage() {
        let db = test_db();

        // Simple test program that exercises all expression types
        let program = r#"
            struct Point { x: felt, y: felt }
            fn test() {
                let p = Point { x: 1, y: 2 };
                let sum = 1 + 2;
                let coord = p.x;
                return;
            }
        "#;
        let crate_id = crate_from_program(&db, program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        // Count how many different expression types we find
        let mut expression_types_found = std::collections::HashSet::new();

        // We are expecting to find that many expressions;
        let expected_expression_count = 8;
        assert_eq!(
            semantic_index.all_expressions().count(),
            expected_expression_count
        );

        for (expr_id, expr_info) in semantic_index.all_expressions() {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);

            // Record the expression variant we found
            let variant_name = match &expr_info.ast_node {
                Expression::Literal(_, _) => "Literal",
                Expression::BooleanLiteral(_) => "BooleanLiteral",
                Expression::Identifier(_) => "Identifier",
                Expression::UnaryOp { .. } => "UnaryOp",
                Expression::BinaryOp { .. } => "BinaryOp",
                Expression::FunctionCall { .. } => "FunctionCall",
                Expression::MemberAccess { .. } => "MemberAccess",
                Expression::IndexAccess { .. } => "IndexAccess",
                Expression::StructLiteral { .. } => "StructLiteral",
                Expression::Tuple(_) => "Tuple",
            };
            expression_types_found.insert(variant_name);

            // Verify we never return Unknown type
            assert!(!matches!(expr_type.data(&db), TypeData::Unknown));

            // Basic sanity checks
            match &expr_info.ast_node {
                Expression::Literal(_, _) => {
                    assert!(matches!(expr_type.data(&db), TypeData::Felt));
                }
                Expression::BinaryOp {
                    op: _,
                    left: _,
                    right: _,
                } => {
                    assert!(matches!(expr_type.data(&db), TypeData::Felt));
                }
                Expression::MemberAccess {
                    object: _,
                    field: _,
                } => {
                    assert_eq!(expr_type.data(&db), TypeData::Felt);
                }
                Expression::StructLiteral { name, .. } if name.value() == "Point" => {
                    if let TypeData::Struct(struct_id) = expr_type.data(&db) {
                        assert_eq!(struct_id.name(&db), "Point");
                    } else {
                        panic!("Expected struct type, got {:?}", expr_type.data(&db));
                    }
                }
                _ => {
                    // For other expression types, just ensure we get some valid type
                    assert!(!matches!(expr_type.data(&db), TypeData::Unknown));
                }
            }
        }

        // Verify we found the main expression types in our test program
        assert!(expression_types_found.contains("Literal"));
        assert!(expression_types_found.contains("StructLiteral"));
        assert!(expression_types_found.contains("BinaryOp"));
        assert!(expression_types_found.contains("MemberAccess"));
    }

    #[test]
    fn test_member_access_edge_cases() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            struct Nested { point: Point, value: felt }

            fn test(p: Point, ptr: felt*, nested: Nested) -> felt {
                let x1 = p.x;           // Direct struct field access
                let n1 = nested.value;  // Nested struct field
                let n2 = nested.point;  // Nested struct returns Point type
                return x1;
            }
        "#;
        let crate_id = crate_from_program(&db, program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        // Find member access expressions and verify their types
        for expr_id in semantic_index.span_to_expression_id.values() {
            let expr_info = semantic_index.expression(*expr_id).unwrap();

            if let Expression::MemberAccess { object: _, field } = &expr_info.ast_node {
                let expr_type = expression_semantic_type(&db, crate_id, file, *expr_id, None);

                match field.value().as_str() {
                    "x" | "value" => {
                        // These should be felt type
                        assert!(
                            matches!(expr_type.data(&db), TypeData::Felt),
                            "Field {} should have felt type",
                            field.value()
                        );
                    }
                    "point" => {
                        // This should be Point type
                        assert!(
                            matches!(expr_type.data(&db), TypeData::Struct(_)),
                            "Field {} should have struct type",
                            field.value()
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_pointer_to_struct_field_access() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }

            fn test(ptr: Point*) -> felt {
                let x = ptr.x;  // Should automatically dereference
                return x;
            }
        "#;
        let crate_id = crate_from_program(&db, program);
        let file = *crate_id.modules(&db).values().next().unwrap();
        let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

        // Find the ptr.x expression
        let mut found_ptr_access = false;
        for expr_id in semantic_index.span_to_expression_id.values() {
            let expr_info = semantic_index.expression(*expr_id).unwrap();

            if let Expression::MemberAccess { object, field } = &expr_info.ast_node
                && let Expression::Identifier(ident) = object.value()
                && ident.value() == "ptr"
                && field.value() == "x"
            {
                let expr_type = expression_semantic_type(&db, crate_id, file, *expr_id, None);
                assert!(
                    matches!(expr_type.data(&db), TypeData::Felt),
                    "ptr.x should have felt type through automatic dereference"
                );
                found_ptr_access = true;
            }
        }
        assert!(found_ptr_access, "Should have found ptr.x expression");
    }
}
