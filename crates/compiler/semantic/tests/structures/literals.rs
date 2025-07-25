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
            // Can use literals for struct fields
            "struct Rectangle { width: u32, height: u32 } fn test() { let r = Rectangle { width: 100, height: 200 }; return;}",

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

            // Order can be different from the order of the fields in the struct definition.
            "struct Point { x: felt, y: felt } fn test() { let p = Point { y: 1, x: 2 }; return; }",

            // Can properly infer the type of a tuple in a struct literal.
            "struct Point { fields: (felt, u32) } fn test() { let p = Point { fields: (10, 20) }; return; }",

            // Can properly infer types in nested structs?
            "struct Point { x: felt, y: felt } struct Rectangle { top_left: Point, width: u32, height: u32 } fn test() { let r = Rectangle { top_left: Point { x: 0, y: 0 }, width: 100, height: 200 }; return; }",
            ],
            err: [
                // Duplicated field names
                "struct Point {x: felt, x: felt}",
                // From: extra_field.cm
                "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20, z: 30 }; return; }",
                // From: missing_field.cm
                "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10 }; } return;",

                // Wrong field type
                "struct Point { x: felt, y: felt } fn test() { let p = Point { x: true, y: 20 }; return;}",
                "struct Point { fields: (felt, bool) } fn test() { let p = Point { fields: (10, 20) }; return; }",
                r#"
                struct Foo { bar: (u32, (bool, felt)) }
                fn test() { let f = Foo { bar: (10, (1, 20)) }; return; }"#,

                // Wrong literal suffix
                "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20u32 }; return; }",

        ]
    }
}
