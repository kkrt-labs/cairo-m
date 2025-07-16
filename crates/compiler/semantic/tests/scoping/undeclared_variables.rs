//! Tests for undeclared variable detection

use crate::*;

#[test]
fn test_simple_undeclared_variable() {
    assert_semantic_err!(
        r#"
        fn test() {
            let x = undefined_var;
        }
    "#
    );
}

#[test]
fn test_undeclared_in_expression() {
    assert_semantic_err!(
        r#"
        fn test() {
            let x = 5;
            let y = x + undefined_var;
        }
    "#
    );
}

#[test]
fn test_undeclared_in_return() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            return undefined_var;
        }
    "#
    );
}

#[test]
fn test_undeclared_in_function_call() {
    assert_semantic_err!(
        r#"
        fn valid_func(x: felt) -> felt {
            return x;
        }

        fn test() {
            let result = valid_func(undefined_var);
        }
    "#
    );
}

#[test]
fn test_undeclared_function_call() {
    assert_semantic_err!(
        r#"
        fn test() {
            let result = undefined_function(42);
        }
    "#
    );
}

#[test]
fn test_multiple_undeclared_variables() {
    assert_semantic_err!(
        r#"
        fn test() {
            let x = first_undefined;
            let y = second_undefined;
            let z = x + y + third_undefined;
        }
    "#
    );
}

#[test]
fn test_undeclared_in_if_condition() {
    assert_semantic_err!(
        r#"
        fn test() {
            if (undefined_condition) {
                let x = 1;
            }
        }
    "#
    );
}

#[test]
fn test_undeclared_in_assignment() {
    assert_semantic_err!(
        r#"
        fn test() {
            let x = 5;
            x = undefined_var;
        }
    "#
    );
}

#[test]
fn test_declared_variable_ok() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 5;
            let y = x + 10;
            return y;
        }
    "#
    );
}

#[test]
fn test_parameter_usage_ok() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            let variable = param + 1;
            return variable;
        }
    "#
    );
}
