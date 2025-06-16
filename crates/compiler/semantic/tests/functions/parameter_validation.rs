//! Tests for function parameter validation

use crate::*;

#[test]
fn test_function_with_single_parameter() {
    assert_semantic_ok!(
        r#"
        func test(x: felt) -> felt {
            return x;
        }
    "#
    );
}

#[test]
fn test_function_with_multiple_parameters() {
    assert_semantic_ok!(
        r#"
        func test(x: felt, y: felt, z: felt) -> felt {
            return x + y + z;
        }
    "#
    );
}

#[test]
fn test_function_with_no_parameters() {
    assert_semantic_ok!(
        r#"
        func test() -> felt {
            return 42;
        }
    "#
    );
}

#[test]
fn test_duplicate_parameter_names() {
    assert_semantic_err!(
        r#"
        func test(x: felt, x: felt) -> felt {
            return x;
        }
    "#
    );
}

#[test]
fn test_parameter_used_in_function_body() {
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            let var = param + 1;
            return var;
        }
    "#
    );
}

#[test]
fn test_unused_parameter_warning() {
    assert_semantic_err!(
        r#"
        func test(unused_param: felt) -> felt {
            return 42;
        }
    "#,
        show_unused
    );
}

#[test]
fn test_parameter_used_in_nested_scope() {
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            {
                let inner = param * 2;
                return inner;
            }
        }
    "#
    );
}

#[test]
fn test_parameter_used_in_if_statement() {
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            if (param == 0) {
                return param;
            } else {
                return 0;
            }
        }
    "#
    );
}

#[test]
fn test_parameter_assignment() {
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            param = param + 1;
            return param;
        }
    "#
    );
}

#[test]
fn test_parameter_vs_local_variable_shadowing() {
    // Shadowing is now supported - local variables can shadow parameters
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            let param = 42; // OK: shadows parameter
            return param;
        }
    "#
    );
}

#[test]
fn test_parameter_shadowed_in_nested_scope() {
    // Parameter can be shadowed in nested scope
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            {
                let param = 100; // OK: shadows parameter in nested scope
                return param;
            }
        }
    "#
    );
}

#[test]
fn test_multiple_parameters_all_used() {
    assert_semantic_ok!(
        r#"
        func test(a: felt, b: felt, c: felt) -> felt {
            return a + b + c;
        }
    "#
    );
}

#[test]
fn test_multiple_parameters_some_unused() {
    assert_semantic_err!(
        r#"
        func test(used: felt, unused1: felt, unused2: felt) -> felt {
            return used;
        }
    "#,
        show_unused
    );
}

#[test]
fn test_parameter_used_in_function_call() {
    assert_semantic_ok!(
        r#"
        func helper(x: felt) -> felt {
            return x * 2;
        }

        func test(param: felt) -> felt {
            return helper(param);
        }
    "#
    );
}

#[test]
fn test_parameter_used_in_complex_expression() {
    assert_semantic_ok!(
        r#"
        func test(a: felt, b: felt, c: felt) -> felt {
            return (a + b) * c - a / b;
        }
    "#
    );
}

#[test]
fn test_parameter_type_annotation() {
    assert_semantic_ok!(
        r#"
        func test(param: felt) -> felt {
            return param;
        }
    "#
    );
}

#[test]
fn test_function_call_with_correct_parameter_count() {
    assert_semantic_ok!(
        r#"
        func helper(x: felt, y: felt) -> felt {
            return x + y;
        }

        func test() -> felt {
            return helper(10, 20);
        }
    "#
    );
}

#[test]
fn test_parameter_used_as_condition() {
    assert_semantic_ok!(
        r#"
        func test(condition: felt) -> felt {
            if (condition == 0) {
                return 1;
            } else {
                return 0;
            }
        }
    "#
    );
}

#[test]
fn test_parameter_used_in_return_statement() {
    assert_semantic_ok!(
        r#"
        func test(value: felt) -> felt {
            return value;
        }
    "#
    );
}

#[test]
fn test_recursive_function_with_parameter() {
    assert_semantic_ok!(
        r#"
        func factorial(n: felt) -> felt {
            if (n == 1) {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }
    "#
    );
}
