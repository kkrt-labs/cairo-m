//! Tests for duplicate definition detection and shadowing.
use crate::*;

#[test]
fn test_duplicate_definitions_and_shadowing() {
    assert_semantic_parameterized! {
        ok: [
            // Shadowing in same scope is allowed
            "fn test() -> felt { let var = 10; let another = 20; let var = 30; return var; }",
            // Local variables can shadow parameters
            "fn test(param: felt) -> felt { let param = 42; return param; }",
            // Shadowing in nested scopes
            "fn test() -> felt { let x = 1; { let x = 2; } return x; }",
            // Multiple shadowing
            "fn test() -> felt { let x = 1; let y = 2; let x = 3; let y = 4; return x + y; }",

            // Shadowing with different types
            "fn test() -> felt { let x = 1; let y = 2; let z = x+y; let x = 2u32; return z; }",

        ],
        err: [
            // Duplicate parameters
            "fn test(param: felt, param: felt) -> felt { return param; }",

            // Duplicate function names
            "fn duplicate_func() {} fn duplicate_func() {}",

            // Duplicate imports
            "use std::math; use bar::math;",

            // Duplicate consts
            "const duplicate_const = 1; const duplicate_const = 2;",

            // Duplicate structs
            "struct foo {x: felt} struct foo {x: felt}",

            // duplicate combination of top-level items
            "fn foo() {return;} const foo = 1;",
            "struct foo {} fn foo(){return;}",
        ]
    }
}
