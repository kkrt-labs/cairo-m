//! Tests for `definition_semantic_type` query
//!
//! These tests verify that the type system correctly determines the semantic type
//! of various definition kinds (variables, parameters, functions, etc.).

use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::{Project, SemanticIndex, project_semantic_index};

use super::*;
use crate::project_from_program;

fn get_main_semantic_index(db: &dyn SemanticDb, project: Project) -> SemanticIndex {
    let semantic_index = project_semantic_index(db, project).unwrap();
    semantic_index.modules().values().next().unwrap().clone()
}

#[test]
fn test_let_variable_type_inference() {
    let db = test_db();
    let program = r#"
        func test() {
            let x = 42;        // Should infer felt
            let y: felt = 100; // Explicit felt type
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| {
            semantic_index.scope(*s).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // Test inferred type
    let (x_def_idx, _) = semantic_index
        .resolve_name_to_definition("x", func_scope)
        .unwrap();
    let x_def_id = DefinitionId::new(&db, file, x_def_idx);
    let x_type = definition_semantic_type(&db, project, x_def_id);
    assert!(matches!(x_type.data(&db), TypeData::Felt));

    // Test explicit type
    let (y_def_idx, _) = semantic_index
        .resolve_name_to_definition("y", func_scope)
        .unwrap();
    let y_def_id = DefinitionId::new(&db, file, y_def_idx);
    let y_type = definition_semantic_type(&db, project, y_def_id);
    assert!(matches!(y_type.data(&db), TypeData::Felt));
}

#[test]
fn test_parameter_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Vector { x: felt, y: felt }
        func magnitude(v: Vector, scale: felt) -> felt {
            return 0;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| {
            semantic_index.scope(*s).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // Test struct parameter
    let (v_def_idx, _) = semantic_index
        .resolve_name_to_definition("v", func_scope)
        .unwrap();
    let v_def_id = DefinitionId::new(&db, file, v_def_idx);
    let v_type = definition_semantic_type(&db, project, v_def_id);
    match v_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Expected struct type, got {other:?}"),
    }

    // Test felt parameter
    let (scale_def_idx, _) = semantic_index
        .resolve_name_to_definition("scale", func_scope)
        .unwrap();
    let scale_def_id = DefinitionId::new(&db, file, scale_def_idx);
    let scale_type = definition_semantic_type(&db, project, scale_def_id);
    assert!(matches!(scale_type.data(&db), TypeData::Felt));
}

#[test]
fn test_function_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func get_point(x: felt) -> Point {
            return Point { x: x, y: 0 };
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // Get the function definition
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("get_point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // Check the full function type from its definition
    let func_type = definition_semantic_type(&db, project, def_id);
    match func_type.data(&db) {
        TypeData::Function(sig_id) => {
            let signature = function_semantic_signature(&db, project, def_id).unwrap();
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
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // Test inferred const type
    let (pi_def_idx, _) = semantic_index
        .resolve_name_to_definition("PI", root_scope)
        .unwrap();
    let pi_def_id = DefinitionId::new(&db, file, pi_def_idx);
    let pi_type = definition_semantic_type(&db, project, pi_def_id);
    assert!(matches!(pi_type.data(&db), TypeData::Felt));

    // Test explicit const type
    let (max_def_idx, _) = semantic_index
        .resolve_name_to_definition("MAX_SIZE", root_scope)
        .unwrap();
    let max_def_id = DefinitionId::new(&db, file, max_def_idx);
    let max_type = definition_semantic_type(&db, project, max_def_id);
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
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // The type of a struct definition should be the struct type itself
    let struct_type = definition_semantic_type(&db, project, def_id);
    match struct_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Expected struct type, got {other:?}"),
    }
}

#[test]
fn test_pointer_variable_types() {
    let db = test_db();
    // TODO: There should be a compile time issue assigning 0 to a pointer without casts.
    // For now we have no support for casts, so, this should not even compile (type checks).
    // TODO: Add typechecks for literal -> pointers.
    let program = r#"
        func test() {
            let ptr: felt* = 0;
            let double_ptr: felt** = 0;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| {
            semantic_index.scope(*s).unwrap().kind
                == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // Test single pointer
    let (ptr_def_idx, _) = semantic_index
        .resolve_name_to_definition("ptr", func_scope)
        .unwrap();
    let ptr_def_id = DefinitionId::new(&db, file, ptr_def_idx);
    let ptr_type = definition_semantic_type(&db, project, ptr_def_id);
    match ptr_type.data(&db) {
        TypeData::Pointer(inner) => {
            assert!(matches!(inner.data(&db), TypeData::Felt));
        }
        other => panic!("Expected pointer type, got {other:?}"),
    }

    // Test double pointer
    let (double_ptr_def_idx, _) = semantic_index
        .resolve_name_to_definition("double_ptr", func_scope)
        .unwrap();
    let double_ptr_def_id = DefinitionId::new(&db, file, double_ptr_def_idx);
    let double_ptr_type = definition_semantic_type(&db, project, double_ptr_def_id);
    match double_ptr_type.data(&db) {
        TypeData::Pointer(outer) => match outer.data(&db) {
            TypeData::Pointer(inner) => {
                assert!(matches!(inner.data(&db), TypeData::Felt));
            }
            other => panic!("Expected pointer to felt, got {other:?}"),
        },
        other => panic!("Expected pointer to pointer, got {other:?}"),
    }
}
