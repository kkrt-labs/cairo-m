//! Tests for function call validation
use crate::*;

#[test]
fn test_function_calls() {
    assert_semantic_parameterized! {
        ok: [
            // Valid call
            "fn helper(x: felt) -> felt { return x + 1; } fn test() -> felt { return helper(42); }",
            // Multiple calls
            "fn add(a: felt, b: felt) -> felt { return a + b; } fn multiply(a: felt, b: felt) -> felt { return a * b; } fn test() -> felt { let sum = add(1, 2); let product = multiply(sum, 3); return product; }",
            // Nested calls
            "fn inner(x: felt) -> felt { return x * 2; } fn outer(x: felt) -> felt { return inner(x) + 1; } fn test() -> felt { return outer(inner(5)); }",
            // Call in expression
            "fn get_value() -> felt { return 42; } fn test() -> felt { let result = get_value() + 10; return result; }",
            // Call as condition
            "fn is_null(x: felt) -> bool { return x == 0; } fn test(x: felt) -> felt { if is_null(x) { return 1; } else { return 0; } }",
            // Recursive call
            "fn factorial(n: felt) -> felt { if n == 1 { return 1; } else { return n * factorial(n - 1); } } fn test() -> felt { return factorial(5); }",
        ],
        err: [
            // Undeclared function
            in_function("let result = undefined_function(42);"),
            // Undeclared argument
            "fn helper(x: felt) -> felt { return x; } fn test() { let result = helper(undefined_var); return (); }",
        ]
    }
}
