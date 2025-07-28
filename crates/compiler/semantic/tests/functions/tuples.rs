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
            in_function("let (x, y) = 42;"), // Error: Cannot destructure non-tuple
            in_function("let (x, y) = (1, 2, 3);"), // Error: Pattern has 2 elements but value has 3
            in_function("let (x, y): felt = (1, 2);"), // Error: Expected felt, found tuple
            in_function("let (x, x) = (1, 2);"), // Error: Duplicate pattern identifier
            ]
    }
}

#[test]
#[ignore = "TODO: Fix this test"]
fn test_tuple_destructuring_unused_variable() {
    assert_semantic_parameterized! {
        err: [
            // y is unused
            in_function("let (x, y) = (1, 2); let z = x + 1;"),
        ]
    }
}

#[test]
fn test_tuple_indexing() {
    assert_semantic_parameterized! {
        ok: [
            // Basic tuple indexing
            in_function("let tt = (1, 2, 3); let x = tt.0;"),
            in_function("let tt = (1, 2, 3); let y = tt.1; let z = tt.2;"),

            // Tuple indexing with expressions
            in_function("let x = (10, 20, 30).0;"),
            in_function("let sum = (10, 20).0 + (30, 40).1;"),

            // Nested tuple indexing
            in_function("let nested = ((1, 2), (3, 4)); let x = nested.0.1;"),
            in_function("let nested = ((1, 2), (3, 4)); let y = nested.1.0;"),

            // Function returning tuple
            "fn get_tuple() -> (felt, felt) { return (10, 20); } fn test() -> felt { return get_tuple().0; }",
            "fn get_tuple() -> (felt, felt) { return (10, 20); } fn test() -> felt { let x = get_tuple().1; return x; }",
        ],
        err: [
            // Out of bounds access
            in_function("let tt = (1, 2); let x = tt.2;"),
            in_function("let tt = (1, 2, 3); let x = tt.3;"),

            // Indexing non-tuple
            in_function("let x = 42; let y = x.0;"),

            // Type mismatch
            in_function("let tt: (felt, felt) = (1, 2); let x: (felt, felt) = tt.0;"),
        ]
    }
}
