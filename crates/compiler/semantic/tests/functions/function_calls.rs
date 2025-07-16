//! Tests for function call validation

use crate::*;

#[test]
fn test_valid_function_call() {
    assert_semantic_ok!(
        r#"
        fn helper(x: felt) -> felt {
            return x + 1;
        }

        fn test() -> felt {
            return helper(42);
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
fn test_function_call_with_undeclared_argument() {
    assert_semantic_err!(
        r#"
        fn helper(x: felt) -> felt {
            return x;
        }

        fn test() {
            let result = helper(undefined_var);
        }
    "#
    );
}

#[test]
fn test_multiple_function_calls() {
    assert_semantic_ok!(
        r#"
        fn add(a: felt, b: felt) -> felt {
            return a + b;
        }

        fn multiply(a: felt, b: felt) -> felt {
            return a * b;
        }

        fn test() -> felt {
            let sum = add(1, 2);
            let product = multiply(sum, 3);
            return product;
        }
    "#
    );
}

#[test]
fn test_nested_function_calls() {
    assert_semantic_ok!(
        r#"
        fn inner(x: felt) -> felt {
            return x * 2;
        }

        fn outer(x: felt) -> felt {
            return inner(x) + 1;
        }

        fn test() -> felt {
            return outer(inner(5));
        }
    "#
    );
}

#[test]
fn test_function_call_in_expression() {
    assert_semantic_ok!(
        r#"
        fn get_value() -> felt {
            return 42;
        }

        fn test() -> felt {
            let result = get_value() + 10;
            return result;
        }
    "#
    );
}

#[test]
fn test_function_call_as_condition() {
    assert_semantic_ok!(
        r#"
        fn is_null(x: felt) -> felt {
            return x == 0;
        }

        fn test(x: felt) -> felt {
            if (is_null(x)) {
                return 1;
            } else {
                return 0;
            }
        }
    "#
    );
}

#[test]
fn test_recursive_function_call() {
    assert_semantic_ok!(
        r#"
        fn factorial(n: felt) -> felt {
            if (n == 1) {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }

        fn test() -> felt {
            return factorial(5);
        }
    "#
    );
}
