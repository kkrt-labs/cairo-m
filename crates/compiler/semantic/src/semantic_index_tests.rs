use std::path::PathBuf;

use cairo_m_compiler_parser::SourceFile;

use super::*;
use crate::db::tests::{test_db, TestDb};
use crate::{module_semantic_index, SemanticDb};

struct TestCase {
    db: TestDb,
    source: SourceFile,
}

fn test_case(content: &str) -> TestCase {
    let db = test_db();
    let source = SourceFile::new(&db, content.to_string(), "test.cm".to_string());
    TestCase { db, source }
}

//TODO For tests only - ideally not present there
fn single_file_crate(db: &dyn SemanticDb, file: File) -> Crate {
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    Crate::new(
        db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    )
}

#[test]
fn test_empty_program() {
    let TestCase { db, source } = test_case("");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    let root = index.root_scope().expect("should have root scope");
    let scope = index.scope(root).unwrap();
    assert_eq!(scope.kind, crate::place::ScopeKind::Module);
    assert_eq!(scope.parent, None);
}

#[test]
fn test_simple_function() {
    let TestCase { db, source } = test_case("fn test() { }");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Should have root scope and function scope
    let root = index.root_scope().unwrap();
    // Function should be defined in root scope
    let def_idx = index
        .latest_definition_index_by_name(root, "test")
        .expect("function should be defined");
    let def = index.definition(def_idx).unwrap();
    assert!(matches!(
        def.kind,
        crate::definition::DefinitionKind::Function(_)
    ));

    // Should have one child scope (the function)
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    assert_eq!(child_scopes.len(), 1);

    let func_scope = child_scopes[0];
    let func_scope_info = index.scope(func_scope).unwrap();
    assert_eq!(func_scope_info.kind, crate::place::ScopeKind::Function);
}

#[test]
fn test_function_with_parameters() {
    let TestCase { db, source } = test_case("fn add(a: felt, b: felt) { }");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    let root = index.root_scope().unwrap();
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    let func_scope = child_scopes[0];
    // Parameters should be defined in function scope
    let names: Vec<_> = index
        .definitions_in_scope(func_scope)
        .map(|(_, d)| (d.name.clone(), &d.kind))
        .collect();
    assert!(names
        .iter()
        .any(|(n, k)| n == "a" && matches!(k, crate::definition::DefinitionKind::Parameter(_))));
    assert!(names
        .iter()
        .any(|(n, k)| n == "b" && matches!(k, crate::definition::DefinitionKind::Parameter(_))));
}

#[test]
fn test_variable_resolution() {
    let TestCase { db, source } = test_case("fn test(param: felt) { let local_var = param; }");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    let root = index.root_scope().unwrap();
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    let func_scope = child_scopes[0];
    // Parameter should be marked as used
    let param_def_idx = index
        .latest_definition_index_by_name(func_scope, "param")
        .expect("parameter should be defined");
    assert!(
        index.is_definition_used(param_def_idx),
        "parameter should be marked as used"
    );

    // Local variable should be defined
    let local_def_idx = index
        .latest_definition_index_by_name(func_scope, "local_var")
        .expect("local variable should be defined");
    let local_def = index.definition(local_def_idx).unwrap();
    assert!(matches!(
        local_def.kind,
        crate::definition::DefinitionKind::Let(_)
    ));
}

#[test]
fn test_comprehensive_semantic_analysis() {
    let TestCase { db, source } = test_case(
        r#"
            const PI = 314;

            struct Point {
                x: felt,
                y: felt
            }

            fn distance(p1: Point, p2: Point) -> felt {
                let dx = p1.x - p2.x;
                let dy: felt = p1.y - p2.y;
                return dx * dx + dy * dy;
            }

            fn square(x: felt) -> felt {
                return x * x;
            }
        "#,
    );

    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Should have root scope plus function scope
    let root = index.root_scope().unwrap();
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    assert_eq!(child_scopes.len(), 2, "Should have function scopes");

    // Check root scope has the expected symbols
    assert!(
        index.latest_definition_index_by_name(root, "PI").is_some(),
        "PI constant should be defined"
    );
    assert!(
        index
            .latest_definition_index_by_name(root, "Point")
            .is_some(),
        "Point struct should be defined"
    );
    assert!(
        index
            .latest_definition_index_by_name(root, "distance")
            .is_some(),
        "distance function should be defined"
    );
    assert!(
        index
            .latest_definition_index_by_name(root, "square")
            .is_some(),
        "square function should be defined"
    );

    // Check definitions are tracked
    let all_definitions = index.all_definitions().count();
    // 1 const, 1 struct, 2 functions, 3 function params, 2 inner fn variables
    assert_eq!(all_definitions, 9);

    // Find function definition
    let distance_idx = index
        .latest_definition_index_by_name(root, "distance")
        .unwrap();
    let distance_def = index.definition(distance_idx).unwrap();
    assert!(matches!(
        distance_def.kind,
        crate::definition::DefinitionKind::Function(_)
    ));

    // Check parameters and locals in function scope
    let func_scope = child_scopes
        .iter()
        .find(|&scope_id| index.scope(*scope_id).unwrap().kind == crate::place::ScopeKind::Function)
        .unwrap();

    let scope_names: Vec<String> = index
        .definitions_in_scope(*func_scope)
        .map(|(_, d)| d.name.clone())
        .collect();
    assert!(
        scope_names.iter().any(|n| n == "p1"),
        "p1 parameter should be defined"
    );
    assert!(
        scope_names.iter().any(|n| n == "p2"),
        "p2 parameter should be defined"
    );
    assert!(
        scope_names.iter().any(|n| n == "dx"),
        "dx local should be defined"
    );
    assert!(
        scope_names.iter().any(|n| n == "dy"),
        "dy local should be defined"
    );
}

#[test]
fn test_real_spans_are_used() {
    let TestCase { db, source } = test_case("fn test(x: felt) { let y = x; }");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Get all identifier usages
    let usages = index.identifier_usages();

    // Should have at least one usage for the identifier 'x' being used
    let x_usage = usages.iter().find(|u| u.name == "x");
    assert!(x_usage.is_some(), "Should find usage of identifier 'x'");

    let x_usage = x_usage.unwrap();
    // Verify that real spans are being used (not dummy spans)
    assert_ne!(
        x_usage.span,
        SimpleSpan::from(0..0),
        "Should not use dummy span for identifier usage"
    );
    assert!(
        x_usage.span.start < x_usage.span.end,
        "Span should have positive length"
    );

    // Check definitions also have real spans
    let definitions: Vec<_> = index.all_definitions().collect();
    assert!(!definitions.is_empty(), "Should have definitions");

    for (_, def) in definitions {
        assert_ne!(
            def.name_span,
            SimpleSpan::from(0..0),
            "Definition name span should not be dummy"
        );
        assert_ne!(
            def.full_span,
            SimpleSpan::from(0..0),
            "Definition full span should not be dummy"
        );
        assert!(
            def.name_span.start < def.name_span.end,
            "Name span should have positive length"
        );
        assert!(
            def.full_span.start < def.full_span.end,
            "Full span should have positive length"
        );
    }
}

#[test]
fn test_definition_expression_ids_are_captured() {
    let TestCase { db, source } = test_case(
        r#"
            const Z = 314;

            fn test() -> felt {
                let x = 42;
                let y: felt = 100;
                return x + y;
            }
            "#,
    );
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Find the let definition
    let let_definitions: Vec<_> = index
        .all_definitions()
        .filter_map(|(_, def)| match &def.kind {
            DefinitionKind::Let(let_ref) => Some(let_ref),
            _ => None,
        })
        .collect();

    // Find the const definition
    let const_definitions: Vec<_> = index
        .all_definitions()
        .filter_map(|(_, def)| match &def.kind {
            DefinitionKind::Const(const_ref) => Some(const_ref),
            _ => None,
        })
        .collect();

    // Check that we found the expected definitions
    assert_eq!(
        let_definitions.len(),
        2, // Now we have 2 let definitions (x and y)
        "Should find exactly 2 let definitions"
    );
    assert_eq!(
        const_definitions.len(),
        1,
        "Should find exactly 1 const definition"
    );

    // Verify the names and expression IDs
    assert_eq!(let_definitions[0].name, "x");
    assert!(
        let_definitions[0].value_expr_id.is_some(),
        "Let definition should have a value expression ID"
    );

    assert_eq!(let_definitions[1].name, "y");
    assert!(
        let_definitions[1].value_expr_id.is_some(),
        "Let definition should have a value expression ID"
    );

    assert_eq!(const_definitions[0].name, "Z");
    assert!(
        const_definitions[0].value_expr_id.is_some(),
        "Const definition should have a value expression ID"
    );

    // Verify that the expression IDs actually correspond to real expressions in the index
    for let_def in &let_definitions {
        if let Some(expr_id) = let_def.value_expr_id {
            assert!(
                index.expression(expr_id).is_some(),
                "Expression ID should be valid"
            );
        }
    }
    for const_def in &const_definitions {
        if let Some(expr_id) = const_def.value_expr_id {
            assert!(
                index.expression(expr_id).is_some(),
                "Expression ID should be valid"
            );
        }
    }
}
