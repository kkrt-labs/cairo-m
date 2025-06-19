use crate::*;

#[test]
fn test_code_after_infinite_loop() {
    // Code after an infinite loop (without breaks) is unreachable
    assert_semantic_err!(
        r#"
        func test() {
            loop {
                let x = 1;
            }
            let y = 2;  // Unreachable
            return;     // Unreachable
        }
        "#
    );
}

#[test]
fn test_code_after_loop_with_break() {
    // Code after a loop with break is reachable
    assert_semantic_ok!(
        r#"
        func test() {
            loop {
                break;
            }
            let y = 2;  // Reachable
            return;     // Reachable
        }
        "#
    );
}

#[test]
fn test_code_after_break() {
    // Code after break in loop is unreachable
    assert_semantic_err!(
        r#"
        func test() {
            loop {
                break;
                let x = 1;  // Unreachable
            }
            return;
        }
        "#
    );
}

#[test]
fn test_code_after_continue() {
    // Code after continue in loop is unreachable
    assert_semantic_err!(
        r#"
        func test() {
            loop {
                continue;
                let x = 1;  // Unreachable
            }
            return;
        }
        "#
    );
}

#[test]
fn test_conditional_break_unreachable() {
    // Conditional breaks don't make following code unreachable
    assert_semantic_ok!(
        r#"
        func test() {
            loop {
                if (1) {
                    break;
                }
                let x = 1;  // Reachable (break is conditional)
            }
            return;
        }
        "#
    );
}

#[test]
fn test_nested_loop_unreachable() {
    // Unreachable code in nested loops
    assert_semantic_err!(
        r#"
        func test() {
            loop {
                loop {
                    break;
                    let x = 1;  // Unreachable
                }
                break;
                let y = 2;  // Unreachable
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_unreachable() {
    // While loops don't make following code unreachable (might not execute)
    assert_semantic_ok!(
        r#"
        func test() {
            while (0) {
                let x = 1;
            }
            let y = 2;  // Reachable
            return;
        }
        "#
    );
}

// TODO: Add for-loop unreachable test when iterator/range types are implemented

#[test]
fn test_loop_with_return() {
    // Return in loop makes code after it unreachable
    assert_semantic_err!(
        r#"
        func test() {
            loop {
                return;
                let x = 1;  // Unreachable
            }
            let y = 2;  // Unreachable (but might not be reported due to infinite loop)
        }
        "#
    );
}
