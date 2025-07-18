//! Tests for unused variable detection

use crate::*;

#[test]
fn test_simple_unused_variable() {
    assert_semantic_err!(
        r#"
        fn test() {
            let unused = 42;
            return ();
        }
    "#,
        show_unused
    );
}

#[test]
fn test_unused_parameter() {
    assert_semantic_err!(
        r#"
        fn test(unused_param: felt) {
            return ();
        }
    "#,
        show_unused
    );
}

#[test]
fn test_multiple_unused_variables() {
    assert_semantic_err!(
        r#"
        fn test() {
            let unused1 = 10;
            let unused2 = 20;
            let unused3 = 30;
            return ();
        }
    "#,
        show_unused
    );
}

#[test]
fn test_mixed_used_and_unused() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            let used = 10;
            let unused = 20;
            return used;
        }
    "#,
        show_unused
    );
}

#[test]
fn test_unused_in_nested_scope() {
    assert_semantic_err!(
        r#"
        fn test() -> felt {
            let used = 10;
            {
                let unused_inner = 20;
            }
            return used;
        }
    "#,
        show_unused
    );
}

#[test]
fn test_variable_used_in_expression() {
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
fn test_variable_used_in_assignment() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 10;
            let y = 20;
            y = x + 5;
            return y;
        }
    "#
    );
}

#[test]
fn test_parameter_used() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            return param + 1;
        }
    "#
    );
}

#[test]
fn test_unused_but_assigned() {
    // Variable is assigned but never read - should still be unused
    // TODO: fix this one
    assert_semantic_err!(
        r#"
        fn test() {
            let unused = 10;
            unused = 20;
            return ();
        }
    "#,
        show_unused
    );
}
