//! Tests for type checking in loop conditions.
use crate::*;

#[test]
fn test_loop_condition_type_checking() {
    assert_semantic_parameterized! {
        ok: [
            // while with bool
            in_function("while true { break; }"),
            // while with comparison
            in_function("let x: felt = 10; while x == 0 { break; }"),
            // while with logical op
            in_function("let a: bool = true; let b: bool = false; while a && b { break; }"),
        ],
        err: [
            // while with felt
            in_function("let x: felt = 1; while x { break; }"),
            // while with struct
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 1, y: 2 }; while p { break; } return (); }",
            // while with tuple
            in_function("let t: (felt, felt) = (1, 2); while t { break; }"),
            // while with complex non-bool expression
            "struct Config { enabled: bool } fn test() { let config: Config = Config { enabled: true }; while config { break; } return (); }",
            // while with nested felt conditions
            in_function("let a: felt = 1; let b: felt = 0; while a { while b { break; } break; }"),
        ]
    }
}
