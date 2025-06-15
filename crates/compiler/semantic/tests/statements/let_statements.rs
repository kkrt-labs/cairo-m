//! Tests for let statement validation

use crate::*;

#[test]
fn test_simple_let_statement() {
    assert_semantic_ok!(&in_function("let x = 42; \n return();"));
}

#[test]
fn test_let_with_expression() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        let y = x + 20;
        return y;
    "
    ));
}

#[test]
fn test_let_with_undeclared_variable() {
    assert_semantic_err!(&in_function("let x = undefined_var;"));
}

#[test]
fn test_multiple_let_statements() {
    assert_semantic_ok!(&in_function(
        "
        let a = 1;
        let b = 2;
        let c = a + b;
        return c;
    "
    ));
}

#[test]
fn test_let_statement_duplicate() {
    assert_semantic_err!(&in_function(
        "
        let x = 1;
        let x = 2; // Duplicate definition
    "
    ));
}

#[test]
fn test_let_statement_with_function_call() {
    assert_semantic_ok!(&with_functions(
        "func helper() -> felt { return 42; }",
        &in_function(
            "
            let x = helper();
            return x;
        "
        )
    ));
}

#[test]
fn test_let_statement_in_nested_scope() {
    assert_semantic_ok!(&in_function(
        "
        let outer = 1;
        {
            let inner = outer + 1;
            return inner;
        }
    "
    ));
}

#[test]
fn test_let_statement_type_annotation() {
    assert_semantic_ok!(&in_function(
        "
        let x: felt = 42;
        return x;
    "
    ));
}

#[test]
fn test_let_statement_with_complex_expression() {
    assert_semantic_ok!(&in_function(
        "
        let a = 10;
        let b = 20;
        let result = (a + b) * 2 - 5;
        return result;
    "
    ));
}
