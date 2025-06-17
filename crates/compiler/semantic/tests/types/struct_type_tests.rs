//! Tests for `struct_semantic_data` query
//!
//! These tests verify that struct types are correctly resolved with proper
//! field information, names, and type metadata.

use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;

#[test]
fn test_simple_struct_data() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let struct_data = struct_semantic_data(&db, def_id).unwrap();

    // Check struct name
    assert_eq!(struct_data.name(&db), "Point");

    // Check fields
    let fields = struct_data.fields(&db);
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_fields = vec![("x".to_string(), felt_type), ("y".to_string(), felt_type)];
    assert_eq!(fields, expected_fields);
}

#[test]
// TODO: add boolean type?
fn test_struct_with_mixed_field_types() {
    let db = test_db();
    let program = r#"
        struct Person {
            age: felt,
            height: felt,
            is_active: felt,  // Assuming no boolean type yet
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Person", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let struct_data = struct_semantic_data(&db, def_id).unwrap();

    assert_eq!(struct_data.name(&db), "Person");

    let fields = struct_data.fields(&db);
    assert_eq!(fields.len(), 3);

    // All fields should be felt for now
    for (field_name, field_type) in &fields {
        assert!(
            matches!(field_type.data(&db), TypeData::Felt),
            "Field '{field_name}' should have felt type"
        );
    }

    // Check specific field names
    let field_names: Vec<_> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(field_names, vec!["age", "height", "is_active"]);
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
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Node", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let struct_data = struct_semantic_data(&db, def_id).unwrap();

    assert_eq!(struct_data.name(&db), "Node");

    let fields = struct_data.fields(&db);
    assert_eq!(fields.len(), 2);

    // Check value field
    let (value_name, value_type) = &fields[0];
    assert_eq!(value_name, "value");
    assert!(matches!(value_type.data(&db), TypeData::Felt));

    // Check next field (should be *Node)
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
fn test_struct_with_struct_fields() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }

        struct Rectangle {
            top_left: Point,
            bottom_right: Point,
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (rect_def_idx, _) = semantic_index
        .resolve_name_to_definition("Rectangle", root_scope)
        .unwrap();
    let rect_def_id = DefinitionId::new(&db, file, rect_def_idx);
    let rect_struct_data = struct_semantic_data(&db, rect_def_id).unwrap();

    assert_eq!(rect_struct_data.name(&db), "Rectangle");

    let fields = rect_struct_data.fields(&db);
    assert_eq!(fields.len(), 2);

    // Both fields should be Point structs
    for (field_name, field_type) in &fields {
        match field_type.data(&db) {
            TypeData::Struct(struct_id) => {
                assert_eq!(struct_id.name(&db), "Point");
            }
            other => panic!("Field '{field_name}' should be Point struct, got {other:?}"),
        }
    }

    let field_names: Vec<_> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(field_names, vec!["top_left", "bottom_right"]);
}

#[test]
fn test_empty_struct() {
    let db = test_db();
    let program = r#"
        struct Empty {
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Empty", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let struct_data = struct_semantic_data(&db, def_id).unwrap();

    assert_eq!(struct_data.name(&db), "Empty");

    let fields = struct_data.fields(&db);
    assert_eq!(fields.len(), 0, "Empty struct should have no fields");
}

#[test]
fn test_struct_in_namespace() {
    let db = test_db();
    let program = r#"
        namespace Geometry {
            struct Point {
                x: felt,
                y: felt,
            }

            struct Circle {
                center: Point,
                radius: felt,
            }
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    // Find namespace scope
    let namespace_scope = semantic_index
        .child_scopes(root_scope)
        .find(|&scope_id| {
            semantic_index.scope(scope_id).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Namespace
        })
        .expect("Should find namespace scope");

    // Test Point struct
    let (point_def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", namespace_scope)
        .unwrap();
    let point_def_id = DefinitionId::new(&db, file, point_def_idx);
    let point_struct_data = struct_semantic_data(&db, point_def_id).unwrap();

    assert_eq!(point_struct_data.name(&db), "Point");
    let point_fields = point_struct_data.fields(&db);
    assert_eq!(point_fields.len(), 2);

    // Test Circle struct
    let (circle_def_idx, _) = semantic_index
        .resolve_name_to_definition("Circle", namespace_scope)
        .unwrap();
    let circle_def_id = DefinitionId::new(&db, file, circle_def_idx);
    let circle_struct_data = struct_semantic_data(&db, circle_def_id).unwrap();

    assert_eq!(circle_struct_data.name(&db), "Circle");
    let circle_fields = circle_struct_data.fields(&db);
    assert_eq!(circle_fields.len(), 2);

    // Check Circle's center field is Point
    let (center_name, center_type) = &circle_fields[0];
    assert_eq!(center_name, "center");
    match center_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Expected Point struct for center field, got {other:?}"),
    }

    // Check Circle's radius field is felt
    let (radius_name, radius_type) = &circle_fields[1];
    assert_eq!(radius_name, "radius");
    assert!(matches!(radius_type.data(&db), TypeData::Felt));
}

#[test]
fn test_struct_data_consistency() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let semantic_index = semantic_index(&db, file)
        .as_ref()
        .expect("Got unexpected parse errors");
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // Get struct data via struct_semantic_data
    let struct_data = struct_semantic_data(&db, def_id).unwrap();

    // Get struct type via definition_semantic_type
    let def_type = definition_semantic_type(&db, def_id);

    // They should be consistent
    match def_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(
                struct_id, struct_data,
                "Struct data should be consistent between queries"
            );
        }
        other => panic!("Expected struct type, got {other:?}"),
    }
}
