//! Tests for control flow path analysis

use crate::*;

#[test]
fn test_all_paths_return() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            } else {
                return 2;
            }
        }
    "#
    );
}

#[test]
fn test_not_all_paths_return() {
    assert_semantic_err!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            }
            // Missing return for else case
        }
    "#
    );
}

#[test]
fn test_nested_control_flow_all_paths() {
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

#[test]
fn test_nested_control_flow_missing_path() {
    assert_semantic_err!(
        r#"
        fn test(x: felt, y: felt) -> felt {
            if (x == 0) {
                if (y == 0) {
                    return 1;
                }
                // Missing return for inner else
            } else {
                return 3;
            }
        }
    "#
    );
}

#[test]
fn test_early_return_valid() {
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
