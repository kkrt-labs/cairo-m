//! Tests for expression statement validation

use crate::*;

#[test]
fn test_function_call_statement() {
    assert_semantic_ok!(&with_functions(
        "func side_effect() { return (); }",
        &in_function(
            "
            side_effect(); // Function call as statement
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_undeclared() {
    assert_semantic_err!(&in_function(
        "
        undefined_function(); // Error: undeclared function
    "
    ));
}

#[test]
fn test_function_call_statement_with_args() {
    assert_semantic_ok!(&with_functions(
        "func process(x: felt) { return (); }",
        &in_function(
            "
            process(42); // Function call with arguments
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_with_undeclared_args() {
    assert_semantic_err!(&with_functions(
        "func process(x: felt) { return (); }",
        &in_function(
            "
            process(undefined_var); // Error: undeclared variable in argument
        "
        )
    ));
}

#[test]
fn test_function_call_statement_with_variable_args() {
    assert_semantic_ok!(&with_functions(
        "func process(x: felt, y: felt) { return (); }",
        &in_function(
            "
            let a = 10;
            let b = 20;
            process(a, b); // Use variables as arguments
            return ();
        "
        )
    ));
}

#[test]
fn test_nested_function_call_statements() {
    assert_semantic_ok!(&with_functions(
        r#"
        func helper1() { return (); }
        func helper2() { return (); }
        func helper3() { return (); }
        "#,
        &in_function(
            "
            helper1();
            helper2();
            helper3();
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_in_block() {
    assert_semantic_ok!(&with_functions(
        "func helper() { return (); }",
        &in_function(
            "
            {
                helper(); // Function call in nested block
            }
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_in_if() {
    assert_semantic_ok!(&with_functions(
        "func helper() { return (); }",
        &in_function(
            "
            if (true) {
                helper(); // Function call in if branch
            } else {
                helper(); // Function call in else branch
            }
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_with_complex_args() {
    assert_semantic_ok!(&with_functions(
        "func process(x: felt) { return (); }",
        &in_function(
            "
            let a = 10;
            let b = 5;
            process(a + b * 2); // Complex expression as argument
            return ();
        "
        )
    ));
}

#[test]
fn test_multiple_function_call_statements() {
    assert_semantic_ok!(&with_functions(
        r#"
        func step1() { return (); }
        func step2(x: felt) { return (); }
        func step3() { return (); }
        "#,
        &in_function(
            "
            step1();
            step2(42);
            step3();
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_return_value_ignored() {
    // Function calls that return values can be used as statements (return value ignored)
    assert_semantic_ok!(&with_functions(
        "func get_value() -> felt { return 42; }",
        &in_function(
            "
            get_value(); // Return value ignored
            return ();
        "
        )
    ));
}

#[test]
fn test_function_call_statement_with_side_effects() {
    // Test that function calls in statements properly validate their arguments
    assert_semantic_ok!(&with_functions(
        "func modify(x: felt) { return (); }",
        &in_function(
            "
            let value = 100;
            modify(value); // value is used here
            return ();
        "
        )
    ));
}

#[test]
fn test_recursive_function_call_statement() {
    assert_semantic_ok!(
        r#"
        func recursive_helper(n: felt) {
            if (n == 0) {
                recursive_helper(n - 1); // Recursive call as statement
            }
            return ();
        }

        func test() {
            recursive_helper(5);
            return ();
        }
    "#
    );
}

#[test]
fn test_chained_function_calls_as_statements() {
    // While we can't chain calls directly, we can have sequential calls
    assert_semantic_ok!(&with_functions(
        r#"
        func first() -> felt { return 1; }
        func second(x: felt) { return (); }
        "#,
        &in_function(
            "
            let result = first();
            second(result);
            return ();
        "
        )
    ));
}
