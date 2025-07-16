//! Tests for complex nested scope scenarios

use crate::*;

#[test]
fn test_deeply_nested_scopes() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let level1 = 1;
            {
                let level2 = level1 + 1;
                {
                    let level3 = level2 + 1;
                    {
                        let level4 = level3 + 1;
                        return level4;
                    }
                }
            }
        }
    "#
    );
}

#[test]
fn test_nested_scope_variable_access() {
    assert_semantic_err!(
        r#"
        fn test() {
            let outer = 1;
            {
                let middle = 2;
                {
                    let inner = 3;
                }
                let bad = inner; // Error: inner not accessible
            }
        }
    "#
    );
}

#[test]
fn test_nested_if_scopes() {
    assert_semantic_ok!(
        r#"
        fn test(x: felt) -> felt {
            if (x == 0) {
                let positive = x;
                if (positive == 10) {
                    let large = positive * 2;
                    return large;
                } else {
                    return positive;
                }
            } else {
                return 0;
            }
        }
    "#
    );
}

#[test]
fn test_nested_scope_shadowing() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 1;
            {
                let x = 2; // Shadows outer x
                {
                    let x = 3; // Shadows middle x
                    return x; // Returns 3
                }
            }
        }
    "#
    );
}

#[test]
fn test_complex_scope_interaction() {
    assert_semantic_err!(
        r#"
        fn test() {
            let a = 1;
            {
                let b = a + 1; // OK: a is visible
                {
                    let c = b + 1; // OK: b is visible
                }
                let bad1 = c; // Error: c not visible
            }
            let bad2 = b; // Error: b not visible outside its scope
        }
    "#
    );
}

#[test]
fn test_nested_function_calls_with_scopes() {
    assert_semantic_ok!(
        r#"
        fn helper(x: felt) -> felt {
            return x * 2;
        }

        fn test() -> felt {
            let outer = 5;
            {
                let middle = helper(outer);
                {
                    let inner = helper(middle);
                    return inner;
                }
            }
        }
    "#
    );
}

#[test]
fn test_scope_boundaries_with_assignments() {
    assert_semantic_err!(
        r#"
        fn test() {
            let x = 1;
            {
                let y = 2;
                x = y + 1; // OK: x is visible, y is visible
            }
            y = 3; // Error: y not visible outside its scope
        }
    "#
    );
}

#[test]
fn test_multiple_nested_branches() {
    assert_semantic_ok!(
        r#"
        fn test(condition: felt) -> felt {
            let base = 10;
            if (condition == 0) {
                let branch1 = base + 1;
                {
                    let nested1 = branch1 * 2;
                    return nested1;
                }
            } else {
                let branch2 = base - 1;
                {
                    let nested2 = branch2 * 3;
                    return nested2;
                }
            }
        }
    "#
    );
}
