
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
    let root_table = index.place_table(root).unwrap();

    // Function should be defined in root scope
    let func_place_id = root_table
        .place_id_by_name("test")
        .expect("function should be defined");
    let func_place = root_table.place(func_place_id).unwrap();
    assert!(func_place
        .flags
        .contains(crate::place::PlaceFlags::FUNCTION));

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
    let func_table = index.place_table(func_scope).unwrap();

    // Parameters should be defined in function scope
    let a_place_id = func_table
        .place_id_by_name("a")
        .expect("parameter 'a' should be defined");
    let a_place = func_table.place(a_place_id).unwrap();
    assert!(a_place.flags.contains(crate::place::PlaceFlags::PARAMETER));

    let b_place_id = func_table
        .place_id_by_name("b")
        .expect("parameter 'b' should be defined");
    let b_place = func_table.place(b_place_id).unwrap();
    assert!(b_place.flags.contains(crate::place::PlaceFlags::PARAMETER));
}

#[test]
fn test_variable_resolution() {
    let TestCase { db, source } = test_case("fn test(param: felt) { let local_var = param; }");
    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    let root = index.root_scope().unwrap();
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    let func_scope = child_scopes[0];
    let func_table = index.place_table(func_scope).unwrap();

    // Parameter should be marked as used
    let param_place_id = func_table.place_id_by_name("param").unwrap();
    let param_place = func_table.place(param_place_id).unwrap();
    assert!(param_place.is_used(), "parameter should be marked as used");

    // Local variable should be defined
    let local_place_id = func_table.place_id_by_name("local_var").unwrap();
    let local_place = func_table.place(local_place_id).unwrap();
    assert!(local_place.is_defined(), "local variable should be defined");
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

            namespace Math {
                fn square(x: felt) -> felt {
                    return x * x;
                }
            }
        "#,
    );

    let crate_id = single_file_crate(&db, source);
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Should have root scope plus function scope and namespace scope
    let root = index.root_scope().unwrap();
    let child_scopes: Vec<_> = index.child_scopes(root).collect();
    assert_eq!(
        child_scopes.len(),
        2,
        "Should have function and namespace scopes"
    );

    // Check root scope has the expected symbols
    let root_table = index.place_table(root).unwrap();
    assert!(
        root_table.place_id_by_name("PI").is_some(),
        "PI constant should be defined"
    );
    assert!(
        root_table.place_id_by_name("Point").is_some(),
        "Point struct should be defined"
    );
    assert!(
        root_table.place_id_by_name("distance").is_some(),
        "distance function should be defined"
    );
    assert!(
        root_table.place_id_by_name("Math").is_some(),
        "Math namespace should be defined"
    );

    // Check definitions are tracked
    let all_definitions = index.all_definitions().count();
    // 1 const, 1 struct, 2 functions, 1 namespace, 3 function params, 2 inner fn variables
    assert_eq!(all_definitions, 10);

    // Find function definition
    let distance_def =
        index.definition_for_place(root, root_table.place_id_by_name("distance").unwrap());
    assert!(matches!(
        distance_def,
        Some((_, def)) if matches!(def.kind, crate::definition::DefinitionKind::Function(_))
    ));

    // Check parameters and locals in function scope
    let func_scope = child_scopes
        .iter()
        .find(|&scope_id| index.scope(*scope_id).unwrap().kind == crate::place::ScopeKind::Function)
        .unwrap();

    let func_table = index.place_table(*func_scope).unwrap();
    assert!(
        func_table.place_id_by_name("p1").is_some(),
        "p1 parameter should be defined"
    );
    assert!(
        func_table.place_id_by_name("p2").is_some(),
        "p2 parameter should be defined"
    );
    assert!(
        func_table.place_id_by_name("dx").is_some(),
        "dx local should be defined"
    );
    assert!(
        func_table.place_id_by_name("dy").is_some(),
        "dy local should be defined"
    );

    // Check namespace scope
    let namespace_scope = child_scopes
        .iter()
        .find(|&scope_id| {
            index.scope(*scope_id).unwrap().kind == crate::place::ScopeKind::Namespace
        })
        .unwrap();

    let namespace_table = index.place_table(*namespace_scope).unwrap();
    assert!(
        namespace_table.place_id_by_name("square").is_some(),
        "square function should be defined in Math namespace"
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
