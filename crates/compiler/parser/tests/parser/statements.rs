use crate::common::in_function;
use crate::{assert_parses_err, assert_parses_ok};

// ===================
// Variable Declarations
// ===================

#[test]
fn let_statement_simple() {
    assert_parses_ok!(&in_function("let x = 5;"));
}

#[test]
fn let_statement_with_type() {
    assert_parses_ok!(&in_function("let x: felt = 5;"));
}

#[test]
fn let_statement_with_expression() {
    assert_parses_ok!(&in_function("let result = a + b * c;"));
}

#[test]
fn let_statement_missing_semicolon() {
    assert_parses_err!(&in_function("let x = 5"));
}

#[test]
fn local_statement_with_type() {
    assert_parses_ok!(&in_function("local x: felt = 42;"));
}

#[test]
fn local_statement_without_type() {
    assert_parses_ok!(&in_function("local x = infer_me;"));
}

#[test]
fn const_statement() {
    assert_parses_ok!(&in_function("const PI = 314;"));
}

// ===================
// Assignment Statements
// ===================

#[test]
fn simple_assignment() {
    assert_parses_ok!(&in_function("x = 5;"));
}

#[test]
fn member_assignment() {
    assert_parses_ok!(&in_function("obj.field = value;"));
}

#[test]
fn index_assignment() {
    assert_parses_ok!(&in_function("arr[0] = item;"));
}

#[test]
fn missing_assignment_target() {
    assert_parses_err!(&in_function("= 5;"));
}

// ===================
// Return Statements
// ===================

#[test]
fn return_with_value() {
    assert_parses_ok!(&in_function("return 42;"));
}

#[test]
fn return_without_value() {
    assert_parses_ok!(&in_function("return;"));
}

// ===================
// Control Flow
// ===================

#[test]
fn if_statement_simple() {
    assert_parses_ok!(&in_function("if (condition) { x = 1; }"));
}

#[test]
fn if_statement_with_else() {
    assert_parses_ok!(&in_function("if (a == b) { return a; } else { return b; }"));
}

#[test]
fn if_statement_nested() {
    assert_parses_ok!(&in_function("if (a) { if (b) { c = 1; } else { c = 2; } }"));
}

#[test]
fn if_statement_invalid_condition() {
    assert_parses_err!(&in_function("if { x = 1; }"));
}

// ===================
// Block Statements
// ===================

#[test]
fn simple_block() {
    assert_parses_ok!(&in_function("{ let x = 1; let y = 2; }"));
}

#[test]
fn nested_blocks() {
    assert_parses_ok!(&in_function("{ { let inner = 1; } let outer = 2; }"));
}

#[test]
fn deep_nesting() {
    assert_parses_ok!(&in_function(
        "if (true) { if (false) { if (true) { if (true) { if (true) { let x = 1; } } } } }"
    ));
}

// ===================
// Expression Statements
// ===================

#[test]
fn expression_statement() {
    assert_parses_ok!(&in_function("foo();"));
}

#[test]
fn complex_expression_statement() {
    assert_parses_ok!(&in_function("obj.method().another();"));
}

// ===================
// Loop Statements
// ===================

#[test]
fn simple_loop() {
    assert_parses_ok!(&in_function("loop { let x = 1; }"));
}

#[test]
fn loop_with_break() {
    assert_parses_ok!(&in_function("loop { break; }"));
}

#[test]
fn loop_with_continue() {
    assert_parses_ok!(&in_function("loop { continue; }"));
}

#[test]
fn while_loop_simple() {
    assert_parses_ok!(&in_function("while (x != 10) { x = x + 1; }"));
}

#[test]
fn while_loop_with_break() {
    assert_parses_ok!(&in_function("while (true) { if (done) { break; } }"));
}

#[test]
fn for_loop_simple() {
    assert_parses_ok!(&in_function("for i in range { let x = i; }"));
}

#[test]
fn for_loop_with_continue() {
    assert_parses_ok!(&in_function(
        "for item in items { if (skip) { continue; } process(item); }"
    ));
}

#[test]
fn nested_loops() {
    assert_parses_ok!(&in_function(
        "while (outer) { for inner in items { if (found) { break; } } }"
    ));
}

#[test]
fn loop_in_if() {
    assert_parses_ok!(&in_function(
        "if (condition) { loop { work(); if (done) { break; } } }"
    ));
}

#[test]
fn break_outside_loop() {
    // This should parse successfully - semantic analysis will catch the error
    assert_parses_ok!(&in_function("break;"));
}

#[test]
fn continue_outside_loop() {
    // This should parse successfully - semantic analysis will catch the error
    assert_parses_ok!(&in_function("continue;"));
}
