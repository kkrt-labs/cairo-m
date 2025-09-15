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
            in_function(format!("{};", u64::MAX as u128 + 1).as_str()), // u64::MAX + 1
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
// Cast Operations
// ===================

#[test]
fn cast_expressions_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("x as felt;"),
            in_function("42u32 as felt;"),
            in_function("(x + y) as felt;"),
            in_function("x + 5 as felt;"),  // Cast has lower precedence than +
            in_function("(x * 2u32) as felt;"),
            in_function("arr[0] as felt;"),
            in_function("point.x as felt;"),
            in_function("x as felt as felt;"),  // Multiple casts
        ],
        err: [
            in_function("x as;"),
            in_function("as felt;"),
            in_function("x as 123;"),  // Invalid type
        ]
    }
}

#[test]
fn cast_precedence() {
    assert_parses_parameterized! {
        ok: [
            in_function("x as felt && y;"),
            in_function("x & 0xFF as felt;"),
            in_function("x as felt == 42;"),
        ],
        err: []
    }
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
            in_function("a % b;"),
            in_function("a == b;"),
            in_function("a != b;"),
            in_function("a < b;"),
            in_function("a <= b;"),
            in_function("a > b;"),
            in_function("a >= b;"),
            in_function("a && b;"),
            in_function("a || b;"),
            in_function("a & b;"),
            in_function("a | b;"),
            in_function("a ^ b;"),
        ],
        err: [
            in_function("a +;"),
            in_function("+ b;"),
            in_function("a ==;"),
            in_function("&& b;"),
            in_function("a << b;"),
            in_function("a >> b;"),
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

#[test]
fn bitwise_precedence() {
    // Tests that & has higher precedence than ^ which has higher precedence than |
    // Expected: ((a & b) ^ c) | d
    assert_parses_ok!(&in_function("let z = a & b ^ c | d;"));
}

#[test]
fn bitwise_vs_logical_precedence() {
    // Tests that bitwise ops have higher precedence than logical ops
    // Expected: (a & b) && (c | d)
    assert_parses_ok!(&in_function("let x = a & b && c | d;"));
}

#[test]
fn bitwise_vs_comparison() {
    // Tests that comparison has higher precedence than bitwise
    // Expected: (a < b) & (c > d)
    assert_parses_ok!(&in_function("let y = a < b & c > d;"));
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
// Tuple Indexing
// ===================

#[test]
fn tuple_indexing_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("tt.0;"),
            in_function("my_tuple.1;"),
            in_function("(1, 2, 3).0;"),
            in_function("foo(bar).2;"),
            in_function("((1, 2), (3, 4)).0.1;"),
            in_function("get_tuple().0;"),
            in_function("tuple_ptr.3;"),
        ],
        err: [
            in_function("tt.0u32;"),  // suffix not allowed
            in_function("tt.0felt;"), // suffix not allowed
            in_function("tt.;"),      // missing index
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
