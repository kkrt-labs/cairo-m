//! Tests for assignment validation and type checking.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_assignments() {
    assert_semantic_parameterized! {
        ok: [
            in_function("let x: u32 = 100; let y: u32 = 200; x = y;"),
            in_function("let x: felt = 42; let y: felt = 100; x = y;"),
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let x: felt = 42; let p = Point { x: 10, y: 20 };")),
        ],

        err: [
            // Assignments of incompatible types
            in_function("let mut x: u32 = 100; let y: felt = 42; x = y;"),
            in_function("let mut z: felt = 50; let x: u32 = 100; z = x;"),
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let x: felt = 42; let p = Point { x: 10, y: 20 }; x = p;")),

            in_function("let x = 10; 42 = x;"),
            "fn get_value() -> felt { 42 } fn test() { let x = 10; get_value() = x; }",
            in_function("let x = 10; (x + 5) = 20;"),
            in_function("let x = 10; (10 + 20) = x;"),

            format!("fn get_tuple() -> (felt, u32, bool) {{ return (42, 100, true); }} {}", in_function("let (a: u32, b: felt, c: bool) = get_tuple();")),
            format!("fn get_tuple() -> (felt, u32, bool) {{ return (42, 100, true); }} {}", in_function("let (x, y) = get_tuple();")),
            format!("fn get_tuple() -> (felt, u32, bool) {{ return (42, 100, true); }} {}", in_function("let (p, q, r, s) = get_tuple();")),

            // Assignment with incompatible operator result type
            in_function("let x: felt = 42; let y: felt = 100; let z: felt = (x == y);"),
            in_function("let x: felt = 42; let y: felt = 100; let z: felt = (x != y);"),
        ]
    }
}

#[test]
fn test_const_assignment() {
    assert_semantic_parameterized! {
        ok: [
            r#"
            const X: felt = 42;
            const Y: felt = 100;
            fn foo() -> felt {
                return X + Y;
            }
            "#,

            r#"
            const X: u32 = 42;
            const Y: u32 = 100;
            fn foo() -> u32 {
                return X + Y;
            }
            "#,
            r#"
            const POW2: [u32; 3] = [1, 2, 4];
            fn foo() -> u32 {
                return POW2[0] + POW2[1];
            }
            "#,
        ],
        err: [
            in_function("const x = 42; x = 100;"), // Cannot re-assign to const
            r#"
            const POW2: [u32; 3] = [1, 2, 4felt];
            "#,
        ]
    }
}
