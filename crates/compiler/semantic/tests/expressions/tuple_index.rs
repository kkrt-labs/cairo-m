//! Tests for tuple index expressions
use crate::{assert_semantic_parameterized, in_function};

#[test]
fn test_tuple_index_expressions() {
    assert_semantic_parameterized! {
        ok: [
            // In range - basic cases
            in_function("let t = (10, 20, 30); let x = t.0;"),
            in_function("let t = (10, 20, 30); let x = t.1;"),
            in_function("let t = (10, 20, 30); let x = t.2;"),

            // Single element tuple
            in_function("let t = (42,); let x = t.0;"),

            // Nested tuples
            in_function("let t = ((1, 2), (3, 4)); let x = t.0;"),
            in_function("let t = ((1, 2), (3, 4)); let x = t.0.1;"),

            // Different types in tuple
            in_function("let t = (42, true); let x = t.0;"),
            in_function("let t = (42, true); let b = t.1;"),

            // Large index numbers (still valid)
            in_function("let t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10); let x = t.9;"),
            ],
            err: [
            // Out of range
            in_function("let t = (10, 20, 30); let x = t.3;"),
            in_function("let t = (10, 20, 30); let x = t.4;"),
            in_function("let t = (10, 20, 30); let x = t[3];"),

            // Single element tuple - out of range
            in_function("let t = (42,); let x = t.1;"),

            // Empty tuple - any index should fail
            in_function("let t = (); let x = t.0;"),

            // Invalid index access (subscript operator)
            in_function("let t = (10, 20, 30); let x = t[0];"),

            // Negative indices (if numeric literals)
            in_function("let t = (10, 20, 30); let x = t.-1;"),

            // Non-tuple types with dot access
            in_function("let x = 42; let y = x.0;"),
            in_function("let x = true; let y = x.0;"),

            // Nested access out of range
            in_function("let t = ((1, 2), (3, 4)); let x = t.0.2;"),
            in_function("let t = ((1, 2), (3, 4)); let x = t.2;"),
        ]
    }
}
