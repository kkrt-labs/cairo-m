//! Tests for array literal expressions and type inference
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_array_literal_type_inference() {
    assert_semantic_parameterized! {
        ok: [
            // Basic array literals with type inference
            in_function("let arr = [1, 2, 3, 4, 5];"),
            in_function("let arr = [1u32, 2u32, 3u32];"),
            in_function("let arr = [true, false, true];"),


            // Array with explicit type annotation
            in_function("let arr: [felt; 3] = [1, 2, 3];"),
            in_function("let arr: [u32; 4] = [1, 2, 3, 4];"),
            in_function("let arr: [bool; 2] = [true, false];"),

            // Single element array
            in_function("let arr = [42];"),
            in_function("let arr: [felt; 1] = [42];"),

            // Large arrays
            in_function("let arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];"),

            // Arrays with trailing comma
            in_function("let arr = [1, 2, 3,];"),

            // Array repetition
            in_function("let arr = [1; 10];"),
            in_function("let arr: [felt; 3] = [1; 3];"),
            in_function("let arr: [u32; 4] = [1u32; 4];"),

            ],
        err: [
                // Mixed types in array literal
                in_function("let arr = [1, true, 3];"),
                in_function("let arr = [1u32, 2, true];"),
                in_function("let arr = [42, \"hello\"];"),

                // Size mismatch with explicit type
                in_function("let arr: [felt; 3] = [1, 2];"),
                in_function("let arr: [felt; 2] = [1, 2, 3];"),
                in_function("let arr: [u32; 5] = [1, 2, 3];"),

                // Type mismatch with explicit type
                in_function("let arr: [bool; 3] = [1, 2, 3];"),
                in_function("let arr: [u32; 2] = [true, false];"),

                // Empty array without type annotation
                in_function("let arr = [];"),

                // Empty array with explicit type annotation
                in_function("let arr: [felt; 0] = [];"),
                in_function("let arr: [u32; 0] = [];"),

                // Nested arrays (not supported yet)
            in_function("let arr = [[1, 2], [3, 4]];"),
            in_function("let arr: [[felt; 2]; 2] = [[1, 2], [3, 4]];"),

                // Array repetition with type mismatch
                in_function("let arr: [felt; 4] = [1u32; 4];"),
                in_function("let arr: [u32; 5] = [1u32; 4];"),
        ]
    }
}

#[test]
fn test_array_literal_in_expressions() {
    assert_semantic_parameterized! {
        ok: [
            // Arrays in function calls
            r#"
                fn process(data: [felt; 3]) -> felt {
                    return 0;
                }
                fn test() {
                    let result = process([1, 2, 3]);
                    return;
                }
            "#,

            // Arrays as return values
            r#"
                fn create_array() -> [felt; 3] {
                    return [1, 2, 3];
                }
            "#,

            // Arrays in assignments
            in_function("let arr: [felt; 3] = [0, 0, 0]; arr = [1, 2, 3];"),
        ],
        err: [
            // Wrong size in function call
            r#"
                fn process(data: [felt; 3]) -> felt {
                    return 0;
                }
                fn test() {
                    let result = process([1, 2]);
                    return;
                }
            "#,

            // Wrong type in function call
            r#"
                fn process(data: [u32; 3]) -> felt {
                    return 0;
                }
                fn test() {
                    let result = process([true, false, true]);
                    return;
                }
            "#,

            // Wrong return type
            r#"
                fn create_array() -> [felt; 3] {
                    return [1, 2];
                }
            "#,
        ]
    }
}
