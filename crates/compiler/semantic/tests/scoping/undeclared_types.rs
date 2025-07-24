//! Tests for undeclared variable detection

use crate::*;

// Parameterized test for all undeclared variable error cases
#[test]
fn test_undeclared_variables_parameterized() {
    assert_semantic_parameterized! {
        ok: [
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use module::MyType; fn test() { let x: MyType = MyType { field: 5 }; return; }"),
                    ("module.cm", "struct MyType { field: felt }"),
                ]
            ),
            "struct Test { field: felt } fn test(x: Test) -> felt { return x.field; }"
        ],
        err: [
            // Undeclared type in variable declaration
            in_function("let x: MyType = 5;"),

            // Undeclared type in function parameter
            "fn test(x: UndefinedType) -> felt { return 42; }",

            // Undeclared type in function return
            "fn test() -> UndefinedReturnType { return 42; }",

            // Undeclared type in struct field
            "struct Test { field: NonExistentType }",
        ]
    }
}
