//! Tests for unreachable code detection

use crate::*;

#[test]
fn test_code_after_return() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            return 42;
            let unreachable = 1; // Error: unreachable code
        }
    "#
    );
}

#[test]
fn test_code_after_return_in_block() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            {
                return 42;
                let unreachable = 1; // Error: unreachable code
            }
        }
    "#
    );
}

#[test]
fn test_code_after_if_with_returns() {
    assert_semantic_err!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            } else {
                return 2;
            }
            let unreachable = 3; // Error: unreachable code
        }
    "#
    );
}

#[test]
fn test_reachable_code_after_partial_if() {
    // Code after if without else should be reachable
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            }
            let reachable = 2; // OK: reachable if condition is false
            return reachable;
        }
    "#
    );
}

#[test]
fn test_multiple_returns_in_sequence() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            return 1;
            return 2; // Error: unreachable code
        }
    "#
    );
}

#[test]
fn test_unreachable_after_nested_return() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            {
                {
                    return 42;
                }
                let unreachable = 1; // Error: unreachable code
            }
        }
    "#
    );
}

#[test]
fn test_reachable_code_in_function() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 1;
            let y = x + 2;
            return y;
        }
    "#
    );
}

#[test]
fn test_unreachable_in_complex_control_flow() {
    assert_semantic_err!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                if (x == 10) {
                    return 1;
                } else {
                    return 2;
                }
                let unreachable = 3; // Error: unreachable code
            }
            return 4;
        }
    "#
    );
}
