//! Tests for struct literal validation.
use crate::assert_semantic_parameterized;

#[test]
fn test_struct_literals() {
    assert_semantic_parameterized! {
        ok: [
            r#"
                struct Point { x: felt, y: felt }
                struct Rectangle {
                    top_left: Point,
                    width: felt,
                    height: felt,
                }
                fn test() {
                    let p = Point { x: 10, y: 20 };
                    let r = Rectangle {
                        top_left: Point { x: 0, y: 0 },
                        width: 100,
                        height: 200,
                    };
                    return;
                }
            "#,
        ],
        err: [
            // TODO: This errors because we cant infer u32 type from struct literals (yet). TODO: fix
            r#"
            struct Point { x: felt, y: felt }
            struct Rectangle {
                top_left: Point,
                width: u32,
                height: u32,
            }
            fn test() {
                let p = Point { x: 10, y: 20 };
                let r = Rectangle {
                    top_left: Point { x: 0, y: 0 },
                    width: 100,
                    height: 200,
                };
                return;
            }
        "#,


            // From: extra_field.cm
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20, z: 30 }; return; }",
            // From: missing_field.cm
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10 }; } return;",
            // From: wrong_field_type.cm
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: true, y: 20 }; return;}",
            "struct Rectangle { width: u32, height: u32 } fn test() { let r = Rectangle { width: 100, height: 200 }; return;}",
        ]
    }
}
