//! Tests for missing return statement detection

use crate::*;

#[test]
fn test_missing_return_simple() {
    assert_semantic_err!(
        r#"
        func test() -> felt {
            let x = 42;
            // Missing return statement
        }
    "#
    );
}

#[test]
fn test_missing_return_with_if() {
    assert_semantic_err!(
        r#"
        func test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            }
            // Missing return for else case
        }
    "#
    );
}

#[test]
fn test_missing_return_in_else() {
    assert_semantic_err!(
        r#"
        func test(x: felt) -> felt {
            if (x == 0) {
                return 1;
            } else {
                let y = x + 1;
                // Missing return in else branch
            }
        }
    "#
    );
}

#[test]
fn test_valid_return_all_paths() {
    assert_semantic_ok!(
        r#"
        func test(x: felt) -> felt {
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
fn test_valid_return_simple() {
    assert_semantic_ok!(
        r#"
        func test() -> felt {
            let x = 42;
            return x;
        }
    "#
    );
}

#[test]
fn test_unit_return_type_ok() {
    assert_semantic_ok!(
        r#"
        func test() {
            let x = 42;
            return ();
        }
    "#
    );
}

#[test]
fn test_unit_return_type_implicit() {
    // Functions with unit return type should still require explicit return
    assert_semantic_err!(
        r#"
        func test() {
            let x = 42;
            // Missing return () for unit functions
        }
    "#
    );
}

#[test]
fn test_nested_missing_return() {
    assert_semantic_err!(
        r#"
        func test(x: felt) -> felt {
            if (x == 0) {
                if (x == 10) {
                    return 1;
                } else {
                    // Missing return in nested else
                }
            } else {
                return 2;
            }
        }
    "#
    );
}

#[test]
fn test_complex_control_flow_valid() {
    assert_semantic_ok!(
        r#"
        func test(x: felt) -> felt {
            if (x == 0) {
                if (x == 10) {
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
fn test_return_in_nested_block() {
    assert_semantic_ok!(
        r#"
        func test() -> felt {
            {
                {
                    return 42;
                }
            }
        }
    "#
    );
}
