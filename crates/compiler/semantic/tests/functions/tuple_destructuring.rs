//! Tests for tuple destructuring validation

use crate::*;

#[test]
fn test_basic_tuple_destructuring() {
    assert_semantic_ok!(
        r#"
        func test() -> felt {
            let (x, y) = (10, 20);
            return x + y;
        }
        "#
    );
}

#[test]
fn test_tuple_destructuring_type_mismatch() {
    assert_semantic_err!(
        r#"
        func test() {
            let (x, y) = 42; // Error: Cannot destructure non-tuple
        }
        "#
    );
}

#[test]
fn test_tuple_destructuring_arity_mismatch() {
    assert_semantic_err!(
        r#"
        func test() {
            let (x, y) = (1, 2, 3); // Error: Pattern has 2 elements but value has 3
        }
        "#
    );
}

#[test]
fn test_tuple_destructuring_wrong_type_annotation() {
    assert_semantic_err!(
        r#"
        func test() {
            let (x, y): felt = (1, 2); // Error: Expected felt, found tuple
        }
        "#
    );
}

#[test]
fn test_local_tuple_destructuring() {
    assert_semantic_ok!(
        r#"
        func test() -> felt {
            local (a, b): (felt, felt) = (100, 200);
            let sum = a + b;
            return sum;
        }
        "#
    );
}

#[test]
fn test_tuple_destructuring_from_function() {
    assert_semantic_ok!(
        r#"
        func returns_tuple() -> (felt, felt) {
            return (100, 200);
        }
        
        func test() -> felt {
            let (a, b) = returns_tuple();
            return a + b;
        }
        "#
    );
}

#[test]
fn test_tuple_destructuring_unused_variables() {
    assert_semantic_err!(
        r#"
        func test() {
            let (x, y) = (1, 2); // y should be marked as unused
            let z = x + 1;
        }
        "#
    );
}
