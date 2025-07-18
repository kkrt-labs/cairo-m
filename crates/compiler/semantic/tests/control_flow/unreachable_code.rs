//! Tests for unreachable code detection.
use crate::*;

#[test]
fn test_unreachable_code_detection() {
    assert_semantic_parameterized! {
        ok: [
            // Reachable code after if with single return
            "fn test(x: bool) -> felt { if (x) { return 1; } let reachable = 2; return reachable; }",
            // Reachable code after loop with break
            in_function("loop { break; } let y = 2;"),
            // Reachable code with conditional break
            "fn test(c: bool) { loop { if (c) { break; } let x = 1; } return (); }",
            // Reachable code after while loop
            in_function("while (false) { let x = 1; } let y = 2;"),
        ],
        err: [
            // Code after return
            "fn test() -> felt { return 42; let unreachable = 1; }",
            // Code after return in block
            "fn test() -> felt { { return 42; let unreachable = 1; } }",
            // Code after if/else with returns in all branches
            "fn test(x: bool) -> felt { if (x) { return 1; } else { return 2; } let unreachable = 3; }",
            // Code after break in loop
            in_function("loop { break; let x = 1; }"),
            // Code after continue in loop
            in_function("loop { continue; let x = 1; }"),
            // Code after infinite loop
            in_function("loop { let x = 1; } let y = 2;"),
            // Code in loop after return
            "fn test() { loop { return (); let x = 1; } }",
            // Code after loop that always returns
            "fn test() { loop { return (); } let y = 2; }",
        ]
    }
}
