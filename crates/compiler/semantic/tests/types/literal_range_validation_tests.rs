use super::in_function;

#[test]
fn test_felt_literals() {
    assert_semantic_parameterized! {
        ok: [
            // Boundary
            in_function("let x: felt = 2147483647;"),
            in_function("const MAX_FELT = 2147483647;"),

            ],
            err: [
            // Overflow
            in_function("let x: felt = 2147483648;"),

            // Negative - we don't support negative felt literals yet
            in_function("let x: felt = -1;"),
            in_function("const MIN_FELT = -2147483648;"),
        ]
    }
}

#[test]
fn test_u32_literals() {
    assert_semantic_parameterized! {
        ok: [
            // Boundary
            in_function("let x: u32 = 4294967295;"),
            in_function("const MAX_U32 = 4294967295u32;"),
        ],
        err: [
            // Overflow
            in_function("let x: u32 = 4294967296;"),
        ]
    }
}
