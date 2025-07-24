//! Tests for let statement validation
use crate::*;

#[test]
fn test_let_statements() {
    assert_semantic_parameterized! {
        ok: [
            in_function("let x = 42;"),
            in_function("let x = 10; let y = x + 20;"),
            in_function("let a = 1; let b = 2; let c = a + b;"),
            "fn helper() -> felt { return 42; } fn test() { let x = helper(); return (); }",
            in_function("let outer = 1; { let inner = outer + 1; }"),
            in_function("let x: felt = 42;"),
            in_function("let a = 10; let b = 20; let result = (a + b) * 2 - 5;"),
            // shadowing is ok
            in_function("let x = 1; let x = 2;"),
        ],
        err: [
            in_function("let x = undefined_var;"),
        ]
    }
}

#[test]
fn test_let_statements_with_type_annotation() {
    assert_semantic_parameterized! {
        ok: [
            in_function("let x: felt = 42;"),
            in_function("let x: u32 = 42;"),
            in_function("let x: u32 = 42u32;"),
            in_function("let x: felt = 42felt;"),
        ],
        err: [
            in_function("let x: felt = 32u32;"),
            in_function("let x: u32 = 32felt;"),
        ]
    }
}
