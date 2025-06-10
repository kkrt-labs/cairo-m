//! Real integration tests that actually test type resolution
//!
//! These tests verify that the type system actually works end-to-end

use crate::db::tests::test_db;
use crate::semantic_index::{semantic_index, DefinitionId};
use crate::type_resolution::{
    definition_semantic_type, expression_semantic_type, function_semantic_signature,
    resolve_ast_type, struct_semantic_data,
};
use crate::types::{TypeData, TypeId};
use crate::File;
use cairo_m_compiler_parser::parser::TypeExpr as AstTypeExpr;

#[test]
fn test_resolve_primitive_types() {
    let db = test_db();
    let file = File::new(&db, "".to_string());
    let semantic_index = semantic_index(&db, file);
    let root_scope = semantic_index.root_scope().unwrap();

    let felt_type = resolve_ast_type(
        &db,
        file,
        AstTypeExpr::Named("felt".to_string()),
        root_scope,
    );
    assert!(matches!(felt_type.data(&db), TypeData::Felt));

    let pointer_felt_type = resolve_ast_type(
        &db,
        file,
        AstTypeExpr::Pointer(Box::new(AstTypeExpr::Named("felt".to_string()))),
        root_scope,
    );
    assert!(
        matches!(pointer_felt_type.data(&db), TypeData::Pointer(t) if matches!(t.data(&db), TypeData::Felt))
    );
}

#[test]
fn test_struct_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }
    "#;
    let file = File::new(&db, program.to_string());
    let semantic_index = semantic_index(&db, file);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Resolve `Point` as a type name.
    let point_type_id = resolve_ast_type(
        &db,
        file,
        AstTypeExpr::Named("Point".to_string()),
        root_scope,
    );
    let point_type_data = point_type_id.data(&db);

    // 2. Assert it resolved to a Struct type.
    let struct_id = match point_type_data {
        TypeData::Struct(id) => {
            assert_eq!(id.name(&db), "Point");
            id
        }
        other => panic!("Expected Point to resolve to a struct, got {other:?}"),
    };

    // 3. Get the struct's semantic data directly and compare.
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let semantic_data = struct_semantic_data(&db, def_id).unwrap();

    assert_eq!(struct_id, semantic_data);
    assert_eq!(semantic_data.name(&db), "Point");

    // 4. Check the fields.
    let fields = semantic_data.fields(&db);
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_fields = vec![("x".to_string(), felt_type), ("y".to_string(), felt_type)];
    assert_eq!(fields, expected_fields);
}

#[test]
fn test_function_signature_resolution() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func get_point(x: felt) -> Point {
            return Point { x: x, y: 0 };
        }
    "#;
    let file = File::new(&db, program.to_string());
    let semantic_index = semantic_index(&db, file);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Get the function definition.
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("get_point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // 2. Resolve the function's signature.
    let signature = function_semantic_signature(&db, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // 3. Assert parameter types.
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_params = vec![("x".to_string(), felt_type)];
    assert_eq!(params, expected_params);

    // 4. Assert return type.
    let point_type_id = resolve_ast_type(
        &db,
        file,
        AstTypeExpr::Named("Point".to_string()),
        root_scope,
    );
    assert_eq!(return_type, point_type_id);
    assert!(matches!(return_type.data(&db), TypeData::Struct(_)));

    // 5. Check the full function type from its definition.
    let func_type = definition_semantic_type(&db, def_id);
    match func_type.data(&db) {
        TypeData::Function(sig_id) => assert_eq!(sig_id, signature),
        other => panic!("Expected function type, got {other:?}"),
    }
}

#[test]
fn test_parameter_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Vector { x: felt, y: felt }
        func magnitude(v: Vector) -> felt {
            return 0;
        }
    "#;
    let file = File::new(&db, program.to_string());
    let semantic_index = semantic_index(&db, file);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| semantic_index.scope(*s).unwrap().kind == crate::place::ScopeKind::Function)
        .unwrap();

    // 1. Find the parameter definition `v`.
    let (param_def_idx, _) = semantic_index
        .resolve_name_to_definition("v", func_scope)
        .unwrap();
    let param_def_id = DefinitionId::new(&db, file, param_def_idx);

    // 2. Get its semantic type.
    let param_type = definition_semantic_type(&db, param_def_id);

    // 3. Assert it's a struct type `Vector`.
    match param_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Expected parameter to be a struct type, got {other:?}"),
    }
}

#[test]
fn test_expression_type_inference() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test(p: Point) -> felt {
            let a = 42;
            let b = a + 1;
            let c = p.x;
            return c;
        }
    "#;
    let file = File::new(&db, program.to_string());
    let semantic_index = semantic_index(&db, file);

    // Helper to find an expression by matching against tracked expressions
    let find_expr_id = |target_text: &str| {
        for (span, expr_id) in &semantic_index.span_to_expression_id {
            let source_text = &program[span.start..span.end];
            if source_text == target_text {
                return *expr_id;
            }
        }
        panic!(
            "Expression '{}' not found in tracked expressions. Available: {:?}",
            target_text,
            semantic_index
                .span_to_expression_id
                .keys()
                .map(|span| &program[span.into_range()])
                .collect::<Vec<_>>()
        );
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test literal
    let expr_id = find_expr_id("42");
    let expr_type = expression_semantic_type(&db, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test identifier `a` (inferred from literal)
    let a_expr_id = find_expr_id("a");
    let a_expr_type = expression_semantic_type(&db, file, a_expr_id);
    assert_eq!(a_expr_type, felt_type);

    // Test binary operation
    let expr_id = find_expr_id("a + 1");
    let expr_type = expression_semantic_type(&db, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test member access
    let expr_id = find_expr_id("p.x");
    let expr_type = expression_semantic_type(&db, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test identifier `c` (inferred from member access)
    // Find the identifier 'c' in the return statement
    let c_expr_id = find_expr_id("c");
    let c_expr_type = expression_semantic_type(&db, file, c_expr_id);
    assert_eq!(c_expr_type, felt_type);
}
