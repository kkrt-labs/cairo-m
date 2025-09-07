//! Tests for semantic validation of const arrays escaping by pointer
use crate::assert_semantic_parameterized;

#[test]
fn test_const_arrays_cant_be_written_to() {
    assert_semantic_parameterized! {
        ok: [
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn main() { let _x = ARR[0]; return;}
            "#,
        ],
        err: [
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn main() { ARR[0] = 3u32; return; }
            "#,
        ]
    }
}

#[test]
fn test_const_arrays_blocked_in_calls_and_aggregates() {
    assert_semantic_parameterized! {
        ok: [
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn id(a: u32) -> u32 { return a; }
            fn main() { let _x = ARR[0]; return; }
            "#,
        ],
        err: [
            // Passing const array to function parameter
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn f(a: [u32; 2]) { let _ = a[0]; return; }
            fn main() { f(ARR); return; }
            "#,
            // Embedding const array in struct field of array type
            r#"
            struct S { a: [u32; 2] }
            const ARR: [u32; 2] = [1u32, 2u32];
            fn main() { let _s = S { a: ARR }; return; }
            "#,
            // Returning const array in function with array return type
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn give() -> [u32; 2] { return ARR; }
            "#,
            // Tuple embedding
            r#"
            const ARR: [u32; 2] = [1u32, 2u32];
            fn main() { let _t = (ARR, 1u32); return; }
            "#,
        ]
    }
}

#[test]
fn test_const_arrays_cross_module_via_use() {
    use crate::multi_file;
    assert_semantic_parameterized! {
        ok: [],
        err: [
            multi_file(
                "main.cm",
                &[
                    ("constants.cm", "const ARR: [u32; 2] = [1u32, 2u32];"),
                    (
                        "main.cm",
                        r#"
                        use constants::ARR;
                        fn f(a: [u32; 2]) { let _ = a[0]; return; }
                        fn main() { f(ARR); return; }
                        "#,
                    ),
                ],
            ),
        ]
    }
}
