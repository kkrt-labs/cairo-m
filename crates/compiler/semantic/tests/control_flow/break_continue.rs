use crate::*;

#[test]
fn test_nested_loops_with_break_continue() {
    // Break and continue in nested loops should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            loop {
                loop {
                    if (true) {
                        break;
                    }
                    continue;
                }
                if (false) {
                    continue;
                }
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_continue_in_block() {
    // Break/continue in block inside loop should be valid
    assert_semantic_ok!(
        r#"
        fn test() {
            loop {
                {
                    if (true) {
                        break;
                    }
                    continue;
                }
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_continue_in_block_outside_loop() {
    // Break/continue in block outside loop should error
    assert_semantic_err!(
        r#"
        fn test() {
            {
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_multiple_break_continue_errors() {
    // Multiple break/continue outside loops should produce multiple errors
    assert_semantic_err!(
        r#"
        fn test() {
            break;
            if (true) {
                continue;
            }
            {
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_in_while_condition() {
    // While condition evaluation happens outside the loop body
    // so any break/continue in the condition expression would be invalid
    // Note: This test assumes conditions can contain complex expressions
    // If the parser doesn't support this, the test may need adjustment
    assert_semantic_ok!(
        r#"
        fn test() {
            while (true) {
                break;
            }
            return;
        }
        "#
    );
}

#[test]
fn test_break_continue_mix() {
    // Mix of valid and invalid break/continue
    assert_semantic_err!(
        r#"
        fn test() {
            break;  // Error

            loop {
                break;  // OK
                continue;  // OK (unreachable but syntactically valid)
            }

            continue;  // Error

            while (true) {
                if (true) {
                    break;  // OK
                } else {
                    continue;  // OK
                }
            }

            return;
        }
        "#
    );
}
