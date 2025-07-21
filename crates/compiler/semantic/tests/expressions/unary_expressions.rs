//! Tests for unary expression validation.
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_unary_operator_types() {
    assert_semantic_parameterized! {
        ok: [
            // bool
            in_function("let b: bool = true; let not_bool = !b;"),

            // felt
            in_function("let x: felt = 42; let neg_felt = -x;"),

            ],
            err: [
            //TODO this currently fails as unary operators are not supported for u32 yet. TODO: fix
            // u32
            in_function("let y: u32 = 100; let neg_u32 = -y;"),


                // bool
            in_function("let b: bool = true; let neg_bool = -b;"),

            // felt
            in_function("let x: felt = 42; let neg_felt = !x;"),

            // u32
            in_function("let y: u32 = 100; let neg_u32 = !y;"),

            // Custom type
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let p = Point { x: 10, y: 20 }; let neg_struct = -p;")),
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let p = Point { x: 10, y: 20 }; let not_struct = !p;")),
        ]
    }
}
