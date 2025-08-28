use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_casts() {
    assert_semantic_parameterized! {
        ok: [
            // u32 -> felt
            in_function("let x: u32 = 10; let y: felt = x as felt;"),
            ],
            err: [
            // Identity casts
            in_function("let x: felt = 10; let y: felt = x as felt;"),
            in_function("let x: u32 = 10; let y: u32 = x as u32;"),
            in_function("let x: bool = true; let y: bool = x as bool;"),

            // non-identity casts
            in_function("let x: felt = 10; let y: u32 = x as u32;"),
            in_function("let x: bool = true; let y: felt = x as felt;"),
            in_function("let x: felt = 10; let y: bool = x as bool;"),

            // Custom type
            format!("struct Point {{ x: felt, y: felt }} {}", in_function("let val = 3; let res = val as Point;")),
            in_function("let val = 3; let res = val as (felt, felt);"),
            in_function("let val = 3; let res = val as [felt; 10];"),
        ]
    }
}
