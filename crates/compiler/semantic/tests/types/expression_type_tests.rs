//! Tests for `expression_semantic_type` query
//!
//! These tests verify that the type system correctly infers types for various
//! expression kinds and handles complex expression trees.

use cairo_m_compiler_parser::parser::Expression;

use super::*;
use crate::{get_main_semantic_index, project_from_program};

#[test]
fn test_literal_expression_types() {
    let db = test_db();
    let program = r#"
        func test() {
            let a = 42;
            let b = 0;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Helper to find an expression by matching against tracked expressions
    let find_expr_id = |target_text: &str| {
        for (span, expr_id) in &semantic_index.span_to_expression_id {
            let source_text = &program[span.start..span.end];
            if source_text == target_text {
                return Some(*expr_id);
            }
        }
        None
    };

    // Test literal 42
    if let Some(expr_id) = find_expr_id("42") {
        let expr_type = expression_semantic_type(&db, project, file, expr_id);
        assert!(matches!(expr_type.data(&db), TypeData::Felt));
    }

    // Test literal 0
    if let Some(expr_id) = find_expr_id("0") {
        let expr_type = expression_semantic_type(&db, project, file, expr_id);
        assert!(matches!(expr_type.data(&db), TypeData::Felt));
    }
}

#[test]
fn test_identifier_expression_types() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test(p: Point) -> felt {
            let a = 42;
            let b = a;  // b should have type felt (same as a)
            let c = p;  // c should have type Point (same as p)
            return c.x;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Find identifier usage 'a' in 'let b = a;'
    // This is tricky because there might be multiple 'a' spans
    // We'll look for expressions that are single identifiers
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        if source_text == "a" {
            let expr_info = semantic_index.expression(*expr_id).unwrap();
            // Check if this is an identifier expression (not a definition)
            if matches!(expr_info.ast_node, Expression::Identifier(_)) {
                let expr_type = expression_semantic_type(&db, project, file, *expr_id);
                assert!(matches!(expr_type.data(&db), TypeData::Felt));
                break;
            }
        }
    }
}

#[test]
fn test_binary_expression_types() {
    let db = test_db();
    let program = r#"
        func test() {
            let a = 10;
            let b = 20;
            let sum = a + b;      // Should be felt
            let diff = a - b;     // Should be felt
            let prod = a * b;     // Should be felt
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for binary expressions
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        // Check for binary operations
        if matches!(
            expr_info.ast_node,
            cairo_m_compiler_parser::parser::Expression::BinaryOp { .. }
        ) {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Binary operations on felt should result in felt
            assert!(
                matches!(expr_type.data(&db), TypeData::Felt),
                "Binary expression '{source_text}' should have felt type"
            );
        }
    }
}

#[test]
fn test_member_access_expression_types() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test(p: Point) -> felt {
            let x_val = p.x;  // Should be felt
            let y_val = p.y;  // Should be felt
            return x_val;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for member access expressions
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        // Check for member access (p.x or p.y)
        if matches!(
            expr_info.ast_node,
            cairo_m_compiler_parser::parser::Expression::MemberAccess { .. }
        ) {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Member access to felt fields should result in felt
            assert!(
                matches!(expr_type.data(&db), TypeData::Felt),
                "Member access '{source_text}' should have felt type"
            );
        }
    }
}

#[test]
fn test_function_call_expression_types() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }

        func make_point(x: felt, y: felt) -> Point {
            return Point { x: x, y: y };
        }

        func test() -> Point {
            let p = make_point(1, 2);  // Should be Point
            return p;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for function call expressions
    let mut found_make_point = false;
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        // Check for function calls
        if matches!(
            expr_info.ast_node,
            cairo_m_compiler_parser::parser::Expression::FunctionCall { .. }
        ) && source_text.contains("make_point")
        {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Function call should return Point
            assert!(
                matches!(expr_type.data(&db), TypeData::Struct(_)),
                "Function call '{source_text}' should have struct type"
            );
            found_make_point = true;
        }
    }
    assert!(found_make_point, "Function call 'make_point' not found");
}

#[test]
fn test_struct_literal_expression_types() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test() -> Point {
            let p = Point { x: 1, y: 2 };  // Should be Point
            return p;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for struct literal expressions
    // Check for struct literals
    let mut found_struct_literal = false;
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        if matches!(
            expr_info.ast_node,
            cairo_m_compiler_parser::parser::Expression::StructLiteral { .. }
        ) {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Struct literal should have the struct type
            match expr_type.data(&db) {
                TypeData::Struct(struct_id) => {
                    assert_eq!(struct_id.name(&db), "Point");
                }
                other => {
                    panic!("Struct literal '{source_text}' should have struct type, got {other:?}")
                }
            }
            found_struct_literal = true;
        }
    }
    assert!(found_struct_literal, "Struct literal not found");
}

#[test]
fn test_complex_expression_type_inference() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }

        func distance_squared(p1: Point, p2: Point) -> felt {
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            let result = dx * dx + dy * dy;  // Complex expression
            return result;
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for the complex expression 'dx * dx + dy * dy'
    let mut found_complex_expression = false;
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        // Look for binary expressions that might be our complex expression
        if matches!(expr_info.ast_node, Expression::BinaryOp { .. })
            && source_text.contains("dx * dx + dy * dy")
        {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Complex arithmetic expression should result in felt
            assert!(
                matches!(expr_type.data(&db), TypeData::Felt),
                "Complex expression '{source_text}' should have felt type"
            );
            found_complex_expression = true;
        }
    }
    assert!(found_complex_expression, "Complex expression not found");
}

#[test]
fn test_unary_expression_types() {
    let db = test_db();
    let program = r#"
        func test() {
            let a = 10;
            let neg_a = -a;       // Should be felt
            let not_a = !a;       // Should be felt
            let neg_lit = -42;    // Should be felt
            let not_lit = !0;     // Should be felt
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Look for unary expressions
    for (span, expr_id) in &semantic_index.span_to_expression_id {
        let source_text = &program[span.start..span.end];
        let expr_info = semantic_index.expression(*expr_id).unwrap();

        // Check for unary operations
        if matches!(
            expr_info.ast_node,
            cairo_m_compiler_parser::parser::Expression::UnaryOp { .. }
        ) {
            let expr_type = expression_semantic_type(&db, project, file, *expr_id);
            // Unary operations on felt should result in felt
            assert!(
                matches!(expr_type.data(&db), TypeData::Felt),
                "Unary expression '{source_text}' should have felt type"
            );
        }
    }
}

#[test]
fn test_unary_operation_type_errors() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test() {
            let p = Point { x: 1, y: 2 };
            let invalid_neg = -p;     // Error: negation on struct
            let invalid_not = !p;     // Error: logical not on struct

            let tuple = (1, 2);
            let invalid_neg2 = -tuple; // Error: negation on tuple
            let invalid_not2 = !tuple; // Error: logical not on tuple
        }
    "#;
    let project = project_from_program(&db, program);
    let file = *project.modules(&db).values().next().unwrap();
    let semantic_index = get_main_semantic_index(&db, project);

    // Run type validation
    use cairo_m_compiler_semantic::validation::Validator;
    use cairo_m_compiler_semantic::validation::type_validator::TypeValidator;

    let validator = TypeValidator;
    let diagnostics = validator.validate(&db, project, file, &semantic_index);

    // Should have type mismatch errors for invalid unary operations
    let type_errors = diagnostics
        .iter()
        .filter(|d| d.code == cairo_m_compiler_diagnostics::DiagnosticCode::TypeMismatch)
        .count();

    assert!(
        type_errors >= 4,
        "Should have at least 4 type mismatch errors for invalid unary operations, got {}",
        type_errors
    );
}
