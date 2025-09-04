//! Tests for fixed-size array type system integration

use cairo_m_compiler_semantic::db::project_validate_semantics;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::types::{TypeData, TypeId};

use super::*;
use crate::{crate_from_program, get_main_semantic_index};

#[test]
fn test_fixed_array_type_creation() {
    let db = test_db();

    // Create element type
    let element_type = TypeId::new(&db, TypeData::Felt);

    // Create array type
    let array_type = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type,
            size: 5,
        },
    );

    // Verify the array type
    match array_type.data(&db) {
        TypeData::FixedArray {
            element_type: elem,
            size,
        } => {
            assert_eq!(size, 5);
            assert_eq!(elem, element_type);
        }
        _ => panic!("Expected FixedArray type"),
    }
}

#[test]
fn test_fixed_array_type_formatting() {
    let db = test_db();

    // Create [felt; 3] type
    let felt_array = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::Felt),
            size: 3,
        },
    );

    assert_eq!(TypeId::format_type(&db, felt_array), "[felt; 3]");

    // Create [u32; 10] type
    let u32_array = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::U32),
            size: 10,
        },
    );

    assert_eq!(TypeId::format_type(&db, u32_array), "[u32; 10]");

    // Create [bool; 0] type (empty array)
    let empty_array = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::Bool),
            size: 0,
        },
    );

    assert_eq!(TypeId::format_type(&db, empty_array), "[bool; 0]");
}

#[test]
fn test_nested_array_rejection() {
    let db = test_db();

    let source = r#"
        fn test_nested_arrays() {
            // This should be rejected during semantic analysis
            let arr: [[felt; 3]; 2] = [[1,2,3], [4,5,6]];
        }
    "#;

    let crate_id = crate_from_program(&db, source);

    // Should have semantic errors for nested arrays
    let diagnostics = project_validate_semantics(&db, crate_id);

    // We expect at least one error about nested arrays
    assert!(
        !diagnostics.all().is_empty(),
        "Expected errors for nested arrays"
    );

    // Check that at least one diagnostic mentions nested arrays
    let has_nested_array_error = diagnostics
        .all()
        .iter()
        .any(|d| d.message.contains("nested") || d.message.contains("Nested"));
    assert!(
        has_nested_array_error,
        "Expected specific error about nested arrays"
    );
}

#[test]
fn test_array_element_type_inference() {
    let db = test_db();

    let source = r#"
        fn test_array_inference() {
            let arr = [1u32, 2u32, 3u32];
            let elem = arr[0];  // Should infer elem as u32
        }
    "#;

    let crate_id = crate_from_program(&db, source);
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

    // Find the type of 'elem' variable
    let elem_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "elem")
        .unwrap();
    let elem_def_id = DefinitionId::new(&db, file, elem_def_idx);
    let elem_type = definition_semantic_type(&db, crate_id, elem_def_id);
    assert!(matches!(elem_type.data(&db), TypeData::U32));
}

#[test]
fn test_array_size_mismatch_detection() {
    let db = test_db();

    let source = r#"
        fn test_size_mismatch() {
            let arr: [felt; 3] = [1, 2];  // Size mismatch: expected 3, got 2
        }
    "#;

    let crate_id = crate_from_program(&db, source);

    // Should have semantic errors for size mismatch
    let diagnostics = project_validate_semantics(&db, crate_id);

    assert!(
        !diagnostics.all().is_empty(),
        "Expected errors for array size mismatch"
    );

    // Check for size mismatch error
    let has_size_mismatch_error = diagnostics.all().iter().any(|d| {
        d.message.contains("size")
            || d.message.contains("Size")
            || d.message.contains("mismatch")
            || d.message.contains("Mismatch")
    });
    assert!(
        has_size_mismatch_error,
        "Expected specific error about size mismatch"
    );
}

#[test]
fn test_array_type_compatibility() {
    let db = test_db();

    // Test that [felt; 3] and [felt; 3] are compatible
    let array1 = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::Felt),
            size: 3,
        },
    );

    let array2 = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::Felt),
            size: 3,
        },
    );

    // Should be equal due to interning
    assert_eq!(array1, array2);

    // Test that [felt; 3] and [felt; 4] are not compatible
    let array3 = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::Felt),
            size: 4,
        },
    );

    assert_ne!(array1, array3);

    // Test that [felt; 3] and [u32; 3] are not compatible
    let array4 = TypeId::new(
        &db,
        TypeData::FixedArray {
            element_type: TypeId::new(&db, TypeData::U32),
            size: 3,
        },
    );

    assert_ne!(array1, array4);
}

#[test]
fn test_array_literal_type_inference() {
    let db = test_db();

    let source = r#"
        fn test_array_literals() {
            let a = [1, 2, 3];        // Should infer [felt; 3]
            let b = [true, false];    // Should infer [bool; 2]
            let c = [1u32, 2u32];     // Should infer [u32; 2]
        }
    "#;

    let crate_id = crate_from_program(&db, source);
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

    // Check type of 'a'
    let a_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "a")
        .unwrap();
    let a_def_id = DefinitionId::new(&db, file, a_def_idx);
    let a_type = definition_semantic_type(&db, crate_id, a_def_id);
    match a_type.data(&db) {
        TypeData::FixedArray { element_type, size } => {
            assert_eq!(size, 3);
            assert!(matches!(element_type.data(&db), TypeData::Felt));
        }
        _ => panic!("Expected FixedArray type for 'a'"),
    }

    // Check type of 'b'
    let b_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "b")
        .unwrap();
    let b_def_id = DefinitionId::new(&db, file, b_def_idx);
    let b_type = definition_semantic_type(&db, crate_id, b_def_id);
    match b_type.data(&db) {
        TypeData::FixedArray { element_type, size } => {
            assert_eq!(size, 2);
            assert!(matches!(element_type.data(&db), TypeData::Bool));
        }
        _ => panic!("Expected FixedArray type for 'b'"),
    }

    // Check type of 'c'
    let c_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "c")
        .unwrap();
    let c_def_id = DefinitionId::new(&db, file, c_def_idx);
    let c_type = definition_semantic_type(&db, crate_id, c_def_id);
    match c_type.data(&db) {
        TypeData::FixedArray { element_type, size } => {
            assert_eq!(size, 2);
            assert!(matches!(element_type.data(&db), TypeData::U32));
        }
        _ => panic!("Expected FixedArray type for 'c'"),
    }
}

#[test]
fn test_array_mixed_types_error() {
    let db = test_db();

    let source = r#"
        fn test_mixed_types() {
            let arr = [1, true, 3];  // Mixed types: felt and bool
        }
    "#;

    let crate_id = crate_from_program(&db, source);

    // Should have semantic errors for mixed types
    let diagnostics = project_validate_semantics(&db, crate_id);

    assert!(
        !diagnostics.all().is_empty(),
        "Expected errors for mixed types in array"
    );

    // Check for type mismatch error
    let has_type_error = diagnostics.all().iter().any(|d| {
        d.message.contains("type")
            || d.message.contains("Type")
            || d.message.contains("mismatch")
            || d.message.contains("Mismatch")
    });
    assert!(
        has_type_error,
        "Expected type mismatch error for mixed array elements"
    );
}

#[test]
fn test_array_index_out_of_bounds() {
    let db = test_db();

    let source = r#"
        fn test_bounds() {
            let arr: [felt; 3] = [1, 2, 3];
            let x = arr[3];  // Out of bounds: index 3 for size 3
            let y = arr[10]; // Out of bounds: index 10 for size 3
        }
    "#;

    let crate_id = crate_from_program(&db, source);

    // Should have semantic errors for out of bounds access
    let diagnostics = project_validate_semantics(&db, crate_id);

    assert!(
        !diagnostics.all().is_empty(),
        "Expected errors for out of bounds access"
    );

    // Should have at least 2 bounds errors
    let bounds_errors = diagnostics
        .all()
        .iter()
        .filter(|d| {
            d.message.contains("bounds")
                || d.message.contains("Bounds")
                || d.message.contains("index")
                || d.message.contains("Index")
        })
        .count();
    assert!(
        bounds_errors >= 2,
        "Expected at least 2 out of bounds errors"
    );
}

#[test]
fn test_array_with_explicit_type() {
    let db = test_db();

    let source = r#"
        fn test_explicit_type() {
            let arr: [u32; 4] = [1u32, 2u32, 3u32, 4u32];
            let elem = arr[0];
        }
    "#;

    let crate_id = crate_from_program(&db, source);
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

    // Check type of 'arr'
    let arr_def_idx = semantic_index
        .latest_definition_index_by_name(func_scope, "arr")
        .unwrap();
    let arr_def_id = DefinitionId::new(&db, file, arr_def_idx);
    let arr_type = definition_semantic_type(&db, crate_id, arr_def_id);
    match arr_type.data(&db) {
        TypeData::FixedArray { element_type, size } => {
            assert_eq!(size, 4);
            assert!(matches!(element_type.data(&db), TypeData::U32));
        }
        _ => panic!("Expected FixedArray type for 'arr'"),
    }
}
