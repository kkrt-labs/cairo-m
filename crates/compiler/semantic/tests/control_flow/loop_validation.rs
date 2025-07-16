use crate::*;

#[test]
fn test_break_in_loop() {
    // Break inside loop should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            let running = 1;
            loop {
                if (running) {
                    break;
                }
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_outside_loop() {
    // Break outside loop should error
    assert_semantic_err!(
        r#"
        fn test() {
            break;
            return;
        }
        "#
    );
}

#[test]
fn test_continue_in_loop() {
    // Continue inside loop should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            let x = 0;
            loop {
                if (x) {
                    break;
                }
                continue;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_continue_outside_loop() {
    // Continue outside loop should error
    assert_semantic_err!(
        r#"
        fn test() {
            continue;
            return;
        }
        "#
    );
}

#[test]
fn test_break_in_nested_loops() {
    // Break in nested loops should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            loop {
                loop {
                    break;
                }
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_in_if_inside_loop() {
    // Break inside if statement within loop should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            loop {
                if (1) {
                    break;
                }
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_in_if_outside_loop() {
    // Break inside if statement outside loop should error
    assert_semantic_err!(
        r#"
        fn test() {
            if (1) {
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_while_loop_with_break() {
    // Break in while loop should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            while (1) {
                break;
            }
            return;
        }
        "#
    );
}

// TODO: Add for-loop tests when iterator/range types are implemented
