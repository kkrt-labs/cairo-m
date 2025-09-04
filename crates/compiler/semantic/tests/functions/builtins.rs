use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_assert_built_in_parameterized() {
    assert_semantic_parameterized! {
        ok: [
            // felt equality/inequality and immediates
            in_function("let x = 3; let y = 4; assert(x == y);"),
            in_function("let x = 3; let y = 4; assert(x != y);"),
            in_function("let x = 3; assert(x == 0);"),
            in_function("let x = 3; assert(0 == x);"),

            // u32 equality and immediates
            in_function("let x32: u32 = 3u32; let y32: u32 = 4u32; assert(x32 == y32);"),
            in_function("let x32: u32 = 3u32; assert(x32 == 5u32);"),
            in_function("let x32: u32 = 3u32; assert(5u32 == x32);"),
            in_function("let x: u32 = 1; let y: u32 = 2; assert(x < y);"),

            // bool equality/inequality
            in_function("let tt = true; let ff = false; assert(tt == ff);"),
            in_function("let tt = true; let ff = false; assert(tt != ff);"),
            // Mixed numeric literal and u32 should infer to bool
            in_function("assert(1 == 2u32);"),
        ],
        err: [
            // Non-binary / wrong operator
            in_function("let x = 1; assert(x);"),

            // Mixed types
            in_function("let a: felt = 1; let b: u32 = 2u32; assert(a == b);"),
            in_function("let a: bool = true; let b: felt = 1; assert(a == b);"),

            // Aggregate types not supported
            format!(
                "struct P {{ a: felt }} {}",
                in_function("let p1 = P { a: 1 }; let p2 = P { a: 2 }; assert(p1 == p2);")
            ),
            in_function("let a: [u32; 2] = [1u32, 2u32]; let b: [u32; 2] = [1u32, 2u32]; assert(a == b);"),
        ]
    }
}
