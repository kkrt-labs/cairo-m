//! Tests for assignment statement validation

use crate::*;

#[test]
fn test_simple_assignment() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        x = 20;
        return;
    "
    ));
}

#[test]
fn test_assignment_to_undeclared_variable() {
    assert_semantic_err!(&in_function(
        "
        x = 42; // Error: undeclared variable
    "
    ));
}

#[test]
fn test_assignment_with_expression() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        let y = 5;
        x = y + 15;
        return;
    "
    ));
}

#[test]
fn test_assignment_with_undeclared_in_rhs() {
    assert_semantic_err!(&in_function(
        "
        let x = 10;
        x = undefined_var + 5; // Error: undeclared variable in RHS
    "
    ));
}

#[test]
fn test_assignment_to_parameter() {
    assert_semantic_ok!(&in_function_with_params(
        "
        param = 42;
        return;
    ",
        "param: felt"
    ));
}

#[test]
fn test_assignment_in_nested_scope() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        {
            x = 20; // OK: x is visible from outer scope
        }
        return;
    "
    ));
}

#[test]
fn test_assignment_to_variable_from_outer_scope() {
    assert_semantic_ok!(&in_function(
        "
        let outer = 1;
        {
            let inner = 2;
            outer = inner + 1; // OK: outer is visible
        }
        return;
    "
    ));
}

#[test]
fn test_assignment_to_variable_not_in_scope() {
    assert_semantic_err!(&in_function(
        "
        {
            let inner = 42;
        }
        inner = 10; // Error: inner not in scope
    "
    ));
}

#[test]
fn test_assignment_with_function_call() {
    assert_semantic_ok!(&with_functions(
        "fn get_value() -> felt { return 42; }",
        &in_function(
            "
            let x = 10;
            x = get_value();
            return;
        "
        )
    ));
}

#[test]
fn test_assignment_with_complex_expression() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        let y = 5;
        let z = 3;
        x = (y + z) * 2 - 1;
        return;
    "
    ));
}

#[test]
fn test_multiple_assignments() {
    assert_semantic_ok!(&in_function(
        "
        let x = 1;
        let y = 2;
        x = 10;
        y = 20;
        x = y + 5;
        return;
    "
    ));
}

#[test]
fn test_assignment_in_if_statement() {
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        if (x == 5) {
            x = 20;
        } else {
            x = 30;
        }
        return;
    "
    ));
}

#[test]
fn test_assignment_rhs_creates_usage() {
    // Variables used in assignment RHS should be marked as used
    assert_semantic_ok!(&in_function(
        "
        let x = 10;
        let y = 5;
        x = y + 1; // y is used here
        return;
    "
    ));
}
