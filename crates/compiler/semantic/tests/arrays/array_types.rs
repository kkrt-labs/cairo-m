//! Tests for array type declarations and compatibility
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_array_type_declarations() {
    assert_semantic_parameterized! {
        ok: [

            // Array types in function parameters
            "fn process_u32_array(data: [u32; 5]) -> u32 { return 0; }",

            // Array types as return types
            "fn create_bool_array() -> [bool; 2] { return [true, false]; }",

            // Arrays in structs
            "struct Data { values: [felt; 10], flags: [bool; 8] }",

            // Arrays in tuples
            in_function("let t: ([felt; 3], felt) = ([1, 2, 3], 42);"),
            ],
            err: [
                // Arrays must be initialized with their declaration.
                in_function("let arr: [felt; 5];"),
                // Nested arrays (not supported)
                in_function("let arr: [[felt; 3]; 2] = [[1, 2, 3], [4, 5, 6]];"),

            // Invalid size (must be compile-time constant)
            in_function("let n = 5; let arr: [felt; n];"),
        ]
    }
}

#[test]
fn test_array_type_compatibility() {
    assert_semantic_parameterized! {
        ok: [
            // Same type and size
            in_function("
                let arr1: [felt; 3] = [1, 2, 3];
                let arr2: [felt; 3] = arr1;
            "),

            // Function argument compatibility
            r#"
                fn process(data: [felt; 3]) -> felt {
                    return data[0];
                }
                fn test() {
                    let arr = [1, 2, 3];
                    let result = process(arr);
                    return;
                }
            "#,
        ],
        err: [
            // Different sizes
            in_function("
                let arr1: [felt; 3] = [1, 2, 3];
                let arr2: [felt; 2] = arr1;  // Size mismatch
            "),

            // Different element types
            in_function("
                let arr1: [felt; 3] = [1, 2, 3];
                let arr2: [u32; 3] = arr1;  // Type mismatch
            "),

            in_function("
                let arr1: [bool; 2] = [true, false];
                let arr2: [felt; 2] = arr1;  // Type mismatch
            "),

            // Function argument type mismatch
                r#"
                fn process(data: [u32; 3]) -> felt {
                    return 0;
                }
                fn test() {
                    let arr: [felt; 3] = [1, 2, 3];
                    let result = process(arr);  // Type mismatch
                    return;
                }
            "#,

            // Function argument size mismatch
            r#"
                fn process(data: [felt; 3]) -> felt {
                    return 0;
                }
                fn test() {
                    let arr: [felt; 4] = [1, 2, 3, 4];
                    let result = process(arr);  // Size mismatch
                    return;
                }
            "#,
        ]
    }
}
