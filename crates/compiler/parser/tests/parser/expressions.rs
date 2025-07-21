use crate::common::in_function;
use crate::{assert_parses_ok, assert_parses_parameterized};

// ===================
// Literals
// ===================

#[test]
fn number_literals_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("0;"),
            in_function("1;"),
            in_function("42;"),
            in_function("1234567890;"),
            in_function("4294967295;"), // u32::MAX
            in_function("0x0;"),
            in_function("0xFF;"),
            in_function("0xABCDEF;"),
        ],
        err: [
            in_function("4294967296;"), // u32::MAX + 1
            in_function("0xGG;"),
            in_function("0x;"),
            in_function("123abc;"),
            in_function("100"),
        ]
    }
}

// ===================
// Identifiers
// ===================

#[test]
fn identifier_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("my_var;"),
            in_function("very_long_variable_name_that_tests_identifier_parsing;"),
        ],
        err: [
            in_function("my_var"),
        ]
    }
}

// ===================
// Unary Operations
// ===================

#[test]
fn unary_neg() {
    assert_parses_ok!(&in_function("-a;"));
}

#[test]
fn unary_not() {
    assert_parses_ok!(&in_function("!a;"));
}

// ===================
// Binary Operations
// ===================

// Parameterized test for various binary operations
#[test]
fn binary_operations_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("a + b;"),
            in_function("a - b;"),
            in_function("a * b;"),
            in_function("a / b;"),
            in_function("a == b;"),
            in_function("a != b;"),
            in_function("a < b;"),
            in_function("a <= b;"),
            in_function("a > b;"),
            in_function("a >= b;"),
            in_function("a && b;"),
            in_function("a || b;"),
        ],
        err: [
            in_function("a +;"),
            in_function("+ b;"),
            in_function("a ==;"),
            in_function("&& b;"),
            in_function("a | b;"),
            in_function("a & b;"),
            in_function("a ^ b;"),
            in_function("a << b;"),
            in_function("a >> b;"),
            in_function("a % b;"),
            in_function("a ** b;"),
        ]
    }
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

// ===================
// Function Calls
// ===================

#[test]
fn function_call_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("foo();"),
            in_function("add(a, b, c);"),
            in_function("foo(a, b, c,);"),
            in_function("obj.method().another();"),
            ],
            err: [
            in_function("foo(a, b;"),
            in_function("add(a: felt, b: u32, c: bool);"),
        ]
    }
}

// ===================
// Member Access
// ===================

#[test]
fn member_access_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("obj.field;"),
            in_function("obj.inner.field;"),
            in_function("obj.method().field.method2()[0].final_field;"),
        ]
    }
}

// ===================
// Array Indexing
// ===================

#[test]
fn array_indexing_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("arr[0];"),
            in_function("matrix[i][j];"),
        ]
    }
}

// ===================
// Struct Literals
// ===================

#[test]
fn struct_literal_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("Point { x: 1, y: 2 };"),
            in_function("Point { x: 1, y: 2, };"),
            in_function("Rectangle { top_left: Point { x: 0, y: 0 }, width: 10 };"),
            in_function("Unit {};"),
        ],
        err: [
            in_function("Point { x: 1, y: 2, z };"),
            in_function("Rectangle { top_left: Point { x: 0, y: 0 }, width: };"),
        ]
    }
}

// ===================
// Tuples
// ===================

#[test]
fn tuple_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("(1, 2, 3);"),
            in_function("((1, 2), (3, 4));"),
            in_function("(single_element,);"),
        ],
        err: [
            in_function("(single_element,"),
        ]
    }
}

// ===================
// Parenthesized Expressions
// ===================

#[test]
fn parenthesized_expr_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("(a + b);"),
            in_function("((((((a + b) * c) - d) / e) == f) && g);"),
        ]
    }
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
