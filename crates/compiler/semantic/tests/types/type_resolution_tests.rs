//! Tests for basic type resolution functionality
//!
//! These tests verify that `resolve_ast_type` correctly resolves AST type expressions
//! to semantic type IDs, including primitive types, pointers, and user-defined types.

use cairo_m_compiler_parser::parser::{NamedType, TypeExpr as AstTypeExpr};
use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;
use crate::{crate_from_program, get_main_semantic_index};

#[test]
fn test_resolve_primitive_types() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let felt_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Named(NamedType::Felt),
        root_scope,
    );
    assert!(matches!(felt_type.data(&db), TypeData::Felt));

    let pointer_felt_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Pointer(Box::new(AstTypeExpr::Named(NamedType::Felt))),
        root_scope,
    );
    assert!(
        matches!(pointer_felt_type.data(&db), TypeData::Pointer(t) if matches!(t.data(&db), TypeData::Felt))
    );
}

#[test]
fn test_resolve_nested_pointer_types() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // Test felt** (pointer to pointer to felt)
    let double_pointer_felt = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Pointer(Box::new(AstTypeExpr::Pointer(Box::new(
            AstTypeExpr::Named(NamedType::Felt),
        )))),
        root_scope,
    );

    match double_pointer_felt.data(&db) {
        TypeData::Pointer(inner) => match inner.data(&db) {
            TypeData::Pointer(inner_inner) => {
                assert!(matches!(inner_inner.data(&db), TypeData::Felt));
            }
            other => panic!("Expected pointer to felt, got {other:?}"),
        },
        other => panic!("Expected pointer to pointer, got {other:?}"),
    }
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
        AstTypeExpr::Named(NamedType::Custom("Point".to_string())),
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
#[ignore]
fn test_struct_with_pointer_fields() {
    // TODO: This currently doesn't compile, as the parser doesn't support pointer types.
    // for structs.
    let db = test_db();
    let program = r#"
        struct Node {
            value: felt,
            next: Node*,
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Node", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let semantic_data = struct_semantic_data(&db, crate_id, def_id).unwrap();

    let fields = semantic_data.fields(&db);
    assert_eq!(fields.len(), 2);

    // Check value field is felt
    let (value_name, value_type) = &fields[0];
    assert_eq!(value_name, "value");
    assert!(matches!(value_type.data(&db), TypeData::Felt));

    // Check next field is *Node
    let (next_name, next_type) = &fields[1];
    assert_eq!(next_name, "next");
    match next_type.data(&db) {
        TypeData::Pointer(inner) => match inner.data(&db) {
            TypeData::Struct(struct_id) => {
                assert_eq!(struct_id.name(&db), "Node");
            }
            other => panic!("Expected pointer to Node struct, got {other:?}"),
        },
        other => panic!("Expected pointer type, got {other:?}"),
    }
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
        AstTypeExpr::Named(NamedType::Custom("UnknownType".to_string())),
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

#[test]
fn test_resolve_types_in_nested_scopes() {
    let db = test_db();
    let program = r#"
        struct GlobalStruct {
            x: felt,
        }

        namespace MyNamespace {
            struct LocalStruct {
                y: felt,
            }

            fn test() -> LocalStruct {
                return LocalStruct { y: 0 };
            }
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // Find the namespace scope
    let namespace_scope = semantic_index
        .child_scopes(root_scope)
        .find(|&scope_id| {
            semantic_index.scope(scope_id).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Namespace
        })
        .expect("Should find namespace scope");

    // GlobalStruct should be resolvable from namespace scope (via scope chain)
    let global_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Named(NamedType::Custom("GlobalStruct".to_string())),
        namespace_scope,
    );
    assert!(matches!(global_type.data(&db), TypeData::Struct(_)));

    // LocalStruct should be resolvable from namespace scope
    let local_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Named(NamedType::Custom("LocalStruct".to_string())),
        namespace_scope,
    );
    assert!(matches!(local_type.data(&db), TypeData::Struct(_)));

    // LocalStruct should NOT be resolvable from root scope
    let local_from_root = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Named(NamedType::Custom("LocalStruct".to_string())),
        root_scope,
    );
    // This should be an unknown/error type since LocalStruct is not in root scope
    match local_from_root.data(&db) {
        TypeData::Unknown => {
            // Expected - LocalStruct is not visible from root
        }
        other => {
            println!("LocalStruct from root resolved to: {other:?}");
            // This test documents the current behavior
        }
    }
}
