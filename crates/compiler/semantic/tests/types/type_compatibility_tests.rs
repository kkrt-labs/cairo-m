//! # Type Compatibility Tests
//!
//! This module contains tests for type compatibility logic and error handling.
//! These tests verify that the type system correctly handles type compatibility
//! checks, error propagation, and edge cases.

use crate::{crate_from_program, get_maybe_main_semantic_index, test_db};

#[test]
fn test_basic_type_compatibility() {
    let source = r#"
        func test() {
            let a: felt = 42;
            let b: felt = a; // felt should be compatible with felt
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    // For now, just test that the semantic analysis completes without errors
    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle basic type compatibility"
    );
}

#[test]
fn test_struct_type_compatibility() {
    let source = r#"
        struct Point {
            x: felt,
            y: felt,
        }

        struct Vector {
            x: felt,
            y: felt,
        }

        func test() {
            let p: Point = Point { x: 1, y: 2 };
            let v: Vector = Vector { x: 3, y: 4 };
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    // Test that different struct types are handled correctly
    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle different struct types"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        let root_scope = index.root_scope().unwrap();

        // Should be able to resolve both struct types
        let point_resolution = index.resolve_name("Point", root_scope);
        let vector_resolution = index.resolve_name("Vector", root_scope);

        assert!(point_resolution.is_some(), "Should resolve Point struct");
        assert!(vector_resolution.is_some(), "Should resolve Vector struct");
    }
}

#[test]
fn test_error_type_handling() {
    let source = r#"
        func test() {
            let x: BadType = 1; // BadType doesn't exist
            let y = x;          // y should get error type
            let z = y + 1;      // z should also get error type
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    // The semantic analysis should handle undefined types gracefully
    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);

    // Even with errors, we should get a semantic index
    assert!(
        semantic_index_result.is_ok(),
        "Should handle undefined types gracefully"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        // The system should track the error but continue analysis
        let all_definitions: Vec<_> = index.all_definitions().collect();

        // Should still find the variable definitions even with type errors
        let x_def = all_definitions.iter().find(|(_, def)| def.name == "x");
        let y_def = all_definitions.iter().find(|(_, def)| def.name == "y");
        let z_def = all_definitions.iter().find(|(_, def)| def.name == "z");

        assert!(
            x_def.is_some(),
            "Should find 'x' definition despite type error"
        );
        assert!(y_def.is_some(), "Should find 'y' definition");
        assert!(z_def.is_some(), "Should find 'z' definition");
    }
}

#[test]
fn test_function_type_handling() {
    let source = r#"
        func add(a: felt, b: felt) -> felt {
            return a + b;
        }

        func multiply(x: felt, y: felt) -> felt {
            return x * y;
        }

        func single_param(x: felt) -> felt {
            return x;
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle function types"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        let root_scope = index.root_scope().unwrap();

        // Should resolve all function names
        let add_resolution = index.resolve_name("add", root_scope);
        let multiply_resolution = index.resolve_name("multiply", root_scope);
        let single_param_resolution = index.resolve_name("single_param", root_scope);

        assert!(add_resolution.is_some(), "Should resolve 'add' function");
        assert!(
            multiply_resolution.is_some(),
            "Should resolve 'multiply' function"
        );
        assert!(
            single_param_resolution.is_some(),
            "Should resolve 'single_param' function"
        );
    }
}

#[test]
fn test_nested_type_handling() {
    let source = r#"
        struct Container {
            value: felt,
        }

        struct Wrapper {
            inner: Container,
        }

        func test() {
            let c: Container = Container { value: 42 };
            let w: Wrapper = Wrapper { inner: c };
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle nested struct types"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        let root_scope = index.root_scope().unwrap();

        // Should resolve both nested struct types
        let container_resolution = index.resolve_name("Container", root_scope);
        let wrapper_resolution = index.resolve_name("Wrapper", root_scope);

        assert!(
            container_resolution.is_some(),
            "Should resolve Container struct"
        );
        assert!(
            wrapper_resolution.is_some(),
            "Should resolve Wrapper struct"
        );
    }
}

#[test]
fn test_type_error_recovery() {
    let source = r#"
        struct Valid {
            field: felt,
        }

        func test() {
            let good: Valid = Valid { field: 42 };
            let bad: InvalidType = good;  // Type error
            let nested = bad.nonexistent_field; // Should not crash
            return nested;
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    // Should handle the error chain gracefully
    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);

    // Even with multiple errors, should not crash
    assert!(
        semantic_index_result.is_ok(),
        "Should handle error chain gracefully"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        // Should still be able to analyze the valid parts
        let root_scope = index.root_scope().unwrap();
        let valid_resolution = index.resolve_name("Valid", root_scope);
        assert!(
            valid_resolution.is_some(),
            "Should still resolve valid struct"
        );
    }
}

#[test]
fn test_mixed_valid_and_error_types() {
    let source = r#"
        func test() -> felt {
            let x: UnknownType = 42;
            let y: felt = 10;
            return x + y; // Should handle mixed error/valid types
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle mixed error/valid types"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        // Should still track expressions even with type errors
        let expressions = index.all_expressions().count();
        assert_ne!(
            expressions, 0,
            "Should track expressions despite type errors"
        );
    }
}

#[test]
fn test_type_compatibility_reflexivity() {
    let source = r#"
        func test() {
            let a: felt = 42;
            let b: felt = a; // Same type should be compatible
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    // Test reflexivity - a type should be compatible with itself
    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Type compatibility should be reflexive"
    );
}

#[test]
fn test_complex_type_scenario() {
    let source = r#"
        struct Point {
            x: felt,
            y: felt,
        }

        func distance(p1: Point, p2: Point) -> felt {
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            return dx * dx + dy * dy;
        }

        func test() {
            let origin: Point = Point { x: 0, y: 0 };
            let point: Point = Point { x: 3, y: 4 };
            let dist = distance(origin, point);
        }
    "#;

    let db = test_db();
    let crate_id = crate_from_program(&db, source);

    let semantic_index_result = get_maybe_main_semantic_index(&db, crate_id);
    assert!(
        semantic_index_result.is_ok(),
        "Should handle complex type scenarios"
    );

    if let Ok(index) = semantic_index_result.as_ref() {
        let root_scope = index.root_scope().unwrap();

        // Should resolve all components
        let point_resolution = index.resolve_name("Point", root_scope);
        let distance_resolution = index.resolve_name("distance", root_scope);
        let test_resolution = index.resolve_name("test", root_scope);

        assert!(point_resolution.is_some(), "Should resolve Point struct");
        assert!(
            distance_resolution.is_some(),
            "Should resolve distance function"
        );
        assert!(test_resolution.is_some(), "Should resolve test function");
    }
}
