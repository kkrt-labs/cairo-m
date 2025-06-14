use crate::common::in_function;
use crate::{assert_parses_err, assert_parses_ok};

// ===================
// Literals
// ===================

#[test]
fn integer_literal() {
    assert_parses_ok!(&in_function("42;"));
}

#[test]
fn large_number() {
    assert_parses_err!(&in_function("4294967295;")); // Max u32
}

#[test]
fn invalid_number_format() {
    assert_parses_err!(&in_function("0xGG;"));
}

#[test]
fn number_overflow() {
    assert_parses_err!(&in_function("0x80000000;"));
}

// ===================
// Identifiers
// ===================

#[test]
fn simple_identifier() {
    assert_parses_ok!(&in_function("my_var;"));
}

#[test]
fn long_identifier() {
    assert_parses_ok!(&in_function(
        "very_long_variable_name_that_tests_identifier_parsing;"
    ));
}

// ===================
// Binary Operations
// ===================

#[test]
fn addition() {
    assert_parses_ok!(&in_function("a + b;"));
}

#[test]
fn multiplication_precedence() {
    assert_parses_ok!(&in_function("a + b * c;"));
}

#[test]
fn comparison_and_logical() {
    assert_parses_ok!(&in_function("a == b && c != d;"));
}

#[test]
fn complex_precedence() {
    assert_parses_ok!(&in_function("a + b * c == d && e || f;"));
}

#[test]
fn precedence_chain() {
    assert_parses_ok!(&in_function("a || b && c == d + e * f / g - h;"));
}

#[test]
fn invalid_binary_op() {
    assert_parses_err!(&in_function("a +;"));
}

#[test]
fn invalid_gt_sign() {
    assert_parses_err!(&in_function("a > b;"));
}

#[test]
fn invalid_lt_sign() {
    assert_parses_err!(&in_function("a < b;"));
}

#[test]
fn invald_geq_sign() {
    assert_parses_err!(&in_function("a >= b;"));
}

#[test]
fn invalid_leq_sign() {
    assert_parses_err!(&in_function("a <= b;"));
}

// ===================
// Function Calls
// ===================

#[test]
fn simple_function_call() {
    assert_parses_ok!(&in_function("foo();"));
}

#[test]
fn function_call_with_args() {
    assert_parses_ok!(&in_function("add(a, b, c);"));
}

#[test]
fn chained_calls() {
    assert_parses_ok!(&in_function("obj.method().another();"));
}

#[test]
fn trailing_comma_function_call() {
    assert_parses_ok!(&in_function("foo(a, b, c,);"));
}

#[test]
fn unclosed_paren() {
    assert_parses_err!(&in_function("foo(a, b;"));
}

// ===================
// Member Access
// ===================

#[test]
fn simple_member_access() {
    assert_parses_ok!(&in_function("obj.field;"));
}

#[test]
fn nested_member_access() {
    assert_parses_ok!(&in_function("obj.inner.field;"));
}

#[test]
fn chained_operations() {
    assert_parses_ok!(&in_function(
        "obj.method1().field.method2()[0].final_field;"
    ));
}

// ===================
// Array Indexing
// ===================

#[test]
fn array_index() {
    assert_parses_ok!(&in_function("arr[0];"));
}

#[test]
fn nested_index() {
    assert_parses_ok!(&in_function("matrix[i][j];"));
}

// ===================
// Struct Literals
// ===================

#[test]
fn simple_struct_literal() {
    assert_parses_ok!(&in_function("Point { x: 1, y: 2 };"));
}

#[test]
fn nested_struct_literal() {
    assert_parses_ok!(&in_function(
        "Rectangle { top_left: Point { x: 0, y: 0 }, width: 10 };"
    ));
}

#[test]
fn empty_struct_literal() {
    assert_parses_ok!(&in_function("Unit {};"));
}

#[test]
fn trailing_comma_struct_literal() {
    assert_parses_ok!(&in_function("Point { x: 1, y: 2, };"));
}

// ===================
// Tuples
// ===================

#[test]
fn simple_tuple() {
    assert_parses_ok!(&in_function("(1, 2, 3);"));
}

#[test]
fn nested_tuple() {
    assert_parses_ok!(&in_function("((1, 2), (3, 4));"));
}

#[test]
fn single_element_tuple() {
    assert_parses_ok!(&in_function("(single_element,);"));
}

// ===================
// Parenthesized Expressions
// ===================

#[test]
fn parenthesized_expr() {
    assert_parses_ok!(&in_function("(a + b);"));
}

#[test]
fn deeply_nested() {
    assert_parses_ok!(&in_function("((((((a + b) * c) - d) / e) == f) && g);"));
}

// ===================
// Complex Expressions
// ===================

#[test]
fn complex_expression_precedence() {
    assert_parses_ok!(&in_function(
        "result = a.field[0].method(b + c * d, e && f || g).value;"
    ));
}
