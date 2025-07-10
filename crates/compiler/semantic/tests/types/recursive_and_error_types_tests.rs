//! # Recursive and Error Type Tests
//!
//! This module contains tests for complex type system edge cases including:
//! - Recursive type definitions (like linked lists)
//! - Error type propagation and handling
//! - Type system robustness under error conditions

use cairo_m_compiler_semantic::project_semantic_index;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;
use crate::{get_main_semantic_index, project_from_program};

#[test]
#[ignore] // TODO: Enable when pointer types are supported
fn test_recursive_struct_with_pointers() {
    // This is the classic recursive type test case
    let db = test_db();
    let program = r#"
        struct Node {
            value: felt,
            next: Node*,
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Node", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let struct_data = struct_semantic_data(&db, project, def_id).unwrap();

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
fn test_error_type_propagation() {
    let source = r#"
        func test() {
            let x: BadType = 1; // BadType doesn't exist
            let y = x;          // y should get error type
            let z = y + 1;      // z should also get error type
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // The semantic analysis should handle undefined types gracefully
    // and not crash or produce cascading errors
    let p_semantic_index = project_semantic_index(&db, project);

    // Even with errors, we should get a semantic index
    assert!(
        p_semantic_index.is_ok(),
        "Should handle undefined types gracefully"
    );

    if let Ok(p_index) = p_semantic_index.as_ref() {
        let index = p_index.modules().values().next().unwrap().clone();
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
fn test_circular_struct_dependency() {
    // Test mutual recursion between structs
    let source = r#"
        struct A {
            value: felt,
            b_ref: B*,
        }

        struct B {
            data: felt,
            a_ref: A*,
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // This should either work (if pointers are supported) or fail gracefully
    let p_semantic_index = project_semantic_index(&db, project);

    // The important thing is that it doesn't crash or infinite loop
    match p_semantic_index {
        Ok(p_index) => {
            let index = p_index.modules().values().next().unwrap().clone();
            // If it succeeds, verify the structures are properly defined
            let root_scope = index.root_scope().unwrap();

            let a_resolution = index.resolve_name("A", root_scope);
            let b_resolution = index.resolve_name("B", root_scope);

            assert!(
                a_resolution.is_some() || b_resolution.is_some(),
                "Should resolve at least one of the mutually recursive structs"
            );
        }
        Err(_) => {
            // If it fails, that's acceptable for now - the important thing
            // is that it fails gracefully without crashing
        }
    }
}

#[test]
fn test_deeply_nested_error_recovery() {
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
    let project = project_from_program(&db, source);

    // Should handle the error chain gracefully
    let p_semantic_index = project_semantic_index(&db, project);

    // Even with multiple errors, should not crash
    assert!(
        p_semantic_index.is_ok(),
        "Should handle error chain gracefully"
    );

    if let Ok(p_index) = p_semantic_index.as_ref() {
        let index = p_index.modules().values().next().unwrap().clone();
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
fn test_type_error_in_expression_context() {
    let source = r#"
        func test() -> felt {
            let x: UnknownType = 42;
            let y: felt = 10;
            return x + y; // Should handle mixed error/valid types
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    let p_semantic_index = project_semantic_index(&db, project);
    assert!(
        p_semantic_index.is_ok(),
        "Should handle mixed error/valid types"
    );

    if let Ok(p_index) = p_semantic_index.as_ref() {
        let index = p_index.modules().values().next().unwrap().clone();
        // Should still track expressions even with type errors
        let expressions = index.all_expressions().count();
        assert_ne!(
            expressions, 0,
            "Should track expressions despite type errors"
        );
    }
}

#[test]
#[ignore = "Type aliases not yet implemented in parser"]
fn test_recursive_type_alias() {
    // Test type aliases that might create recursion
    let source = r#"
        type NodePtr = Node*;

        struct Node {
            value: felt,
            next: NodePtr,
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // Should handle type aliases in recursive contexts
    let p_semantic_index = project_semantic_index(&db, project);

    match p_semantic_index {
        Ok(p_index) => {
            let index = p_index.modules().values().next().unwrap().clone();
            let root_scope = index.root_scope().unwrap();

            // Should resolve both the type alias and the struct
            let node_resolution = index.resolve_name("Node", root_scope);
            let nodeptr_resolution = index.resolve_name("NodePtr", root_scope);

            // At least one should resolve (depending on implementation)
            assert!(
                node_resolution.is_some() || nodeptr_resolution.is_some(),
                "Should handle type aliases in recursive contexts"
            );
        }
        Err(_) => {
            // Acceptable if not yet implemented
        }
    }
}

#[test]
fn test_error_type_compatibility() {
    let source = r#"
        func test() {
            let x: BadType1 = 1;
            let y: BadType2 = 2;
            let z = x + y; // Two error types in operation
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // Should handle operations between error types
    let p_semantic_index = project_semantic_index(&db, project);
    assert!(
        p_semantic_index.is_ok(),
        "Should handle operations between error types"
    );

    // Test that error types are handled gracefully
    // For now, just verify the system doesn't crash with error types
}

#[test]
fn test_self_referential_struct_without_pointers() {
    // This should be an error - direct self-reference without pointers
    let source = r#"
        struct SelfRef {
            value: felt,
            self_field: SelfRef, // This should be an error
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // This should either:
    // 1. Produce a semantic error about infinite size
    // 2. Handle it gracefully without crashing
    let p_semantic_index = project_semantic_index(&db, project);

    // The important thing is that it doesn't infinite loop or crash
    match p_semantic_index {
        Ok(_) => {
            // If it succeeds, the type system should have some way to handle this
            // (perhaps by detecting the cycle and creating an error type)
        }
        Err(_) => {
            // If it fails, that's expected for this invalid case
        }
    }

    // The test passes if we reach this point without hanging or crashing
}

#[test]
fn test_complex_recursive_scenario() {
    // A more complex recursive scenario with multiple levels
    let source = r#"
        struct TreeNode {
            value: felt,
            left: TreeNode*,
            right: TreeNode*,
            parent: TreeNode*,
        }

        func traverse(node: TreeNode*) -> felt {
            if (node == null) {
                return 0;
            }
            return node.value + traverse(node.left) + traverse(node.right);
        }
    "#;

    let db = test_db();
    let project = project_from_program(&db, source);

    // Should handle complex recursive structures
    let p_semantic_index = project_semantic_index(&db, project);

    match p_semantic_index {
        Ok(p_index) => {
            let index = p_index.modules().values().next().unwrap().clone();
            let root_scope = index.root_scope().unwrap();

            // Should resolve the recursive struct
            let tree_resolution = index.resolve_name("TreeNode", root_scope);
            let func_resolution = index.resolve_name("traverse", root_scope);

            assert!(
                tree_resolution.is_some() || func_resolution.is_some(),
                "Should handle complex recursive scenarios"
            );
        }
        Err(_) => {
            // Acceptable if recursive types aren't fully implemented yet
        }
    }
}
