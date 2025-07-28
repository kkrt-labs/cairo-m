//! Tests for variable scoping within loops.
use crate::*;

#[test]
fn test_loop_scoping() {
    assert_semantic_parameterized! {
        ok: [
            // Variables in loop can shadow outer variables
            in_function("let x = 1; loop { let x = 2; if x == 2 { break; } } let y = x;"),
        ],
        err: [
            // Variables declared in loop body should not be visible outside
            in_function("loop { let x = 42; break; } let y = x;"),
            // Each loop creates its own scope
            in_function("loop { let outer = 1; loop { let inner = 2; let x = outer; break; } let y = inner; break; }"),
            // While loop body creates new scope
            in_function("let condition = true; while (condition) { let loop_var = 42; break; } let x = loop_var;"),
            // Blocks inside loops create additional scopes
            in_function("loop { let loop_var = 1; { let block_var = 2; let x = loop_var; } let y = block_var; break; }"),
        ]
    }
}
