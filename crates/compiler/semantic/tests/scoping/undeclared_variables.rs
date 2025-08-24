//! Tests for undeclared variable detection

use crate::*;

// Parameterized test for all undeclared variable error cases
#[test]
fn test_undeclared_variables_parameterized() {
    assert_semantic_parameterized! {
        ok: [
            // Declared variable usage
            r#"
            fn test() -> felt {
                let x = 5;
                let y = x + 10;
                return y;
            }
            "#,

            // Parameter usage
            r#"
            fn test(param: felt) -> felt {
                let variable = param + 1;
                return variable;
            }
            "#,
        ],
        err: [
            // Simple undeclared variable
            in_function("let x = undefined_var;"),

            // Undeclared.to_owned() in expression
            in_function("let x = 5; let y = x + undefined_var;"),

            // Undeclared in return
            "fn test() -> felt { return undefined_var; }",

            // Undeclared in function call
            r#"
            fn valid_func(x: felt) -> felt { return x; }
            fn test() { let result = valid_func(undefined_var); }"
            "#,

            // Undeclared function call
            "fn test() { let result = undefined_function(42); }",

            // Multiple undeclared variables
            in_function(
                "let x = first_undefined; let y = second_undefined; let z = x + y + third_undefined;",
            ),

            // Undeclared in if condition
            in_function("if undefined_condition { let x = 1; }"),

            // Undeclared in assignment
            in_function("let x = 5; x = undefined_var;"),
        ]
    }
}
