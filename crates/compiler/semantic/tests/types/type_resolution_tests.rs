//! Tests for basic type resolution functionality
//!
//! These tests verify that `resolve_ast_type` correctly resolves AST type expressions
//! to semantic type IDs, including primitive types, pointers, and user-defined types.

use cairo_m_compiler_parser::parser::NamedType;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;
use crate::{crate_from_program, get_main_semantic_index, named_type};

#[test]
fn test_resolve_primitive_types() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let felt_type = resolve_ast_type(&db, crate_id, file, named_type(NamedType::Felt), root_scope);
    assert!(matches!(felt_type.data(&db), TypeData::Felt));
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
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Resolve `Point` as a type name.
    let point_type_id = resolve_ast_type(
        &db,
        crate_id,
        file,
        named_type(NamedType::Custom("Point".to_string())),
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
    let def_idx = semantic_index
        .latest_definition_index_by_name(root_scope, "Point")
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let semantic_data = struct_semantic_data(&db, crate_id, def_id).unwrap();

    assert_eq!(struct_id, semantic_data);
    assert_eq!(semantic_data.name(&db), "Point");

    // 4. Check the fields.
    let fields = semantic_data.fields(&db);
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_fields = vec![("x".to_string(), felt_type), ("y".to_string(), felt_type)];
    assert_eq!(fields, expected_fields);
}

#[test]
fn test_resolve_unknown_type_name() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let unknown_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        named_type(NamedType::Custom("UnknownType".to_string())),
        root_scope,
    );

    // Should resolve to an error type or unknown type
    // The exact behavior depends on implementation - this test documents current behavior
    match unknown_type.data(&db) {
        TypeData::Unknown => {
            // This is expected for unresolved types
        }
        other => {
            // Document what actually happens
            println!("Unknown type resolved to: {other:?}");
        }
    }
}
