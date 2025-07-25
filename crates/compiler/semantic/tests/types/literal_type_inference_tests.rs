//! Parameterised tests for context‑aware numeric literal type inference.
//!
//! We group all scenarios into `ok` and `err` buckets so they run through the
//! fast table‑driven macro, mirroring the style of other test modules.

use crate::{assert_semantic_parameterized, in_function};

#[test]
fn literal_type_inference_suite() {
    assert_semantic_parameterized! {
        // ------------------------------------------------------------------
        // Cases that must succeed
        // ------------------------------------------------------------------
        ok: [
            // -------- Basic inference from explicit variable/annotation ----
            in_function("let x: u32 = 3;"),
            in_function("let y: felt = 42;"),
            in_function("let x: u32 = 3; let y = x;"),

            // -------- Binary operations ------------------------------------
            in_function("let x: u32 = 3; let y = x + 1;"),
            in_function("let x: u32 = 3; let y = 1 + x;"),
            in_function("let a: u32 = 10; let b = (a + 5) * 2;"),
            in_function("let a: u32 = 10; let c = a - 1 + 3;"),

            // -------- Unary literal (negatives default to felt) ------------
            in_function("let neg = -5;"),

            // -------- Chained/complex expressions --------------------------
            in_function("let a: u32 = 5; let b: u32 = 10; let c = a + (b + 15);"),
            in_function("let a: u32 = 5; let b: u32 = 10; let d = a * 2 + b * 3;"),

            // -------- Tuple destructuring -----------------------------------
            in_function("let pair: (felt, u32) = (10, 20); let (x, y) = pair; let sum = x + 5;"),

            // -------- Comparison operators ----------------------------------
            in_function("let x: u32 = 10; let b = x < 100;"),
            in_function("let x: u32 = 10; let b = x == 0;"),
            in_function("let x: u32 = 10; let b = x >= 5 && x <= 15;"),

            // -------- Struct literals ---------------------------------------
            r#"
                struct Config { port: u32, timeout: u32 }
                fn test() { let _c = Config { port: 8080, timeout: 30 }; return; }
            "#,

            // Tuple‑typed field inference inside struct
            "struct P { f: (felt, u32) } fn test() { let _p = P { f: (10, 20) }; return; }",

            // Nested structs with mixed numeric kinds
            r#"
                struct Point { x: felt, y: felt }
                struct Rect  { tl: Point, w: u32, h: u32 }
                fn test() { let _r = Rect { tl: Point { x: 0, y: 0 }, w: 100, h: 200 }; return; }
            "#,

            // -------- Function call argument inference ----------------------
            r#"
                fn add_u32(a: u32, b: u32) -> u32 { return a + b; }
                fn test() { let res = add_u32(20, 30); return; }
            "#,

            // Explicit literal suffix still compiles
            in_function("let x: u32 = 10u32; let y: felt = 10felt;"),
        ],

        // ------------------------------------------------------------------
        // Cases that must fail
        // ------------------------------------------------------------------
        err: [
            // Mixed primitive kinds in arithmetic
            in_function("let x: u32 = 10; let y: felt = 20; let z = x + y;"),
            // Assigning u32 value to felt‑annotated variable
            in_function("let x: u32 = 10; let y: felt = x;"),
            // Wrong literal for explicit annotation
            in_function("let b: bool = 42;"),
            // Wrong suffix vs annotation
            in_function("let x: felt = 32u32;"),
            // Negative literal into unsigned variable
            in_function("let x: u32 = -5;"),

            // Struct field type mismatch from literal
            "struct P { x: felt, y: felt } fn test() { let _p = P { x: 10, y: 20u32 }; return; }",
            // Tuple field mismatch
            "struct S { f: (felt, bool) } fn test() { let _s = S { f: (10, 20) }; return; }",

            // Wrong addition of types
            in_function("let pair: (felt, u32) = (10, 20); let (x, y) = pair; let sum = x + y;"),
        ]
    }
}
