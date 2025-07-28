//! # Integration Tests
//!
//! End-to-end integration tests that combine multiple semantic validation
//! features to ensure they work together correctly.

use crate::*;

mod multi_file;

#[test]
fn test_complete_program_with_multiple_functions() {
    assert_semantic_ok!(
        r#"
        fn add(a: felt, b: felt) -> felt {
            return a + b;
        }

        fn multiply(a: felt, b: felt) -> felt {
            return a * b;
        }

        fn calculate(x: felt, y: felt) -> felt {
            let sum = add(x, y);
            let product = multiply(sum, 2);
            return product;
        }

        fn main() -> felt {
            let result = calculate(10, 20);
            return result;
        }
    "#
    );
}

#[test]
fn test_program_with_structs_and_functions() {
    assert_semantic_ok!(
        r#"
        struct Point { x: felt, y: felt }

        fn create_point(x: felt, y: felt) -> Point {
            return Point { x: x, y: y };
        }

        fn distance_squared(p: Point) -> felt {
            return p.x * p.x + p.y * p.y;
        }

        fn main() -> felt {
            let point = create_point(3, 4);
            return distance_squared(point);
        }
        "#
    );
}

#[test]
#[ignore]
fn test_complex_control_flow_integration() {
    // TODO: fix this when support for arrays is added
    assert_semantic_ok!(
        r#"
        fn process_number(n: felt) -> felt {
            if n == 0 {
                return n / 2;
            } else {
                return 0;
            }
        }

        fn main() -> felt {
            let numbers: felt* = [1, 2, 3, 4, 5];
            let result = 0;

            // Process each number
            result = result + process_number(numbers[0]);
            result = result + process_number(numbers[1]);
            result = result + process_number(numbers[2]);

            return result;
        }
    "#
    );
}

#[test]
fn test_error_combination_undeclared_and_unused() {
    // This should catch both undeclared variable and unused variable errors
    assert_semantic_err!(
        r#"
        fn test() {
            let unused_var = 42;
            let result = undefined_var + 10;
        }
    "#
    );
}

#[test]
fn test_nested_scopes_with_function_calls() {
    assert_semantic_ok!(
        r#"
        fn helper(x: felt) -> felt {
            return x * 2;
        }

        fn complex_function(param: felt) -> felt {
            let outer = param;
            {
                let middle = helper(outer);
                {
                    let inner = helper(middle);
                    if inner == 100 {
                        return inner;
                    } else {
                        return helper(inner);
                    }
                }
            }
        }

        fn main() -> felt {
            return complex_function(10);
        }
    "#
    );
}

#[test]
fn test_comprehensive_error_detection() {
    // Test that multiple types of errors are detected in one program
    assert_semantic_err!(
        r#"
        fn helper(x: felt) -> felt {
            let unused = 42; // Unused variable
            return undefined_var; // Undeclared variable
        }

        fn test() -> felt {
            let shadowed = 1;
            let shadowed = 2;

            let result = nonexistent_function(10); // Undeclared function
            return result;

            let unreachable = 3; // Unreachable code
        }
    "#
    );
}
