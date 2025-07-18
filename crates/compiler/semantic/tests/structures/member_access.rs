//! Tests for struct member access validation.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_struct_member_access() {
    assert_semantic_parameterized! {
        ok: [
            // From: valid_field_access.cm
            r#"
                struct Point { x: felt, y: felt }
                struct Rectangle {
                    top_left: Point,
                    bottom_right: Point,
                }
                fn test() -> felt {
                    let p = Point { x: 10, y: 20 };
                    let x_val = p.x;
                    let y_val = p.y;

                    let rect = Rectangle {
                        top_left: Point { x: 0, y: 0 },
                        bottom_right: Point { x: 100, y: 100 },
                    };

                    let corner_x = rect.top_left.x;
                    let corner_y = rect.bottom_right.y;

                    return corner_x + corner_y;
                }
            "#,
        ],
        err: [
            // From: member_access_on_primitive.cm
            in_function("let x: felt = 42; let a = x.field;"),
            in_function("let y: u32 = 100; let b = y.value;"),
            in_function("let z: bool = true; let c = z.flag;"),
            // From: member_access_on_tuple.cm
            in_function("let t = (10, 20, 30); let x = t.x;"),
            in_function("let t = (10, 20, 30); let first = t.first;"),
            // From: non_existent_field.cm
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20 }; let z = p.z; return;}",
            "struct Point { x: felt, y: felt } fn test() { let p = Point { x: 10, y: 20 }; let mag = p.magnitude; return;}",
        ]
    }
}
