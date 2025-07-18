//! Tests for u32 type support
//!
//! These tests verify that the u32 type is properly integrated into the type system,
//! including type resolution, type inference, compatibility checks, and literal validation.

use cairo_m_compiler_parser::parser::{NamedType, TypeExpr as AstTypeExpr};
use cairo_m_compiler_semantic::module_semantic_index;
use cairo_m_compiler_semantic::type_resolution::{are_types_compatible, expression_semantic_type};
use cairo_m_compiler_semantic::types::{TypeData, TypeId};

use super::*;
use crate::{crate_from_program, get_main_semantic_index};

#[test]
fn test_u32_type_resolution() {
    let db = test_db();
    let crate_id = crate_from_program(&db, "");
    let file = *crate_id.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, crate_id);
    let root_scope = semantic_index.root_scope().unwrap();

    // Test resolving "u32" as a type name
    let u32_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Named(NamedType::U32),
        root_scope,
    );
    assert!(matches!(u32_type.data(&db), TypeData::U32));

    // Test resolving "u32*" (pointer to u32)
    let u32_pointer_type = resolve_ast_type(
        &db,
        crate_id,
        file,
        AstTypeExpr::Pointer(Box::new(AstTypeExpr::Named(NamedType::U32))),
        root_scope,
    );
    match u32_pointer_type.data(&db) {
        TypeData::Pointer(inner_type) => {
            assert!(matches!(inner_type.data(&db), TypeData::U32));
        }
        _ => panic!("Expected pointer to u32"),
    }
}

#[test]
fn test_u32_explicit_declaration() {
    let db = test_db();
    let program = "fn test() { let x: u32 = 42; let y = x; return;}"; // Use x so we get an identifier expression
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Find the identifier 'x' and verify its type
    let mut found_u32_var = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::Identifier(name) = &expr_info.ast_node {
            if name.value() == "x" {
                let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
                assert!(matches!(expr_type.data(&db), TypeData::U32));
                found_u32_var = true;
            }
        }
    }
    assert!(found_u32_var, "Should have found u32 variable 'x'");
}

#[test]
fn test_u32_arithmetic_operations() {
    let db = test_db();
    let program = r#"
        fn test() {
            let a: u32 = 10;
            let b: u32 = 20;
            let sum = a + b;
            let diff = a - b;
            let prod = a * b;
            let quot = a / b;
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Count binary operations and verify they return u32
    let mut u32_operations = 0;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::BinaryOp { .. } = &expr_info.ast_node {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
            if matches!(expr_type.data(&db), TypeData::U32) {
                u32_operations += 1;
            }
        }
    }
    assert_eq!(u32_operations, 4, "Should have 4 u32 arithmetic operations");
}

#[test]
fn test_u32_comparison_operations() {
    let db = test_db();
    let program = r#"
        fn test() -> bool {
            let a: u32 = 10;
            let b: u32 = 20;
            let eq = a == b;
            let neq = a != b;
            let lt = a < b;
            let gt = a > b;
            let lte = a <= b;
            let gte = a >= b;
            return eq;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Count comparison operations and verify they return bool
    let mut bool_operations = 0;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::BinaryOp {
            op:
                cairo_m_compiler_parser::parser::BinaryOp::Eq
                | cairo_m_compiler_parser::parser::BinaryOp::Neq
                | cairo_m_compiler_parser::parser::BinaryOp::Less
                | cairo_m_compiler_parser::parser::BinaryOp::Greater
                | cairo_m_compiler_parser::parser::BinaryOp::LessEqual
                | cairo_m_compiler_parser::parser::BinaryOp::GreaterEqual,
            ..
        } = &expr_info.ast_node
        {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
            if matches!(expr_type.data(&db), TypeData::Bool) {
                bool_operations += 1;
            }
        }
    }
    assert_eq!(
        bool_operations, 6,
        "Should have 6 comparison operations returning bool"
    );
}

#[test]
fn test_u32_unary_operations() {
    let db = test_db();
    let program = r#"
        fn test() {
            let a: u32 = 10;
            let neg = -a;
            let not = !a;
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    let mut found_neg_u32 = false;
    let mut found_not_bool = false;

    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::UnaryOp { op, .. } = &expr_info.ast_node
        {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
            match op {
                cairo_m_compiler_parser::parser::UnaryOp::Neg => {
                    if matches!(expr_type.data(&db), TypeData::U32) {
                        found_neg_u32 = true;
                    }
                }
                cairo_m_compiler_parser::parser::UnaryOp::Not => {
                    if matches!(expr_type.data(&db), TypeData::Bool) {
                        found_not_bool = true;
                    }
                }
            }
        }
    }

    assert!(found_neg_u32, "Should have found negation returning u32");
    assert!(
        found_not_bool,
        "Should have found logical not returning bool"
    );
}

#[test]
fn test_u32_felt_incompatibility() {
    let db = test_db();

    // Create type IDs
    let u32_type = TypeId::new(&db, TypeData::U32);
    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test that u32 and felt are not compatible
    assert!(!are_types_compatible(&db, u32_type, felt_type));
    assert!(!are_types_compatible(&db, felt_type, u32_type));
}

#[test]
fn test_u32_tuple_type() {
    let db = test_db();
    let program = r#"
        fn test() {
            let pair: (u32, u32) = (10, 20);
            let first = pair;  // Use pair so we get an identifier expression
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Find the tuple variable and verify its type
    let mut found_u32_tuple = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::Identifier(name) = &expr_info.ast_node {
            if name.value() == "pair" {
                let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
                if let TypeData::Tuple(types) = expr_type.data(&db) {
                    assert_eq!(types.len(), 2);
                    assert!(matches!(types[0].data(&db), TypeData::U32));
                    assert!(matches!(types[1].data(&db), TypeData::U32));
                    found_u32_tuple = true;
                }
            }
        }
    }
    assert!(found_u32_tuple, "Should have found u32 tuple type");
}

#[test]
fn test_u32_in_struct() {
    let db = test_db();
    let program = r#"
        struct Counter {
            value: u32,
            max: u32,
        }

        fn test() {
            let c = Counter { value: 0, max: 120 };
            let v = c.value;
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Find member access and verify it returns u32
    let mut found_u32_field = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::MemberAccess { field, .. } =
            &expr_info.ast_node
        {
            if field.value() == "value" {
                let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
                assert!(matches!(expr_type.data(&db), TypeData::U32));
                found_u32_field = true;
            }
        }
    }
    assert!(found_u32_field, "Should have found u32 struct field");
}

#[test]
fn test_u32_in_function_signature() {
    let db = test_db();
    let program = r#"
        fn add_u32(a: u32, b: u32) -> u32 {
            return a + b;
        }

        fn test() {
            let result = add_u32(10, 20);
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Find the function call and verify its return type
    let mut found_u32_call = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::FunctionCall { .. } =
            &expr_info.ast_node
        {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
            if matches!(expr_type.data(&db), TypeData::U32) {
                found_u32_call = true;
            }
        }
    }
    assert!(
        found_u32_call,
        "Should have found function call returning u32"
    );
}

#[test]
fn test_u32_default_literal_inference() {
    let db = test_db();
    // Without explicit type, literals should default to felt
    let program = r#"
        fn test() {
            let x = 42;  // Should be felt, not u32
            return;
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string());

    // Find the identifier 'x' and verify its type
    for (expr_id, expr_info) in index.all_expressions() {
        if let cairo_m_compiler_parser::parser::Expression::Identifier(name) = &expr_info.ast_node {
            if name.value() == "x" {
                let expr_type = expression_semantic_type(&db, crate_id, file, expr_id);
                assert!(matches!(expr_type.data(&db), TypeData::Felt));
                assert!(!matches!(expr_type.data(&db), TypeData::U32));
            }
        }
    }
}

// Integration tests using assert_semantic_ok! and assert_semantic_err! macros
#[test]
fn test_u32_type_checking() {
    assert_semantic_parameterized! {
        ok: [
            // u32 literals can be assigned to u32 variables
            "fn test() -> u32 { let x: u32 = 10; return x; }",
        ],
        err: [
            // u32 and felt are not compatible
            in_function("let x: u32 = 42; let y: felt = x;"),
            in_function("let z:felt = 3; let u: u32 = z;"),
            // Mismatch in binary op
            in_function("let x: u32 = 10; let y: felt = 20; let z = x + y;"),
            // Struct field assignment from felt literal
            "struct Config { port: u32 } fn create_config() -> Config { return Config { port: 8080 }; }",
        ]
    }
}
