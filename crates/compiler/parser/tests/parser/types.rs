use crate::assert_parses_ok;

// Helper to wrap type expressions in function parameters for testing
fn with_param(type_expr: &str) -> String {
    format!("func test(x: {type_expr}) {{ }}")
}

// ===================
// Basic Types
// ===================

#[test]
fn named_type() {
    assert_parses_ok!(&with_param("felt"));
}

#[test]
fn custom_type() {
    assert_parses_ok!(&with_param("MyStruct"));
}

// ===================
// Pointer Types
// ===================

#[test]
fn pointer_type() {
    assert_parses_ok!(&with_param("felt*"));
}

#[test]
fn nested_pointer() {
    assert_parses_ok!(&with_param("felt**"));
}

#[test]
fn pointer_to_custom_type() {
    assert_parses_ok!(&with_param("MyStruct*"));
}

// ===================
// Tuple Types
// ===================

#[test]
fn tuple_type() {
    assert_parses_ok!(&with_param("(felt, felt)"));
}

#[test]
fn nested_tuple_type() {
    assert_parses_ok!(&with_param("((felt, felt), felt)"));
}

#[test]
fn complex_tuple_type() {
    assert_parses_ok!(&with_param("(felt, felt*, (felt, felt))"));
}

#[test]
fn single_element_tuple_type() {
    assert_parses_ok!(&with_param("(felt,)"));
}

// ===================
// Complex Type Combinations
// ===================

#[test]
fn pointer_to_tuple() {
    assert_parses_ok!(&with_param("(felt, felt)*"));
}

#[test]
fn tuple_of_pointers() {
    assert_parses_ok!(&with_param("(felt*, felt*)"));
}

#[test]
fn deeply_nested_types() {
    assert_parses_ok!(&with_param("((felt*, felt), (felt, felt*))*"));
}
