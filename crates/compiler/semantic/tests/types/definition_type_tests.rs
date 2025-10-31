//! Tests for `definition_semantic_type` query
//!
//! These tests verify that the type system correctly determines the semantic type
//! of various definition kinds (variables, parameters, functions, etc.).

use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::{SemanticIndex, module_semantic_index};

use super::*;
use crate::crate_from_program;

fn get_main_semantic_index(db: &dyn SemanticDb, crate_id: Crate) -> SemanticIndex {
    module_semantic_index(db, crate_id, "main".to_string()).unwrap()
}

#[test]
fn test_let_variable_type_inference() {
    let db = test_db();
    let program = r#"
        fn test() {
            let x = 42;        // Should infer felt
            let y: felt = 100; // Explicit felt type
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| {
            semantic_index.scope(*s).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // Test inferred type
    let x_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "x")
        .unwrap();
    let x_def_id = DefinitionId::new(&db, file, x_def_idx);
    let x_type = definition_semantic_type(&db, crate_id, x_def_id);
    assert!(matches!(x_type.data(&db), TypeData::Felt));

    // Test explicit type
    let y_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "y")
        .unwrap();
    let y_def_id = DefinitionId::new(&db, file, y_def_idx);
    let y_type = definition_semantic_type(&db, crate_id, y_def_id);
    assert!(matches!(y_type.data(&db), TypeData::Felt));
}

#[test]
fn test_parameter_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Vector { x: felt, y: felt }
        fn magnitude(v: Vector, scale: felt) -> felt {
            return 0;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| {
            semantic_index.scope(*s).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // Test struct parameter
    let v_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "v")
        .unwrap();
    let v_def_id = DefinitionId::new(&db, file, v_def_idx);
    let v_type = definition_semantic_type(&db, crate_id, v_def_id);
    match v_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Expected struct type, got {other:?}"),
    }

    // Test felt parameter
    let scale_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "scale")
        .unwrap();
    let scale_def_id = DefinitionId::new(&db, file, scale_def_idx);
    let scale_type = definition_semantic_type(&db, crate_id, scale_def_id);
    assert!(matches!(scale_type.data(&db), TypeData::Felt));
}

#[test]
fn test_function_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        fn get_point(x: felt) -> Point {
            return Point { x: x, y: 0 };
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // Get the function definition
    let def_idx = semantic_index
        .latest_definition_index_by_name(root_scope, "get_point")
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // Check the full function type from its definition
    let func_type = definition_semantic_type(&db, crate_id, def_id);
    match func_type.data(&db) {
        TypeData::Function(sig_id) => {
            let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();
            assert_eq!(sig_id, signature);

            let params = signature.params(&db);
            let return_type = signature.return_type(&db);

            // Check parameter types
            let felt_type = TypeId::new(&db, TypeData::Felt);
            let expected_params = vec![("x".to_string(), felt_type)];
            assert_eq!(params, expected_params);

            // Check return type is Point struct
            assert!(matches!(return_type.data(&db), TypeData::Struct(_)));
        }
        other => panic!("Expected function type, got {other:?}"),
    }
}

#[test]
fn test_const_type_resolution() {
    let db = test_db();
    let program = r#"
        const PI = 314;
        const MAX_SIZE = 1000;
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // Test inferred const type
    let pi_def_idx = semantic_index
        .latest_definition_index_by_name(root_scope, "PI")
        .unwrap();
    let pi_def_id = DefinitionId::new(&db, file, pi_def_idx);
    let pi_type = definition_semantic_type(&db, crate_id, pi_def_id);
    assert!(matches!(pi_type.data(&db), TypeData::Felt));

    // Test explicit const type
    let max_def_idx = semantic_index
        .latest_definition_index_by_name(root_scope, "MAX_SIZE")
        .unwrap();
    let max_def_id = DefinitionId::new(&db, file, max_def_idx);
    let max_type = definition_semantic_type(&db, crate_id, max_def_id);
    assert!(matches!(max_type.data(&db), TypeData::Felt));
}

#[test]
fn test_struct_definition_type() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let def_idx = semantic_index
        .latest_definition_index_by_name(root_scope, "Point")
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // The type of a struct definition should be the struct type itself
    let struct_type = definition_semantic_type(&db, crate_id, def_id);
    match struct_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Expected struct type, got {other:?}"),
    }
}
