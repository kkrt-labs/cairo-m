//! Tests for unused variable detection
use crate::*;

#[test]
fn test_unused_variable_detection() {
    assert_semantic_parameterized! {
        ok: [
            "fn test() -> felt { let x = 10; let y = 20; return x + y; }",
            "fn test() -> felt { let y = 20; let x = 10; y = x + 5; return y; }",
            "fn test(param: felt) -> felt { return param + 1; }",
        ],
        err: [
            in_function("let unused = 42;"),
            "fn test(unused_param: felt) { return (); }",
            in_function("let unused1 = 10; let unused2 = 20; let unused3 = 30;"),
            "fn test() -> felt { let used = 10; let unused = 20; return used; }",
            "fn test() -> felt { let used = 10; { let unused_inner = 20; } return used; }",
        ],
        show_unused
    }
}

#[test]
#[ignore]
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
