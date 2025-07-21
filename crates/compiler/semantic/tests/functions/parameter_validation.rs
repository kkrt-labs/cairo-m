//! Tests for function parameter validation
use crate::*;

#[test]
fn test_function_parameters() {
    assert_semantic_parameterized! {
        ok: [
            "fn test(x: felt) -> felt { return x; }",
            "fn test(x: felt, y: felt, z: felt) -> felt { return x + y + z; }",
            "fn test() -> felt { return 42; }",
            "fn test(param: felt) -> felt { let var = param + 1; return var; }",
            "fn test(param: felt) -> felt { { let inner = param * 2; return inner; } }",
            "fn test(param: bool) -> felt { if (param) { return 1; } else { return 0; } }",
            "fn test(param: felt) -> felt { param = param + 1; return param; }",
            // Shadowing is ok
            "fn test(param: felt) -> felt { let param = 42; return param; }",
            "fn test(param: felt) -> felt { { let param = 100; return param; } }",
            "fn helper(x: felt) -> felt { return x * 2; } fn test(param: felt) -> felt { return helper(param); }",
        ],
        err: [
            // Duplicate parameter names
            "fn test(x: felt, x: felt) -> felt { return x; }",
            // Incompatible types
            "fn foo(x: felt){return;} \n fn test(){foo(true); return;}"
        ]
    }
}

#[test]
fn test_unused_parameters() {
    assert_semantic_parameterized! {
        ok: [
            "fn test(a: felt, b: felt, c: felt) -> felt { return a + b + c; }",
        ],
        err: [
            "fn test(unused_param: felt) -> felt { return 42; }",
            "fn test(used: felt, unused1: felt, unused2: felt) -> felt { return used; }",
        ],
        show_unused
    }
}
