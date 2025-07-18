//! Tests for break/continue statement validation.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_break_continue_validation() {
    assert_semantic_parameterized! {
        ok: [
            // break/continue in loop
            in_function("loop { break; }"),
            in_function("loop { if(1==2) { continue; } else {break;} }"),
            // break/continue in while loop
            in_function("while(true) { break; }"),
            in_function("while(true) { if(true) {continue;} }"),
            // in nested loops
            in_function("loop { loop { break; } break; }"),
            in_function("loop { loop { if (true) {continue;} else {break;} } break; }"),
            // in block inside loop
            in_function("loop { { if (true) { break; } continue; } }"),
        ],
        err: [
            // break/continue outside loop
            in_function("break;"),
            in_function("continue;"),
            // in if outside loop
            in_function("if (true) { break; }"),
            in_function("if (true) { continue; }"),
            // in block outside loop
            in_function("{ break; }"),
            // multiple errors
            in_function("break; if (true) { continue; } { break; }"),
            // mix of valid and invalid
            in_function("break; loop { break; } continue; while (true) { if (true) { break; } else { continue; } }"),
        ]
    }
}
