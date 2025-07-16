use crate::*;

#[test]
fn test_loop_body_creates_new_scope() {
    // Variables declared in loop body should not be visible outside
    assert_semantic_err!(
        r#"
        fn test() {
            loop {
                let x = 42;
                break;
            }
            let y = x;  // Error: x not in scope
            return;
        }
        "#
    );
}

// TODO: Add for-loop scoping test when iterator/range types are implemented

#[test]
fn test_nested_loop_scopes() {
    // Each loop creates its own scope
    assert_semantic_err!(
        r#"
        fn test() {
            loop {
                let outer = 1;
                loop {
                    let inner = 2;
                    let x = outer;  // OK: can access outer scope
                    break;
                }
                let y = inner;  // Error: inner not in scope
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_scoping() {
    // While loop body creates new scope
    assert_semantic_err!(
        r#"
        fn test() {
            let condition = true;
            while (condition) {
                let loop_var = 42;
                break;
            }
            let x = loop_var;  // Error: loop_var not in scope
            return;
        }
        "#
    );
}

#[test]
fn test_loop_variable_shadowing() {
    // Variables in loop can shadow outer variables
    assert_semantic_ok!(
        r#"
        fn test() {
            let x = 1;
            loop {
                let x = 2;  // Shadows outer x
                if (x == 2) {
                    break;
                }
            }
            let y = x;  // OK: refers to outer x
            return;
        }
        "#
    );
}

// TODO: Add for-loop shadowing test when iterator/range types are implemented

// TODO: Add multiple for-loops test when iterator/range types are implemented

#[test]
fn test_loop_scope_with_blocks() {
    // Blocks inside loops create additional scopes
    assert_semantic_err!(
        r#"
        fn test() {
            loop {
                let loop_var = 1;
                {
                    let block_var = 2;
                    let x = loop_var;  // OK
                }
                let y = block_var;  // Error: block_var not in scope
                break;
            }
            return;
        }
        "#
    );
}
