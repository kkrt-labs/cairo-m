//! Tests for nested scopes and variable visibility.
use crate::*;

#[test]
fn test_nested_scopes_and_visibility() {
    assert_semantic_parameterized! {
        ok: [
            // Deeply nested scopes access
            "fn test() -> felt { let l1 = 1; { let l2 = l1 + 1; { let l3 = l2 + 1; { let l4 = l3 + 1; return l4; } } } }",
            // Nested if scopes
            "fn test(x: felt) -> felt { if (x == 0) { let p = x; if (p == 10) { let l = p * 2; return l; } else { return p; } } else { return 0; } }",
            // Nested scope shadowing
            "fn test() -> felt { let x = 1; { let x = 2; { let x = 3; return x; } } }",
            // Nested function calls
            "fn helper(x: felt) -> felt { return x * 2; } fn test() -> felt { let o = 5; { let m = helper(o); { let i = helper(m); return i; } } }",
            // Multiple nested branches
            "fn test(c: felt) -> felt { let b = 10; if (c == 0) { let b1 = b + 1; { let n1 = b1 * 2; return n1; } } else { let b2 = b - 1; { let n2 = b2 * 3; return n2; } } }",
            // Basic visibility
            "fn test() -> felt { let outer = 1; { let inner = outer + 1; return inner; } }",
            // Parameter visibility
            "fn test(p: felt) -> felt { let v = p + 1; return v; }",
            "fn test(p: felt) -> felt { { let i = p + 1; { let d = p + i; return d; } } }",
        ],
        err: [
            // Inner not accessible from outer
            in_function("let outer = 1; { let middle = 2; { let inner = 3; } let bad = inner; }"),
            // Complex scope interaction
            in_function("let a = 1; { let b = a + 1; { let c = b + 1; } let bad1 = c; } let bad2 = b;"),
            // Assignment to variable out of scope
            in_function("let x = 1; { let y = 2; x = y + 1; } y = 3;"),
            // Sibling scopes not visible to each other
            in_function("{ let first = 1; } { let second = first; }"),
            // Variable in if-scope not visible outside
            in_function("if (true) { let if_var = 42; } let bad = if_var;"),
        ]
    }
}
