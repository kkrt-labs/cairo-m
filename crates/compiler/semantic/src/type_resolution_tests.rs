use chumsky::span::SimpleSpan;

use super::*;
use crate::db::tests::{crate_from_program, test_db};
use crate::module_semantic_index;
use crate::place::FileScopeId;
use crate::semantic_index::DefinitionIndex;

// Helper functions for tests
fn spanned<T>(value: T) -> Spanned<T> {
    Spanned::new(value, SimpleSpan::from(0..0))
}

fn named_type(name: NamedType) -> Spanned<AstTypeExpr> {
    spanned(AstTypeExpr::Named(spanned(name)))
}

fn tuple_type(elements: Vec<Spanned<AstTypeExpr>>) -> Spanned<AstTypeExpr> {
    spanned(AstTypeExpr::Tuple(elements))
}

#[test]
fn test_resolve_felt_type() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let scope_id = FileScopeId::new(0);

    let felt_type = resolve_ast_type(&db, crate_id, file, named_type(NamedType::Felt), scope_id);
    let felt_data = felt_type.data(&db);

    assert!(matches!(felt_data, TypeData::Felt));
}

#[test]
fn test_resolve_u32_type() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let scope_id = FileScopeId::new(0);

    let u32_type = resolve_ast_type(&db, crate_id, file, named_type(NamedType::U32), scope_id);
    let u32_data = u32_type.data(&db);

    assert!(matches!(u32_data, TypeData::U32));
}

#[test]
fn test_resolve_tuple_type() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let scope_id = FileScopeId::new(0);

    let tuple_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        tuple_type(vec![
            named_type(NamedType::Felt),
            named_type(NamedType::Felt),
        ]),
        scope_id,
    );
    let tuple_data = tuple_type.data(&db);

    match tuple_data {
        TypeData::Tuple(types) => {
            assert_eq!(types.len(), 2);
            for type_id in types {
                let inner_data = type_id.data(&db);
                assert!(matches!(inner_data, TypeData::Felt));
            }
        }
        _ => panic!("Expected tuple type"),
    }
}

#[test]
fn test_type_compatibility() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();

    let felt1 = TypeId::new(&db, TypeData::Felt);
    let felt2 = TypeId::new(&db, TypeData::Felt);
    let u32_1 = TypeId::new(&db, TypeData::U32);
    let u32_2 = TypeId::new(&db, TypeData::U32);
    let bool_type = TypeId::new(&db, TypeData::Bool);
    let error_type = TypeId::new(&db, TypeData::Error);
    let unknown_type = TypeId::new(&db, TypeData::Unknown);

    // Same types should be compatible
    assert!(are_types_compatible(&db, felt1, felt2));
    assert!(are_types_compatible(&db, u32_1, u32_2));
    assert!(are_types_compatible(&db, bool_type, bool_type));

    // Different primitive types should NOT be compatible
    assert!(!are_types_compatible(&db, felt1, u32_1));
    assert!(!are_types_compatible(&db, u32_1, felt1));
    assert!(!are_types_compatible(&db, felt1, bool_type));
    assert!(!are_types_compatible(&db, u32_1, bool_type));

    // Error and Unknown types should be compatible with anything
    assert!(are_types_compatible(&db, felt1, error_type));
    assert!(are_types_compatible(&db, error_type, felt1));
    assert!(are_types_compatible(&db, felt1, unknown_type));
    assert!(are_types_compatible(&db, unknown_type, felt1));
    assert!(are_types_compatible(&db, u32_1, error_type));
    assert!(are_types_compatible(&db, error_type, u32_1));
    assert!(are_types_compatible(&db, u32_1, unknown_type));
    assert!(are_types_compatible(&db, unknown_type, u32_1));

    // Structs should be compatible if they have the same definitions.
    let def_id_1 = DefinitionId::new(&db, file, DefinitionIndex::from(0));
    let def_id_2 = DefinitionId::new(&db, file, DefinitionIndex::from(1));
    let scope_id = FileScopeId::new(0);

    let struct_type_id1 = StructTypeId::new(&db, def_id_1, "struct1".to_string(), vec![], scope_id);
    let struct_type_id1_dup =
        StructTypeId::new(&db, def_id_1, "struct1".to_string(), vec![], scope_id);
    let struct_type_id2 = StructTypeId::new(&db, def_id_2, "struct2".to_string(), vec![], scope_id);

    let instance_def_1_1 = TypeId::new(&db, TypeData::Struct(struct_type_id1));
    let instance_def_1_2 = TypeId::new(&db, TypeData::Struct(struct_type_id1_dup));
    let instance_def_2_1 = TypeId::new(&db, TypeData::Struct(struct_type_id2));

    assert!(are_types_compatible(
        &db,
        instance_def_1_1,
        instance_def_1_2
    ));
    assert!(!are_types_compatible(
        &db,
        instance_def_1_1,
        instance_def_2_1
    ));

    // Tuples should be compatible if they have the same length and compatible elements
    let tuple1 = TypeId::new(&db, TypeData::Tuple(vec![felt1, felt2]));
    let tuple2 = TypeId::new(&db, TypeData::Tuple(vec![felt1, felt2]));
    assert!(are_types_compatible(&db, tuple1, tuple2));
}

#[test]
fn test_direct_ast_node_access() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "fn test() { let x = 42; }");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Find any expression in the index
    let all_expressions: Vec<_> = semantic_index.all_expressions().collect();
    assert!(
        !all_expressions.is_empty(),
        "Should have at least one expression"
    );

    for (expr_id, expr_info) in all_expressions {
        // Verify that we can access the AST node directly without lookup
        match &expr_info.ast_node {
            Expression::Literal(value, _) => {
                // Test that we can access literal values directly
                assert_eq!(*value, 42);

                // Verify the expression type can be resolved efficiently
                let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);
                assert!(matches!(expr_type.data(&db), TypeData::Felt));
            }
            Expression::Identifier(name) => {
                // Test that we can access identifier names directly
                assert_eq!(name.value(), "x");
            }
            _ => {
                panic!("Test data does not contain this expr")
            }
        }

        // Verify that span information is still available for diagnostics
        assert!(expr_info.ast_span.start < expr_info.ast_span.end);
    }
}

#[test]
fn test_expression_type_coverage() {
    let db = test_db();

    // Simple test program that exercises all expression types
    let program = r#"
            struct Point { x: felt, y: felt }
            fn test() {
                let p = Point { x: 1, y: 2 };
                let sum = 1 + p.y;
                let coord = p.x;
                return;
            }
        "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Count how many different expression types we find
    let mut expression_types_found = std::collections::HashSet::new();

    // We are expecting to find that many expressions;
    let expected_expression_count = 9;
    assert_eq!(
        semantic_index.all_expressions().count(),
        expected_expression_count
    );

    for (expr_id, expr_info) in semantic_index.all_expressions() {
        let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);

        // Record the expression variant we found
        let variant_name = match &expr_info.ast_node {
            Expression::Literal(_, _) => "Literal",
            Expression::BooleanLiteral(_) => "BooleanLiteral",
            Expression::Identifier(_) => "Identifier",
            Expression::UnaryOp { .. } => "UnaryOp",
            Expression::BinaryOp { .. } => "BinaryOp",
            Expression::Parenthesized(_) => "Parenthesized",
            Expression::FunctionCall { .. } => "FunctionCall",
            Expression::MemberAccess { .. } => "MemberAccess",
            Expression::IndexAccess { .. } => "IndexAccess",
            Expression::StructLiteral { .. } => "StructLiteral",
            Expression::Tuple(_) => "Tuple",
            Expression::ArrayLiteral(_) => "ArrayLiteral",
            Expression::ArrayRepeat { .. } => "ArrayRepeat",
            Expression::TupleIndex { .. } => "TupleIndex",
            Expression::Cast { .. } => "Cast",
            Expression::New { .. } => "New",
        };
        expression_types_found.insert(variant_name);

        // Verify we never return Unknown type
        assert!(!matches!(expr_type.data(&db), TypeData::Unknown));

        // Basic sanity checks
        match &expr_info.ast_node {
            Expression::Literal(_, _) => {
                assert!(matches!(expr_type.data(&db), TypeData::Felt));
            }
            Expression::BinaryOp {
                op: _,
                left: _,
                right: _,
            } => {
                assert!(matches!(expr_type.data(&db), TypeData::Felt));
            }
            Expression::MemberAccess {
                object: _,
                field: _,
            } => {
                assert_eq!(expr_type.data(&db), TypeData::Felt);
            }
            Expression::StructLiteral { name, .. } if name.value() == "Point" => {
                if let TypeData::Struct(struct_id) = expr_type.data(&db) {
                    assert_eq!(struct_id.name(&db), "Point");
                } else {
                    panic!("Expected struct type, got {:?}", expr_type.data(&db));
                }
            }
            _ => {
                // For other expression types, just ensure we get some valid type
                assert!(!matches!(expr_type.data(&db), TypeData::Unknown));
            }
        }
    }

    // Verify we found the main expression types in our test program
    assert!(expression_types_found.contains("Literal"));
    assert!(expression_types_found.contains("StructLiteral"));
    assert!(expression_types_found.contains("BinaryOp"));
    assert!(expression_types_found.contains("MemberAccess"));
}

#[test]
fn test_member_access_edge_cases() {
    let db = test_db();
    let program = r#"
            struct Point { x: felt, y: felt }
            struct Nested { point: Point, value: felt }

            fn test(p: Point, ptr: felt*, nested: Nested) -> felt {
                let x1 = p.x;           // Direct struct field access
                let n1 = nested.value;  // Nested struct field
                let n2 = nested.point;  // Nested struct returns Point type
                return x1;
            }
        "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Find member access expressions and verify their types
    for expr_id in semantic_index.span_expression_mappings().values() {
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        if let Expression::MemberAccess { object: _, field } = &expr_info.ast_node {
            let expr_type = expression_semantic_type(&db, crate_id, file, *expr_id, None);

            match field.value().as_str() {
                "x" | "value" => {
                    // These should be felt type
                    assert!(
                        matches!(expr_type.data(&db), TypeData::Felt),
                        "Field {} should have felt type",
                        field.value()
                    );
                }
                "point" => {
                    // This should be Point type
                    assert!(
                        matches!(expr_type.data(&db), TypeData::Struct(_)),
                        "Field {} should have struct type",
                        field.value()
                    );
                }
                _ => {}
            }
        }
    }
}

#[test]
fn test_typed_const_resolution() {
    let db = test_db();
    let program = r#"
        const SIZE: felt = 3;
        const MAX: u32 = 100;
        const PI = 314;  // No type annotation, should infer felt
        const POW2: [u32; 3] = [1, 2, 4];

        fn test_felt() -> felt {
            return SIZE;
        }

        fn test_u32() -> u32 {
            return MAX;
        }

        fn test_inferred() -> felt {
            return PI;
        }
    "#;

    let crate_id = crate_from_program(&db, program);
    let main_file = *crate_id.modules(&db).values().next().unwrap();
    let module_index = module_semantic_index(&db, crate_id, "main".into()).unwrap();

    // Check SIZE const has felt type
    let size_def_idx = module_index
        .latest_definition_index_by_name(FileScopeId::new(0), "SIZE")
        .expect("SIZE const should be defined");
    let size_def_id = DefinitionId::new(&db, main_file, size_def_idx);
    let size_type = definition_semantic_type(&db, crate_id, size_def_id);
    assert!(
        matches!(size_type.data(&db), TypeData::Felt),
        "SIZE should have felt type"
    );

    // Check MAX const has u32 type
    let max_def_idx = module_index
        .latest_definition_index_by_name(FileScopeId::new(0), "MAX")
        .expect("MAX const should be defined");
    let max_def_id = DefinitionId::new(&db, main_file, max_def_idx);
    let max_type = definition_semantic_type(&db, crate_id, max_def_id);
    assert!(
        matches!(max_type.data(&db), TypeData::U32),
        "MAX should have u32 type"
    );

    // Check PI const infers felt type
    let pi_def_idx = module_index
        .latest_definition_index_by_name(FileScopeId::new(0), "PI")
        .expect("PI const should be defined");
    let pi_def_id = DefinitionId::new(&db, main_file, pi_def_idx);
    let pi_type = definition_semantic_type(&db, crate_id, pi_def_id);
    assert!(
        matches!(pi_type.data(&db), TypeData::Felt),
        "PI should infer felt type"
    );

    // Check POW2 const has [u32; 3] type
    let pow2_def_idx = module_index
        .latest_definition_index_by_name(FileScopeId::new(0), "POW2")
        .expect("POW2 const should be defined");
    let pow2_def_id = DefinitionId::new(&db, main_file, pow2_def_idx);
    let pow2_type = definition_semantic_type(&db, crate_id, pow2_def_id);
    let pow2_data = pow2_type.data(&db);
    if let TypeData::FixedArray { element_type, size } = pow2_data {
        assert_eq!(size, 3, "POW2 should have size 3");
        assert!(
            matches!(element_type.data(&db), TypeData::U32),
            "POW2 element type should be U32"
        );
    } else {
        panic!("POW2 should be a FixedArray type");
    }
}
