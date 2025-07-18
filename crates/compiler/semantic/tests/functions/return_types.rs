//! Tests for function return type validation
use crate::*;

#[test]
fn test_return_type_validation() {
    assert_semantic_parameterized! {
        ok: [
            "fn test() -> felt { return 42; }",
            "fn test() { return (); }",
            "fn test() -> () { return (); }",
            "fn test() -> felt { let x = 42; return x; }",
            "fn test() -> felt { let x = 10; let y = 20; return x + y; }",
            "fn helper() -> felt { return 42; } fn test() -> felt { return helper(); }",
            "fn test(x: bool) -> felt { if (x) { return 1; } else { return 0; } }",
            "fn test() -> felt { { { return 42; } } }",
            "fn test(param: felt) -> felt { return param; }",
            "fn test(x: bool) -> felt { if (x) { return 0; } return 1; }",
            "fn test(x: felt, y: felt) -> felt { if (x > 0) { if (y > 0) { return 1; } else { return 2; } } else { return 3; } }",
        ],
        err: [
            // Wrong return type
            "fn test() -> felt { return (); }",
            "fn test() { return 42; }",
            "fn test() -> () { return 42; }",
            // Return type mismatch
            #[allow(clippy::literal_string_with_formatting_args)]
            "struct Point {x:felt} fn test() -> felt { return Point { x: 1 }; }",
            // Undeclared variable in return
            "fn test() -> felt { return undefined_var; }",
        ]
    }
}
