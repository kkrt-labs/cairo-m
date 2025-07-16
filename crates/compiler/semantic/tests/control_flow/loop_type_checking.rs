use crate::{assert_semantic_err, assert_semantic_ok};

#[test]
fn test_while_loop_with_felt_condition() {
    // This should fail - only bool is allowed for conditions
    assert_semantic_err!(
        r#"
        fn test() {
            let x: felt = 1;
            while (x) {
                return;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_with_non_felt_condition() {
    assert_semantic_err!(
        r#"
        struct Point {
            x: felt,
            y: felt,
        }

        fn test() {
            let p: Point = Point { x: 1, y: 2 };
            while (p) {
                return;
            }
        }
        "#
    );
}

#[test]
fn test_while_loop_with_tuple_condition() {
    assert_semantic_err!(
        r#"
        fn test() {
            let t: (felt, felt) = (1, 2);
            while (t) {
                return;
            }
        }
        "#
    );
}

#[test]
fn test_while_loop_with_complex_non_felt_expression() {
    assert_semantic_err!(
        r#"
        struct Config {
            enabled: felt,
        }

        fn test() {
            let config: Config = Config { enabled: 1 };
            // This should fail - accessing the struct itself, not the field
            while (config) {
                return;
            }
        }
        "#
    );
}

#[test]
fn test_while_loop_with_nested_conditions() {
    // This should fail - conditions must be bool, not felt
    assert_semantic_err!(
        r#"
        fn test() {
            let a: felt = 1;
            let b: felt = 0;

            while (a) {
                while (b) {
                    return;
                }
                return;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_with_comparison_expression() {
    // This should pass - comparisons return bool
    assert_semantic_ok!(
        r#"
        fn test() {
            let x: felt = 10;
            while (x == 0) {
                return;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_with_logical_expression() {
    // This should pass - logical operations return bool
    assert_semantic_ok!(
        r#"
        fn test() {
            let a: bool = true;
            let b: bool = false;
            while (a && b) {
                return;
            }
            return;
        }
        "#
    );
}
