//! Tests for classic C-style `for` loops.
use crate::*;

#[test]
fn test_for_loops() {
    assert_semantic_parameterized! {
        ok: [
            // Basic for with break
            in_function("for (let i: u32 = 0; i < 3; i = i + 1) { if i == 2 { break; } }"),
            // for with continue
            in_function("for (let i: u32 = 0; i < 2; i = i + 1) { if i == 0 { continue; } }"),
            // Shadow outer variable & ensure outer still visible after
            in_function("let x: u32 = 5; for (let x: u32 = 0; x < 1; x = x + 1) { let y = x; } let z = x;"),
            // Nested for loops
            in_function("for (let i: u32 = 0; i < 2; i = i + 1) { for (let j: u32 = 0; j < 2; j = j + 1) { if i == j { continue; } } }"),
        ],
        err: [
            // Variable declared in init is not visible after the loop
            in_function("for (let i: u32 = 0; i < 3; i = i + 1) { } let y = i;"),
            // Condition must be bool
            in_function("let x: u32 = 1; for (let i: u32 = 0; x; i = i + 1) { break; }"),
            // Invalid assignment target in step
            in_function("for (let i: u32 = 0; i < 1; 42 = i) { }")
        ]
    }
}
