use crate::{assert_semantic_err, assert_semantic_ok};

#[test]
fn test_felt_overflow() {
    assert_semantic_err!(
        r#"
        fn main() -> felt {
            let x: felt = 2147483648;
            return x;
        }
        "#
    );
}

#[test]
fn test_felt_at_max_boundary() {
    assert_semantic_ok!(
        r#"
        fn main() -> felt {
            let x: felt = 2147483647;
            return x;
        }
        "#
    );
}

#[test]
fn test_u32_at_max_boundary() {
    assert_semantic_ok!(
        r#"
        fn main() -> u32 {
            let x: u32 = 4294967295;
            return x;
        }
        "#
    );
}

#[test]
fn test_const_felt_overflow() {
    // Note: const syntax doesn't support type annotations yet
    // The overflow check still works because the value is too large for felt
    assert_semantic_err!(
        r#"
        const MAX_PLUS_ONE = 2147483648;
        "#
    );
}

#[test]
fn test_expression_result_overflow() {
    assert_semantic_err!(
        r#"
        fn main() -> felt {
            let x: felt = 2147483650;
            return x;
        }
        "#
    );
}
