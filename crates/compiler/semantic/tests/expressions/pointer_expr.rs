use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_new_expression_semantics() {
    assert_semantic_parameterized! {
        ok: [
            // Basic allocations
            in_function("let p: felt* = new felt[10];"),
            in_function("let n = 3; let q: u32* = new u32[n];"),

            // Struct allocations
            r#"
                struct Point { x: felt, y: felt }
                fn test() { let r: Point* = new Point[2]; return; }
            "#,

            // Zero-sized allocation is allowed
            in_function("let p: felt* = new felt[0];"),

            // Indexing pointers
            in_function("let p: felt* = new felt[10]; let a = p[0];"),
            in_function("let p: felt* = new felt[10]; p[0] = 1;"),

            // With structs
            r#"
                struct Point { x: felt, y: felt }
                fn test() { let p: Point* = new Point[2]; let a = p[0].x; return; }
            "#,

            // With tuples
            r#"
                fn test() { let p: (felt, felt)* = new (felt, felt)[2]; let a = p[0].0; return; }
            "#,
        ],
        err: [
            // Invalid element type (undeclared)
            in_function("let p = new Unknown[3];"),

            // Invalid count type
            in_function("let p = new felt[true];"),
            in_function("let p = new u32[Point { x: 1, y: 2 }];"),

            // Invalid type for count (non felt)
            in_function("let p = new felt[1u32];"),
        ]
    }
}
