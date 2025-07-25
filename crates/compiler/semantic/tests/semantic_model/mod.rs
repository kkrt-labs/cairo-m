//! # Semantic Model Tests
//!
//! This module contains white-box tests that directly verify the correctness of the
//! `SemanticIndex` and its internal data structures. These tests complement the
//! diagnostic tests by ensuring the semantic model itself is correct.
//!
//! ## Test Categories
//!
//! - **Use-Def Chains**: Verify identifier usages resolve to correct definitions
//! - **Scope Hierarchy**: Test parent-child relationships and scope traversal
//! - **Symbol Tables**: Verify place flags and symbol metadata
//! - **Span Mappings**: Test source location to semantic entity mappings
//! - **Expression Tracking**: Verify expression metadata and scope context

use cairo_m_compiler_semantic::File;
use cairo_m_compiler_semantic::db::module_semantic_index;
use cairo_m_compiler_semantic::definition::DefinitionKind;
use cairo_m_compiler_semantic::place::{PlaceFlags, ScopeKind};
use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use crate::*;

/// Helper to run a test with a semantic index
fn with_semantic_index<F>(source: &str, test_fn: F)
where
    F: FnOnce(&TestDb, File, &cairo_m_compiler_semantic::semantic_index::SemanticIndex),
{
    let db = test_db();
    let crate_id = crate_from_program(&db, source);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();
    test_fn(&db, file, &index);
}

#[test]
fn test_use_def_resolution_simple() {
    let source = r#"
        fn main() {
            let x = 42; // Definition
            let y = x;  // Usage
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        // Find the usage of `x` in `let y = x;`
        let usage_of_x = index
            .identifier_usages()
            .iter()
            .find(|u| u.name == "x")
            .expect("Could not find usage of 'x'");

        let usage_index = index
            .identifier_usages()
            .iter()
            .position(|u| u == usage_of_x)
            .unwrap();

        // Resolve it to its definition
        let definition = index
            .get_use_definition(usage_index)
            .expect("Usage of 'x' should resolve to a definition");

        // Assert that we found the correct definition
        assert_eq!(definition.name, "x");
        assert!(matches!(definition.kind, DefinitionKind::Let(_)));
    });
}

#[test]
fn test_use_def_resolution_across_scopes() {
    let source = r#"
        const global_var = 100;

        fn test() {
            let local_var = global_var; // Usage of global_var
            return local_var;           // Usage of local_var
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        // Find usage of global_var inside the function
        let global_usage = index
            .identifier_usages()
            .iter()
            .find(|u| u.name == "global_var")
            .expect("Could not find usage of 'global_var'");

        let global_usage_index = index
            .identifier_usages()
            .iter()
            .position(|u| u == global_usage)
            .unwrap();

        // Resolve to definition
        let global_definition = index
            .get_use_definition(global_usage_index)
            .expect("Usage of 'global_var' should resolve to a definition");

        assert_eq!(global_definition.name, "global_var");
        assert!(matches!(global_definition.kind, DefinitionKind::Const(_)));

        // Find usage of local_var in return statement
        let local_usage = index
            .identifier_usages()
            .iter()
            .find(|u| u.name == "local_var")
            .expect("Could not find usage of 'local_var'");

        let local_usage_index = index
            .identifier_usages()
            .iter()
            .position(|u| u == local_usage)
            .unwrap();

        let local_definition = index
            .get_use_definition(local_usage_index)
            .expect("Usage of 'local_var' should resolve to a definition");

        assert_eq!(local_definition.name, "local_var");
        assert!(matches!(local_definition.kind, DefinitionKind::Let(_)));
    });
}

#[test]
fn test_scope_hierarchy_correctness() {
    let source = r#"
        namespace Math {
            fn square(x: felt) -> felt {
                let result = x * x;
                return result;
            }
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        let root_scope = index.root_scope().expect("Should have root scope");
        let root_scope_info = index.scope(root_scope).unwrap();
        assert_eq!(root_scope_info.kind, ScopeKind::Module);
        assert_eq!(root_scope_info.parent, None);

        // Find namespace scope
        let namespace_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Namespace)
            .expect("Should find namespace scope");

        let namespace_scope_info = index.scope(namespace_scope).unwrap();
        assert_eq!(namespace_scope_info.kind, ScopeKind::Namespace);
        assert_eq!(namespace_scope_info.parent, Some(root_scope));

        // Find function scope
        let function_scope = index
            .child_scopes(namespace_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        let function_scope_info = index.scope(function_scope).unwrap();
        assert_eq!(function_scope_info.kind, ScopeKind::Function);
        assert_eq!(function_scope_info.parent, Some(namespace_scope));

        // Verify scope chain traversal works
        let mut current_scope = Some(function_scope);
        let mut depth = 0;
        while let Some(scope_id) = current_scope {
            let scope_info = index.scope(scope_id).unwrap();
            current_scope = scope_info.parent;
            depth += 1;
            if depth > 10 {
                panic!("Infinite loop in scope hierarchy");
            }
        }
        assert_eq!(depth, 3); // function -> namespace -> module
    });
}

#[test]
fn test_symbol_table_flags() {
    let source = r#"
        fn test(param: felt) {
            let used_var = 1;
            let unused_var = 2;
            return used_var + param;
        }
    "#;
    with_semantic_index(source, |_db, _file, index| {
        let root_scope = index.root_scope().unwrap();
        let function_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        let func_table = index.place_table(function_scope).unwrap();

        // Check parameter flags
        let param_place_id = func_table
            .place_id_by_name("param")
            .expect("Should find parameter");
        let param_place = func_table.place(param_place_id).unwrap();
        assert!(param_place.flags.contains(PlaceFlags::PARAMETER));
        assert!(param_place.flags.contains(PlaceFlags::USED));

        // Check used variable flags
        let used_var_place_id = func_table
            .place_id_by_name("used_var")
            .expect("Should find used_var");
        let used_var_place = func_table.place(used_var_place_id).unwrap();
        assert!(used_var_place.flags.contains(PlaceFlags::DEFINED));
        assert!(used_var_place.flags.contains(PlaceFlags::USED));

        // Check unused variable flags
        let unused_var_place_id = func_table
            .place_id_by_name("unused_var")
            .expect("Should find unused_var");
        let unused_var_place = func_table.place(unused_var_place_id).unwrap();
        assert!(unused_var_place.flags.contains(PlaceFlags::DEFINED));
        assert!(!unused_var_place.flags.contains(PlaceFlags::USED));
    });
}

#[test]
fn test_span_to_scope_mapping() {
    let source = r#"fn test() { let x = 1; }"#;

    with_semantic_index(source, |_db, _file, index| {
        // The exact spans depend on the parser, but we can verify the mapping exists
        // and that different parts of the code map to appropriate scopes
        let root_scope = index.root_scope().unwrap();
        let function_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        // Check that we have span mappings
        let span_mappings_count = index.span_to_expression_id.len();
        assert!(
            span_mappings_count > 0,
            "Should have span to expression mappings"
        );

        // Verify that expressions have proper scope context
        for expr_id in index.span_to_expression_id.values() {
            let expr_info = index.expression(*expr_id).expect("Expression should exist");
            // Expression should be in either root or function scope for this simple example
            assert!(
                expr_info.scope_id == root_scope || expr_info.scope_id == function_scope,
                "Expression should be in a valid scope"
            );
        }
    });
}

#[test]
fn test_expression_tracking() {
    let source = r#"
        fn test() {
            let a = 42;
            let b = a + 1;
            return b;
        }
    "#;

    with_semantic_index(source, |_db, file, index| {
        // Verify that expressions are tracked with correct metadata
        let expressions: Vec<_> = index.all_expressions().collect();
        assert!(!expressions.is_empty(), "Should have tracked expressions");

        for (expr_id, expr_info) in expressions {
            // Each expression should have valid metadata
            assert_eq!(expr_info.file, file);

            // Scope should be valid
            let scope_info = index.scope(expr_info.scope_id);
            assert!(scope_info.is_some(), "Expression scope should be valid");

            // Span should be non-empty
            assert!(
                expr_info.ast_span.start < expr_info.ast_span.end,
                "Expression span should be valid"
            );

            // Should be able to look up expression by span
            let looked_up_expr_id = index.expression_id_by_span(expr_info.ast_span);
            assert_eq!(
                looked_up_expr_id,
                Some(expr_id),
                "Should be able to look up expression by span"
            );
        }
    });
}

#[test]
fn test_definition_completeness() {
    let source = r#"
        const PI = 314;

        struct Point {
            x: felt,
            y: felt,
        }

        fn distance(p1: Point, p2: Point) -> felt {
            let dx = p1.x - p2.x;
            return dx;
        }
    "#;

    with_semantic_index(source, |_db, file, index| {
        let all_definitions: Vec<_> = index.all_definitions().collect();

        // Should have: 1 const, 1 struct, 1 function, 2 parameters, 1 local variable
        assert_eq!(all_definitions.len(), 6);

        // Check that each definition has proper metadata
        for (def_idx, definition) in &all_definitions {
            // Name should not be empty
            assert!(!definition.name.is_empty(), "Definition should have a name");

            // Spans should be valid
            assert!(definition.name_span.start < definition.name_span.end);
            assert!(definition.full_span.start < definition.full_span.end);

            // Should be able to create DefinitionId
            let temp_db = test_db();
            let def_id = DefinitionId::new(&temp_db, file, *def_idx);
            assert_eq!(def_id.file(&temp_db), file);
            assert_eq!(def_id.id_in_file(&temp_db), *def_idx);
        }

        // Verify specific definition kinds
        let const_def = all_definitions
            .iter()
            .find(|(_, def)| def.name == "PI")
            .expect("Should find PI constant");
        assert!(matches!(const_def.1.kind, DefinitionKind::Const(_)));

        let struct_def = all_definitions
            .iter()
            .find(|(_, def)| def.name == "Point")
            .expect("Should find Point struct");
        assert!(matches!(struct_def.1.kind, DefinitionKind::Struct(_)));

        let func_def = all_definitions
            .iter()
            .find(|(_, def)| def.name == "distance")
            .expect("Should find distance function");
        assert!(matches!(func_def.1.kind, DefinitionKind::Function(_)));
    });
}

#[test]
fn test_unresolved_identifier_tracking() {
    let source = r#"
        fn test() {
            let x = undefined_variable; // This should be tracked as unresolved
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        // Find the usage of undefined_variable
        let undefined_usage = index
            .identifier_usages()
            .iter()
            .find(|u| u.name == "undefined_variable")
            .expect("Should track usage of undefined_variable");

        let usage_index = index
            .identifier_usages()
            .iter()
            .position(|u| u == undefined_usage)
            .unwrap();

        // This usage should NOT be resolved
        assert!(
            !index.is_usage_resolved(usage_index),
            "undefined_variable should not be resolved"
        );
        assert!(
            index.get_use_definition(usage_index).is_none(),
            "undefined_variable should have no definition"
        );
    });
}

#[test]
fn test_nested_scope_name_resolution() {
    let source = r#"
        const outer = 1;

        fn test() {
            let inner = 2;
            let combined = outer + inner; // Uses both outer and inner
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        let root_scope = index.root_scope().unwrap();
        let function_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        // Test name resolution from function scope
        let outer_resolution = index.resolve_name("outer", function_scope);
        assert!(
            outer_resolution.is_some(),
            "Should resolve 'outer' from function scope"
        );

        let inner_resolution = index.resolve_name("inner", function_scope);
        assert!(
            inner_resolution.is_some(),
            "Should resolve 'inner' from function scope"
        );

        // Test that inner is not visible from root scope
        let inner_from_root = index.resolve_name("inner", root_scope);
        assert!(
            inner_from_root.is_none(),
            "'inner' should not be visible from root scope"
        );

        // Test resolve_name_to_definition
        let outer_def = index.resolve_name_to_definition("outer", function_scope);
        assert!(outer_def.is_some(), "Should resolve 'outer' to definition");
        let (_, outer_def_info) = outer_def.unwrap();
        assert_eq!(outer_def_info.name, "outer");

        let inner_def = index.resolve_name_to_definition("inner", function_scope);
        assert!(inner_def.is_some(), "Should resolve 'inner' to definition");
        let (_, inner_def_info) = inner_def.unwrap();
        assert_eq!(inner_def_info.name, "inner");
    });
}

#[test]
fn test_variable_shadowing_resolution() {
    let source = r#"
        const x = 1; // Outer x
        fn test() {
            let x = 2; // Inner x, shadows outer
            let y = x; // This should resolve to the inner x
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        // Find all definitions of 'x'
        let all_definitions: Vec<_> = index.all_definitions().collect();
        let x_definitions: Vec<_> = all_definitions
            .iter()
            .filter(|(_, def)| def.name == "x")
            .collect();

        assert_eq!(x_definitions.len(), 2, "Should have two definitions of 'x'");

        // Find the outer and inner definitions
        let outer_x = x_definitions
            .iter()
            .find(|(_, def)| {
                matches!(def.kind, DefinitionKind::Const(_)) && def.name_span.start < 20
            })
            .expect("Should find outer 'x' definition");

        let first_inner_x = x_definitions
            .iter()
            .find(|(_, def)| {
                matches!(def.kind, DefinitionKind::Let(_))
                    && def.name_span.start > 20
                    && def.name_span.end < 80
            })
            .expect("Should find inner 'x' definition");

        // Find the usage of 'x' in 'let y = x;'
        let x_usages: Vec<_> = index
            .identifier_usages()
            .iter()
            .enumerate()
            .filter(|(_, usage)| usage.name == "x")
            .collect();

        // Should have one usage (in 'let y = x;')
        assert_eq!(x_usages.len(), 1, "Should have one usage of 'x'");
        let (usage_index, _) = x_usages[0];

        // Resolve the usage to its definition
        let resolved_definition = index
            .get_use_definition(usage_index)
            .expect("Usage of 'x' should resolve to a definition");

        // The usage should resolve to the inner 'x', not the outer one
        assert_eq!(resolved_definition.name, "x");
        assert_eq!(
            resolved_definition.name_span, first_inner_x.1.name_span,
            "Usage of 'x' should resolve to the inner definition, not the outer one"
        );

        // Verify that the inner definition shadows the outer one
        let root_scope = index.root_scope().unwrap();
        let function_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        // From within the function scope, 'x' should resolve to the inner definition
        // TODO(shadowing): this doesn't support shadowing (will only return the last seen definition)
        let resolved_from_function = index.resolve_name_to_definition("x", function_scope);
        assert!(
            resolved_from_function.is_some(),
            "Should resolve 'x' from function scope"
        );
        let (_, resolved_def) = resolved_from_function.unwrap();
        assert_eq!(
            resolved_def.name_span, first_inner_x.1.name_span,
            "Name resolution from function scope should find the inner 'x'"
        );

        // From the root scope, 'x' should resolve to the outer definition
        let resolved_from_root = index.resolve_name_to_definition("x", root_scope);
        assert!(
            resolved_from_root.is_some(),
            "Should resolve 'x' from root scope"
        );
        let (_, resolved_def) = resolved_from_root.unwrap();
        assert_eq!(
            resolved_def.name_span, outer_x.1.name_span,
            "Name resolution from root scope should find the outer 'x'"
        );
    });
}

#[test]
fn test_same_scope_shadowing_resolution() {
    let source = r#"
        fn test() -> felt {
            let x = 10;     // First definition
            let y = x;      // Uses first x (10)
            let x = 20;     // Shadows x in same scope
            let z = x;      // Uses second x (20)
            let x = 30;     // Shadows x again
            let w = x;      // Uses third x (30)
            return y + z + w;  // 10 + 20 + 30 = 60
        }
    "#;

    with_semantic_index(source, |_db, _file, index| {
        // Get the function scope
        let root_scope = index.root_scope().unwrap();
        let func_scope = index
            .child_scopes(root_scope)
            .find(|&scope_id| index.scope(scope_id).unwrap().kind == ScopeKind::Function)
            .expect("Should find function scope");

        let place_table = index.place_table(func_scope).unwrap();

        // Check that we have multiple definitions of x in the same scope
        let all_x_places = place_table
            .all_place_ids_by_name("x")
            .expect("Should have x definitions");
        assert_eq!(
            all_x_places.len(),
            3,
            "Should have exactly 3 definitions of x"
        );

        // Check that place_id_by_name returns the most recent x
        let current_x = place_table.place_id_by_name("x").unwrap();
        assert_eq!(
            current_x, all_x_places[2],
            "Should return the most recent x"
        );

        // Find all usages of x
        let x_usages: Vec<_> = index
            .identifier_usages()
            .iter()
            .enumerate()
            .filter(|(_, usage)| usage.name == "x")
            .collect();

        assert_eq!(
            x_usages.len(),
            3,
            "Should have 3 usages of x (in y, z, and w assignments)"
        );

        // Verify each usage is resolved
        for (usage_index, usage) in &x_usages {
            assert!(
                index.is_usage_resolved(*usage_index),
                "Usage of '{}' should be resolved",
                usage.name
            );

            let definition = index
                .get_use_definition(*usage_index)
                .expect("Should resolve to a definition");
            assert_eq!(definition.name, "x");
        }

        // Find all definitions of x
        let all_definitions: Vec<_> = index.all_definitions().collect();
        let x_definitions: Vec<_> = all_definitions
            .iter()
            .filter(|(_, def)| def.name == "x")
            .collect();

        assert_eq!(x_definitions.len(), 3, "Should have 3 definitions of x");

        // All should be Let definitions
        for (_, def) in &x_definitions {
            assert!(matches!(def.kind, DefinitionKind::Let(_)));
        }
    });
}
