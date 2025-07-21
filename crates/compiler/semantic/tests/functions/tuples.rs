//! Tests for tuple destructuring in let statements and function returns.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_tuple_destructuring() {
    assert_semantic_parameterized! {
        ok: [
            "fn test() -> felt { let (x, y) = (10, 20); return x + y; }",
            in_function("let (a, b): (felt, felt) = (1, 2); let (c, d): (felt, felt) = (3, 4);"),
            "fn returns_tuple() -> (felt, felt) { return (100, 200); } fn test() -> felt { let (a, b) = returns_tuple(); return a + b; }",
            in_function("let (x, y) = (1, 2); { let (x, y) = (10, 20); let sum = x + y; } let sum = x + y;"),
        ],
        err: [
            "fn test() { let (x, y) = 42; }", // Error: Cannot destructure non-tuple
            "fn test() { let (x, y) = (1, 2, 3); }", // Error: Pattern has 2 elements but value has 3
            "fn test() { let (x, y): felt = (1, 2); }", // Error: Expected felt, found tuple
        ]
    }
}
