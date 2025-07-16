//! Tests for function return type validation

use crate::*;

#[test]
fn test_function_with_felt_return_type() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            return 42;
        }
    "#
    );
}

#[test]
fn test_function_with_unit_return_type() {
    assert_semantic_ok!(
        r#"
        fn test() {
            return ();
        }
    "#
    );
}

#[test]
fn test_function_with_explicit_unit_return_type() {
    assert_semantic_ok!(
        r#"
        fn test() -> () {
            return ();
        }
    "#
    );
}

#[test]
fn test_function_missing_return_statement() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            let x = 42;
            // Missing return statement
        }
    "#
    );
}

#[test]
fn test_function_unit_missing_return_statement() {
    // Unit functions should still require explicit return
    assert_semantic_err!(
        r#"
        fn test() {
            let x = 42;
            // Missing return () statement
        }
    "#
    );
}

#[test]
fn test_function_return_variable() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 42;
            return x;
        }
    "#
    );
}

#[test]
fn test_function_return_expression() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 10;
            let y = 20;
            return x + y;
        }
    "#
    );
}

#[test]
fn test_function_return_function_call() {
    assert_semantic_ok!(
        r#"
        fn helper() -> felt {
            return 42;
        }

        fn test() -> felt {
            return helper();
        }
    "#
    );
}

#[test]
fn test_function_return_complex_expression() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let a = 10;
            let b = 5;
            let c = 2;
            return (a + b) * c - 1;
        }
    "#
    );
}

#[test]
fn test_function_return_in_if_statement() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return x;
            } else {
                return 0;
            }
        }
    "#
    );
}

#[test]
fn test_function_return_missing_in_if_branch() {
    assert_semantic_err!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return x;
            }
            // Missing return for else case
        }
    "#
    );
}

#[test]
fn test_function_return_missing_in_else_branch() {
    assert_semantic_err!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return x;
            } else {
                let y = x + 1;
                // Missing return in else branch
            }
        }
    "#
    );
}

#[test]
fn test_function_return_in_nested_blocks() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            {
                {
                    return 42;
                }
            }
        }
    "#
    );
}

#[test]
fn test_function_return_parameter() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            return param;
        }
    "#
    );
}

#[test]
fn test_function_return_modified_parameter() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            param = param + 1;
            return param;
        }
    "#
    );
}

#[test]
fn test_function_early_return() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return 0; // Early return
            }

            let result = x * 2;
            return result;
        }
    "#
    );
}

#[test]
fn test_function_multiple_return_paths() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 100) {
                return 100;
            } else if (x == 0) {
                return 0;
            } else {
                return x;
            }
        }
    "#
    );
}

#[test]
fn test_function_return_undeclared_variable() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            return undefined_var;
        }
    "#
    );
}

#[test]
fn test_recursive_function_return() {
    assert_semantic_ok!(
        r#"
        fn factorial(n: felt) -> felt {
            if (n == 1) {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }
    "#
    );
}

#[test]
fn test_function_return_literal() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            return 42;
        }
    "#
    );
}

#[test]
fn test_function_void_return_with_value() {
    // Unit functions should return () not values
    assert_semantic_err!(
        r#"
        fn test() {
            return 42; // Error: should return ()
        }
    "#
    );
}

#[test]
fn test_function_nested_control_flow_all_paths_return() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt, y: felt) -> felt {
            if (x == 0) {
                if (y == 0) {
                    return 1;
                } else {
                    return 2;
                }
            } else {
                return 3;
            }
        }
    "#
    );
}
