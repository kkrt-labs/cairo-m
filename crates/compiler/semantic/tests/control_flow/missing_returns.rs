//! Tests for missing return statement detection in all control flow paths.
use crate::*;

#[test]
fn test_return_path_analysis() {
    assert_semantic_parameterized! {
        ok: [
            // Simple valid return
            "fn test() -> felt { let x = 42; return x; }",
            // All paths return in if/else
            "fn test(x: bool) -> felt { if x { return 1; } else { return 2; } }",
            // Early return
            "fn test(x: bool) -> felt { if x { return 0; } return 2; }",
            // Nested control flow, all paths return
            "fn test(x: bool, y: bool) -> felt { if x { if y { return 1; } else { return 2; } } else { return 3; } }",
            // Return in nested block
            "fn test() -> felt { { { return 42; } } }",
            // Unit return type with explicit return
            in_function("let x = 42;"),
        ],
        err: [
            // Simple missing return
            "fn test() -> felt { let x = 42; }",
            // Missing return in one path of if
            "fn test(x: bool) -> felt { if x { return 1; } }",
            // Missing return in else branch
            "fn test(x: bool) -> felt { if x { return 1; } else { let y = 1; } }",
            // Missing return in nested if
            "fn test(x: bool, y: bool) -> felt { if x { if y { return 1; } } else { return 3; } }",
            // Implicit unit return is an error
            "fn test() { let x = 42; }",
        ]
    }
}
