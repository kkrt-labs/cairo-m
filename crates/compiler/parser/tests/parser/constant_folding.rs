use crate::assert_parses_ok;

#[test]
fn test_simple_addition() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 3 + 4;
            return x;
        }
        "#
    );
}

#[test]
fn test_complex_expression() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 3 + 4 * 2;
            return x;
        }
        "#
    );
}

#[test]
fn test_subtraction() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 10 - 3;
            return x;
        }
        "#
    );
}

#[test]
fn test_division() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 12 / 3;
            return x;
        }
        "#
    );
}

#[test]
fn test_division_with_remainder() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 13 / 3;
            return x;
        }
        "#
    );
}

#[test]
fn test_division_by_zero() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 10 / 0;
            return x;
        }
        "#
    );
}

#[test]
fn test_nested_operations() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = (2 + 3) * (10 - 5);
            return x;
        }
        "#
    );
}

#[test]
fn test_with_suffix() {
    assert_parses_ok!(
        r#"
        fn main() -> u32 {
            let x = 3u32 + 4u32;
            return x;
        }
        "#
    );
}

#[test]
fn test_mixed_suffix() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = 3u32 + 4;
            return x;
        }
        "#
    );
}

#[test]
fn test_partial_folding() {
    assert_parses_ok!(
        r#"
        fn main(y: felt) -> felt {
            let x = 3 + 4 + y;
            return x;
        }
        "#
    );
}

#[test]
fn test_unary_negation() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = -0;
            return x;
        }
        "#
    );
}

#[test]
fn test_unary_negation_non_zero() {
    assert_parses_ok!(
        r#"
        fn main() -> felt {
            let x = -5;
            return x;
        }
        "#
    );
}
