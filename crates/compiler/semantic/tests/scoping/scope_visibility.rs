//! Tests for scope visibility and boundaries

use crate::*;

#[test]
fn test_basic_scope_visibility() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let outer = 1;
            {
                let inner = outer + 1; // OK: outer is visible
                return inner;
            }
        }
    "#
    );
}

#[test]
fn test_inner_scope_not_visible_outside() {
    assert_semantic_err!(
        r#"
        fn test() {
            {
                let inner = 42;
            }
            let bad = inner; // Error: inner not visible
        }
    "#
    );
}

#[test]
fn test_nested_scope_access() {
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let level1 = 1;
            {
                let level2 = level1 + 1; // OK: level1 visible
                {
                    let level3 = level2 + level1; // OK: both visible
                    return level3;
                }
            }
        }
    "#
    );
}

#[test]
fn test_sibling_scopes_not_visible() {
    assert_semantic_err!(
        r#"
        fn test() {
            {
                let first_scope = 1;
            }
            {
                let second_scope = first_scope; // Error: not visible
            }
        }
    "#
    );
}

#[test]
fn test_parameter_visible_in_function() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            let var =  param + 1;
            return var;
        }
    "#
    );
}

#[test]
fn test_parameter_visible_in_nested_scopes() {
    assert_semantic_ok!(
        r#"
        fn test(param: felt) -> felt {
            {
                let inner = param + 1; // OK: param visible
                {
                    let deeper = param + inner; // OK: both visible
                    return deeper;
                }
            }
        }
    "#,
        show_unused
    );
}

#[test]
fn test_if_statement_scope() {
    assert_semantic_err!(
        r#"
        fn test() {
            if (true) {
                let if_var = 42;
            }
            let bad = if_var; // Error: if_var not visible outside if
        }
    "#
    );
}

#[test]
fn test_variable_shadowing_different_scopes() {
    // This should be OK - different scopes can have same variable names
    assert_semantic_ok!(
        r#"
        fn test() -> felt {
            let x = 1;
            {
                let x = 2; // OK: different scope
                return x;
            }
        }
    "#
    );
}

#[test]
fn test_complex_nested_visibility() {
    assert_semantic_err!(
        r#"
        fn test() {
            let outer = 1;
            {
                let middle = outer + 1; // OK
                {
                    let inner = middle + outer; // OK
                }
                let bad = inner; // Error: inner not visible
            }
        }
    "#
    );
}
