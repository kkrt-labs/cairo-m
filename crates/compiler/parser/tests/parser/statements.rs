use crate::assert_parses_parameterized;
use crate::common::in_function;

#[test]
fn let_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("let x = 5;"),
            in_function("let x: felt = 5;"),
            in_function("let x: u32 = 5;"),
            in_function("let result = a + b * c;"),
            in_function("const PI = 314;"),
        ],
        err: [
            in_function("let x = 5"),
        ]
    }
}

#[test]
fn tuple_destructuring_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("let (x, y) = (1, 2);"),
            in_function("let (x, y): (felt, felt) = (1, 2);"),
            in_function("let (x, y) = get_pair();"),
            in_function("let (sum, diff) = (a + b, a - b);"),
            in_function("let (x, y, z) = (1, 2, 3);"),
            in_function("let (x, y): (felt, felt) = (42, 84);"),
            in_function("let (a, b): (felt, felt) = (1, 2);"),
        ]
    }
}

#[test]
fn assignment_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("x = 5;"),
            in_function("obj.field = value;"),
            in_function("arr[0] = item;"),
        ],
        err: [
            in_function("= 5;"),
        ]
    }
}

#[test]
fn return_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("return 42;"),
            in_function("return;"),
        ]
    }
}

#[test]
fn if_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("if (condition) { x = 1; }"),
            in_function("if (a == b) { return a; } else { return b; }"),
            in_function("if (a) { if (b) { c = 1; } else { c = 2; } }"),
        ],
        err: [
            in_function("if { x = 1; }"),
        ]
    }
}

#[test]
fn block_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("{ let x = 1; let y = 2; }"),
            in_function("{ { let inner = 1; } let outer = 2; }"),
            in_function("if (true) { if (false) { if (true) { if (true) { if (true) { let x = 1; } } } } }"),
        ]
    }
}

#[test]
fn expression_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("foo();"),
            in_function("obj.method().another();"),
        ]
    }
}

#[test]
fn loop_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            in_function("loop { let x = 1; }"),
            in_function("loop { break; }"),
            in_function("loop { continue; }"),
            in_function("while (x != 10) { x = x + 1; }"),
            in_function("while (true) { if (done) { break; } }"),
            in_function("for i in range { let x = i; }"),
            in_function("for item in items { if (skip) { continue; } process(item); }"),
            in_function("while (outer) { for inner in items { if (found) { break; } } }"),
            in_function("if (condition) { loop { work(); if (done) { break; } } }"),
            in_function("break;"),
            in_function("continue;"),
        ]
    }
}
