//! Tests for duplicate definition detection

use crate::*;

#[test]
fn test_duplicate_in_same_scope() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let var = 10;
            let another = 20;
            let var = 30;
            return var;
        }
    "#
    );
}

#[test]
fn test_duplicate_parameters() {
    assert_semantic_err!(
        r#"
        fn test(param: felt, param: felt) -> felt {
            return param;
        }
    "#
    );
}

#[test]
fn test_parameter_and_local_shadowing() {
    // Local variables can shadow parameters
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            let param = 42;
            return param;
        }
    "#
    );
}

#[test]
fn test_no_duplicate_across_scopes() {
    // This should be OK - different scopes can have same variable names
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 1;
            {
                let x = 2; // OK: different scope
            }
            return x;
        }
    "#
    );
}

#[test]
fn test_duplicate_function_names() {
    assert_semantic_err!(
        r#"
        fn duplicate_func() {
            return ();
        }

        fn duplicate_func() {
            return ();
        }
    "#
    );
}

#[test]
fn test_multiple_shadowing() {
    // Multiple variables can be shadowed
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 1;
            let y = 2;
            let x = 3;
            let y = 4;
            return x + y;
        }
    "#
    );
}
