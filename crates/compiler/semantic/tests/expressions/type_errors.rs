//! Tests for enhanced type error messages with suggestions

use crate::assert_semantic_err;

#[test]
fn test_struct_in_arithmetic_operation() {
    // This test verifies that we get helpful suggestions when trying to use structs in arithmetic
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func test() {
            let p = Point { x: 10, y: 20 };
            let result = p + 5;  // Error: struct in arithmetic
        }
    "#
    );
}

#[test]
fn test_struct_with_numeric_field_suggestion() {
    // Test suggestion for accessing numeric field when struct has one
    assert_semantic_err!(
        r#"
        struct Counter { value: felt }

        func test() {
            let c = Counter { value: 42 };
            let result = c * 2;  // Should suggest accessing 'value' field
        }
    "#
    );
}

#[test]
fn test_tuple_in_arithmetic_operation() {
    // Test tuple in arithmetic with suggestion
    assert_semantic_err!(
        r#"
        func test() {
            let t = (42,);
            let result = t + 10;  // Should suggest accessing with [0]
        }
    "#
    );
}

#[test]
fn test_function_not_called_error() {
    // Test function used without parentheses
    assert_semantic_err!(
        r#"
        func get_value() -> felt {
            return 42;
        }

        func test() {
            let x = get_value + 5;  // Should suggest adding parentheses
        }
    "#
    );
}

#[test]
fn test_comparison_type_mismatch_with_context() {
    // Test enhanced comparison error messages
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func test() {
            let p = Point { x: 1, y: 2 };
            let num = 42;
            if (p == num) {  // Type mismatch with context
                return ();
            }
            return ();
        }
    "#
    );
}

#[test]
fn test_function_argument_type_mismatch_with_param_name() {
    // Test enhanced function call error messages
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func distance(p1: Point, p2: Point) -> felt {
            return 0;
        }

        func test() {
            let p = Point { x: 1, y: 2 };
            let d = distance(p, 42);  // Should show parameter name 'p2'
        }
    "#
    );
}

#[test]
fn test_assignment_type_mismatch_with_context() {
    // Test enhanced assignment error messages
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func test() {
            let x: felt = 10;
            let p = Point { x: 1, y: 2 };
            x = p;  // Should show variable type context
            return();
        }
    "#
    );
}

#[test]
fn test_return_type_mismatch_with_function_context() {
    // Test enhanced return type error messages
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func get_coordinate() -> felt {
            let p = Point { x: 10, y: 20 };
            return p;  // Should show function signature context
        }
    "#
    );
}

#[test]
fn test_if_condition_type_error() {
    // Test if condition type checking
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func test() {
            let p = Point { x: 1, y: 2 };
            if (p) {  // Non-felt condition
                return ();
            }
            return ();
        }
    "#
    );
}

#[test]
fn test_multiple_type_errors_with_suggestions() {
    // Test that multiple errors all get suggestions
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }
        struct Counter { value: felt }

        func test() -> felt {
            let p = Point { x: 1, y: 2 };
            let c = Counter { value: 10 };
            let result = p + c;  // Two type errors, both should have suggestions
            return result;
        }
    "#
    );
}

#[test]
fn test_unary_op_type_error() {
    // Test unary op type error
    assert_semantic_err!(
        r#"
        struct Point { x: felt, y: felt }

        func test() -> felt {
            let p = Point { x: 1, y: 2 };
            let x = -p;  // Should show type error for negation on struct
            return x;
        }
    "#
    );
}
