//! Tests for let statement validation

use crate::*;

#[test]
fn test_simple_let_statement() {
    assert_semantic_ok!(&in_function("let x = 42; \n return();"));
}

#[test]
fn test_simple_local_statement() {
    assert_semantic_ok!(&in_function("local x = 42; \n return();"));
}

#[test]
fn test_let_with_expression() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        let y = x + 20;
        return;
    "
    ));
}

#[test]
fn test_local_with_expression() {
    assert_semantic_ok!(&in_function(
        "
        local x = 10;
        local y = x + 20;
        return;
    "
    ));
}

#[test]
fn test_let_with_undeclared_variable() {
    assert_semantic_err!(&in_function("let x = undefined_var; return();"));
}

#[test]
fn test_multiple_let_statements() {
    assert_semantic_ok!(&in_function(
        "
        let a = 1;
        let b = 2;
        let c = a + b;
        return;
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
            return;
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
            return;
        }
    "
    ));
}

#[test]
fn test_let_statement_type_annotation() {
    assert_semantic_ok!(&in_function(
        "
        let x: felt = 42;
        return;
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
        return;
    "
    ));
}

#[test]
fn test_let_statement_shadowing() {
    assert_semantic_ok!(&in_function(
        "
        let x = 1;
        let x = 2;
        return;
    "
    ));
}

#[test]
fn test_local_statement_shadowing() {
    assert_semantic_ok!(&in_function(
        "
        local x = 1;
        local x = 2;
        return;
    "
    ));
}
