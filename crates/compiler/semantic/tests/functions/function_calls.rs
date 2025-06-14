//! Tests for function call validation

use crate::*;

#[test]
fn test_valid_function_call() {
    assert_semantic_ok!(
        r#"
        func helper(x: felt) -> felt {
            return x + 1;
        }

        func test() -> felt {
            return helper(42);
        }
    "#
    );
}

#[test]
fn test_undeclared_function_call() {
    assert_semantic_err!(
        r#"
        func test() {
            let result = undefined_function(42);
        }
    "#
    );
}

#[test]
fn test_function_call_with_undeclared_argument() {
    assert_semantic_err!(
        r#"
        func helper(x: felt) -> felt {
            return x;
        }

        func test() {
            let result = helper(undefined_var);
        }
    "#
    );
}

#[test]
fn test_multiple_function_calls() {
    assert_semantic_ok!(
        r#"
        func add(a: felt, b: felt) -> felt {
            return a + b;
        }

        func multiply(a: felt, b: felt) -> felt {
            return a * b;
        }

        func test() -> felt {
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
        func inner(x: felt) -> felt {
            return x * 2;
        }

        func outer(x: felt) -> felt {
            return inner(x) + 1;
        }

        func test() -> felt {
            return outer(inner(5));
        }
    "#
    );
}

#[test]
fn test_function_call_in_expression() {
    assert_semantic_ok!(
        r#"
        func get_value() -> felt {
            return 42;
        }

        func test() -> felt {
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
        func is_null(x: felt) -> felt {
            return x == 0;
        }

        func test(x: felt) -> felt {
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
        func factorial(n: felt) -> felt {
            if (n == 1) {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }

        func test() -> felt {
            return factorial(5);
        }
    "#
    );
}
