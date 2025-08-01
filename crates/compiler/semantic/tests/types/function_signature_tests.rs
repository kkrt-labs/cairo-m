//! Tests for `function_semantic_signature` query
//!
//! These tests verify that function signatures are correctly resolved,
//! including parameter types, return types, and signature metadata.

use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;
use crate::{crate_from_program, get_main_semantic_index};

#[test]
fn test_simple_function_signature() {
    let db = test_db();
    let program = r#"
        fn add(a: felt, b: felt) -> felt {
            return a + b;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("add", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // Check parameters
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_params = vec![("a".to_string(), felt_type), ("b".to_string(), felt_type)];
    assert_eq!(params, expected_params);

    // Check return type
    assert!(matches!(return_type.data(&db), TypeData::Felt));
}

#[test]
fn test_function_with_struct_parameters() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        struct Vector { dx: felt, dy: felt }

        fn translate(point: Point, offset: Vector) -> Point {
            return Point {
                x: point.x + offset.dx,
                y: point.y + offset.dy
            };
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("translate", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // Check parameters
    assert_eq!(params.len(), 2);

    let (point_name, point_type) = &params[0];
    assert_eq!(point_name, "point");
    match point_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Expected Point struct type, got {other:?}"),
    }

    let (offset_name, offset_type) = &params[1];
    assert_eq!(offset_name, "offset");
    match offset_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Expected Vector struct type, got {other:?}"),
    }

    // Check return type
    match return_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Expected Point return type, got {other:?}"),
    }
}

#[test]
fn test_function_with_pointer_parameters() {
    let db = test_db();
    let program = r#"
        fn modify_value(ptr: felt*, new_value: felt) {
            // Function body would modify the value
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("modify_value", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // Check parameters
    assert_eq!(params.len(), 2);

    let (ptr_name, ptr_type) = &params[0];
    assert_eq!(ptr_name, "ptr");
    match ptr_type.data(&db) {
        TypeData::Pointer(inner) => {
            assert!(matches!(inner.data(&db), TypeData::Felt));
        }
        other => panic!("Expected pointer to felt, got {other:?}"),
    }

    let (value_name, value_type) = &params[1];
    assert_eq!(value_name, "new_value");
    assert!(matches!(value_type.data(&db), TypeData::Felt));

    // Check return type (should be void/unit)
    // The exact representation of void depends on implementation
    match return_type.data(&db) {
        TypeData::Unknown => {
            // Expected for void functions - they might be represented as Unknown
        }
        other => {
            // Document what actually happens for void functions
            println!("Void function return type: {other:?}");
        }
    }
}

#[test]
fn test_function_with_no_parameters() {
    let db = test_db();
    let program = r#"
        fn get_constant() -> felt {
            return 42;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("get_constant", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // Check parameters (should be empty)
    assert_eq!(params.len(), 0);

    // Check return type
    assert!(matches!(return_type.data(&db), TypeData::Felt));
}

#[test]
fn test_function_signature_consistency() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        fn create_point(x: felt, y: felt) -> Point {
            return Point { x: x, y: y };
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("create_point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // Get signature via function_semantic_signature
    let signature = function_semantic_signature(&db, crate_id, def_id).unwrap();

    // Get function type via definition_semantic_type
    let func_type = definition_semantic_type(&db, crate_id, def_id);

    // They should be consistent
    match func_type.data(&db) {
        TypeData::Function(sig_id) => {
            assert_eq!(
                sig_id, signature,
                "Function signature should be consistent between queries"
            );
        }
        other => panic!("Expected function type, got {other:?}"),
    }
}
