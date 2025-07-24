//! Tests for binary expression validation.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_arithmetic_operator_types() {
    assert_semantic_parameterized! {
        ok: [
            // felt
            in_function("let a: felt = 10; let b: felt = 20; let sum = a + b;"),
            in_function("let a: felt = 10; let b: felt = 20; let diff = a - b;"),
            in_function("let a: felt = 10; let b: felt = 20; let prod = a * b;"),
            in_function("let a: felt = 10; let b: felt = 20; let quot = a / b;"),

            // u32
            in_function("let a: u32 = 1; let b: u32 = 2; let sum = a + b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let diff = a - b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let prod = a * b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let quot = a / b;"),
            ],
            err: [
            // bool
            in_function("let a: bool = true; let b: bool = false; let sum = a + b;"),
            in_function("let a: bool = true; let b: bool = false; let diff = a - b;"),
            in_function("let a: bool = true; let b: bool = false; let prod = a * b;"),
            in_function("let a: bool = true; let b: bool = false; let quot = a / b;"),

            // Incompatible base types
            in_function("let x: felt = 42; let y: u32 = 100; let result = x + y;"),

            // Custom type
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let p1 = Point { x: 10, y: 20 }; let p2 = Point { x: 30, y: 40 }; let p3 = p1 + p2;")),
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let p1 = Point { x: 10, y: 20 }; let p4 = p1 * 2;")),
        ]
    }
}

#[test]
fn test_comparison_operator_types() {
    assert_semantic_parameterized! {
        ok: [
            // felt
            in_function("let a: felt = 1; let b: felt = 2; let c = a == b;"),
            in_function("let a: felt = 1; let b: felt = 2; let c = a != b;"),

            // u32
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a < b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a > b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a <= b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a >= b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a == b;"),
            in_function("let a: u32 = 1; let b: u32 = 2; let c = a != b;"),

            // bool
            in_function("let a: bool = true; let b: bool = false; let c = a == b;"),
            in_function("let a: bool = true; let b: bool = false; let c = a != b;"),
        ],
        err: [
            // felt
            in_function("let a: felt = 1; let b: u32 = 2; let c = a > b;"),
            in_function("let a: felt = 1; let b: u32 = 2; let c = a < b;"),
            in_function("let a: felt = 1; let b: u32 = 2; let c = a <= b;"),
            in_function("let a: felt = 1; let b: u32 = 2; let c = a >= b;"),

            // bool
            in_function("let a: bool = true; let b: bool = false; let c = a > b;"),
            in_function("let a: bool = true; let b: bool = false; let c = a < b;"),
            in_function("let a: bool = true; let b: bool = false; let c = a <= b;"),
            in_function("let a: bool = true; let b: bool = false; let c = a >= b;"),
        ]
    }
}

#[test]
fn test_logical_operator_types() {
    assert_semantic_parameterized! {
        ok: [
            // bool
            in_function("let c: bool = true; let d: bool = false; let and_correct = c && d;  let or_correct = c || d;"),
        ],
        err: [
            // felt
            in_function("let x: felt = 42; let y: felt = 100; let and1 = x && y;"),
            in_function("let x: felt = 42; let y: felt = 100; let or1 = x || y;"),

            // // u32
            in_function("let a: u32 = 1; let b: u32 = 0; let and2 = a && b;"),
            in_function("let a: u32 = 1; let b: u32 = 0; let or2 = a || b;"),

            // // Custom type
            "struct Point { x: felt, y: felt } fn test() { let p1 = Point { x: 10, y: 20 }; let p2 = Point { x: 30, y: 40 }; let p1_and_p2 = p1 && p2; return;}",
            "struct Point { x: felt, y: felt } fn test() { let p1 = Point { x: 10, y: 20 }; let p2 = p1; let p1_or_p2 = p1 || p2; return;}",
        ]
    }
}

#[test]
fn test_comparison_operator_type_errors() {
    assert_semantic_parameterized! {
        err: [
            // From: comparison_type_mismatch.cm
            "fn test() { let x: felt = 42; let y: u32 = 100; let c1 = x == y; }",
            "fn test() { let x: felt = 42; let y: u32 = 100; let c2 = x != y; }",
            "fn test() { let x: felt = 42; let y: u32 = 100; let c3 = x < y; }",
            "fn test() { let x: felt = 42; let y: u32 = 100; let c4 = x > y; }",
            "fn test() { let x: felt = 42; let y: u32 = 100; let c5 = x <= y; }",
            "fn test() { let x: felt = 42; let y: u32 = 100; let c6 = x >= y; }",
            "fn test() { let x: felt = 42; let b: bool = true; let c7 = x == b; }",
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20 }; let x: felt = 42; let c8 = p == x; }",
            // From: struct_arithmetic_error.cm (comparison part)
            "struct Point { x: felt, y: felt } fn test() { let p1 = Point { x: 10, y: 20 }; let p2 = Point { x: 30, y: 40 }; let is_greater = p1 > p2; }",
        ]
    }
}
