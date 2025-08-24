//! Tests for array indexing operations and bounds checking
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_array_indexing_bounds() {
    assert_semantic_parameterized! {
        ok: [
            // Valid constant index access
            in_function("let arr = [10, 20, 30]; let x = arr[0];"),
            in_function("let arr = [10, 20, 30]; let x = arr[1];"),
            in_function("let arr = [10, 20, 30]; let x = arr[2];"),

            // Single element array
            in_function("let arr = [42]; let x = arr[0];"),

            // Indexing with variable (dynamic indexing)
            in_function("
                let arr = [10, 20, 30];
                let idx = 1;
                let x = arr[idx];
            "),

            // Indexing in expressions
            in_function("let arr = [10, 20, 30]; let x = arr[0] + arr[1];"),
            in_function("let arr = [10, 20, 30]; let x = arr[1] * 2;"),

            // Indexing with u32 and felt
            in_function("let arr = [10, 20, 30]; let x = arr[1u32];"),
            in_function("let arr = [10, 20, 30]; let x = arr[1felt];"),
        ],
        err: [
            // Out of bounds - constant indices
            in_function("let arr = [10, 20, 30]; let x = arr[3];"),

            // Indexing non-array types
            in_function("let x = 42; let y = x[0];"),
            in_function("let x = true; let y = x[0];"),
            in_function("let t = (1, 2, 3); let x = t[0];"), // Tuples use dot notation

            // Invalid index type
            in_function("let arr = [10, 20, 30]; let x = arr[true];"),

            // Negative indices (if we detect them)
            // in_function("let arr = [10, 20, 30]; let x = arr[-1];"),
        ]
    }
}

#[test]
fn test_array_indexing_type_propagation() {
    assert_semantic_parameterized! {
        ok: [
            // Type inference through indexing
            in_function("
                let arr: [u32; 3] = [1, 2, 3];
                let x = arr[0];  // x should be u32
                let y: u32 = x;
            "),

            in_function("
                let arr: [bool; 2] = [true, false];
                let b = arr[0];  // b should be bool
                if b {
                    let x = 1;
                }
            "),

            // Chained array access (if we support multi-dimensional in future)
            // For now, this should fail since we don't support nested arrays
        ],
        err: [
            // Type mismatch after indexing
            in_function("
                let arr: [u32; 3] = [1, 2, 3];
                let x = arr[0];
                let y: bool = x;  // Type error: u32 != bool
            "),
        ]
    }
}

#[test]
fn test_compile_time_bounds_checking() {
    assert_semantic_parameterized! {
        ok: [
            // Valid compile-time constant indices
            in_function("
                const SIZE = 3;
                let arr = [1, 2, 3];
                let x = arr[0];
                let y = arr[1];
                let z = arr[2];
            "),

            // Indexing with a variable, even with a constant out of bounds works (might cause runtime issues!)
            in_function("
                const SIZE = 3;
                let arr = [1, 2, 3];
                let x = arr[SIZE];
                "),

            // Indexing with an expression does not trigger bounds checking.
            in_function("
                let arr = [1, 2, 3, 4, 5];
                let x = arr[2 + 3];
            "),
        ],
        err: [
            // Invalid compile-time constant indices
            in_function("
                const SIZE = 3;
                let arr = [1, 2, 3];
                let x = arr[3];  // Out of bounds
            "),

            // Compile-time expression evaluation
        ]
    }
}
