//! Tests for return type inference with literals
//!
//! These tests verify that literals in return statements correctly
//! infer their type from the function's declared return type.

use cairo_m_compiler_parser::parser::Expression;
use cairo_m_compiler_semantic::module_semantic_index;
use cairo_m_compiler_semantic::type_resolution::expression_semantic_type;
use cairo_m_compiler_semantic::types::TypeData;

use crate::{crate_from_program, test_db};

#[test]
fn test_return_literal_u32() {
    let db = test_db();
    let program = r#"
        fn test() -> u32 {
            return 0;
        }
    "#;

    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Find the literal 0 in the return statement
    let mut found_u32_literal = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if matches!(&expr_info.ast_node, Expression::Literal(0, None)) {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);
            assert!(matches!(expr_type.data(&db), TypeData::U32));
            found_u32_literal = true;
        }
    }
    assert!(found_u32_literal, "Should have found u32 literal in return");
}

#[test]
fn test_return_literal_felt() {
    let db = test_db();
    let program = r#"
        fn test() -> felt {
            return 42;
        }
    "#;

    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

    // Find the literal 42 in the return statement
    let mut found_felt_literal = false;
    for (expr_id, expr_info) in index.all_expressions() {
        if matches!(&expr_info.ast_node, Expression::Literal(42, None)) {
            let expr_type = expression_semantic_type(&db, crate_id, file, expr_id, None);
            assert!(matches!(expr_type.data(&db), TypeData::Felt));
            found_felt_literal = true;
        }
    }
    assert!(
        found_felt_literal,
        "Should have found felt literal in return"
    );
}
