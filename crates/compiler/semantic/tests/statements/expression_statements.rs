//! Tests for expression statement validation
use crate::*;

#[test]
fn test_expression_statements() {
    assert_semantic_parameterized! {
        ok: [
            "fn side_effect() {return;} fn test() { side_effect(); return; }",
            "fn process(x: felt) {return;} fn test() { process(42); return; }",
            "fn process(x: felt, y: felt) {return;} fn test() { let a = 1; let b = 2; process(a, b); return; }",
            "fn helper() {return;} fn test() { { helper(); } return (); }",
            "fn helper() {return;} fn test() { if true { helper(); } else { helper(); } return (); }",
            "fn process(x: felt) {return;} fn test() { let a = 1; process(a + 2); return (); }",
            // Return value ignored is OK
            "fn get_value() -> felt { return 42; } fn test() { get_value(); return (); }",
            // Recursive call as statement
            "fn recursive(n: u32) { if n > 0 { recursive(n - 1); } return (); } fn test() { recursive(5); return (); }",
        ],
        err: [
            in_function("undefined_function();"),
            "fn process(x: felt) {return;} fn test() { process(undefined_var); return (); }",
        ]
    }
}
