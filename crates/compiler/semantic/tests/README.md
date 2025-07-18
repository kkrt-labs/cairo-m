# Semantic Test Infrastructure

This directory contains the comprehensive test suite for the Cairo-M semantic
analyzer. The tests are organized to ensure thorough coverage of all semantic
validation features while maintaining clarity and ease of maintenance.

## Test Organization

The test suite is organized by semantic concern into different modules:

- **`scoping/`** - Variable scoping, visibility, and declaration tests.
- **`control_flow/`** - Control flow analysis tests.
- **`functions/`** - Function-related validation tests.
- **`statements/`** - Statement-level validation tests.
- **`types/`** - Type system validation tests.
- **`structures/`** - Struct validation tests.
- **`expressions/`** - Expression validation tests.
- **`integration/`** - End-to-end integration tests.
- **`semantic_model/`** - White-box tests for internal semantic model
  consistency.

This structure makes it easy to locate tests for a specific language feature and
to identify areas that may need more test coverage.

## Test Infrastructure

### Basic Assertions

The test suite provides macros for simple, inline semantic validation tests:

```rust
// Assert that code validates successfully
assert_semantic_ok!("fn test() { let x = 42; }");

// Assert that code produces validation errors
assert_semantic_err!("fn test() { let x = undefined; }");

// Show unused variable warnings (hidden by default)
assert_semantic_ok!("fn test() { let x = 42; }", show_unused);
assert_semantic_err!("fn test() { let x = y; }", show_unused);
```

By default, unused variable warnings are ignored by the assertion macros to
allow tests to focus on other errors. The `show_unused` flag can be used to make
these warnings visible to the assertions.

### Parameterized Testing

For testing multiple similar scenarios, the `assert_semantic_parameterized!`
macro is provided. This is the preferred way to write most tests as it reduces
boilerplate and groups related cases.

```rust
#[test]
fn test_variable_declarations() {
    assert_semantic_parameterized! {
        ok: [
            "fn test() { let x = 42; }",
            "fn test() { let x: felt = 42; }",
            "fn test(y: felt) { let x = y; }",
        ],
        err: [
            "fn test() { let x = undefined; }",
            "fn test() { let x: u32 = 42; }",  // Type mismatch
            "fn test() { x = 42; }",  // Undeclared variable
        ]
    }
}
```

You can also test only successful or only failing cases:

```rust
assert_semantic_parameterized! {
    ok: ["fn test() {}", "fn main() { let x = 1; }"]
}

assert_semantic_parameterized! {
    err: ["fn test() { undefined; }", "fn test() { x = 1; }"]
}
```

### Snapshot Testing

For complex scenarios where the exact diagnostic output is important, snapshot
tests can be used:

- `assert_diagnostics_snapshot!(file, name)` - Snapshot test for `.cm` files.
  While most tests have been migrated to be inline, this can still be useful for
  very large or complex integration tests.

### Helper Functions

A set of helper functions is available to reduce boilerplate in test cases by
wrapping code snippets in common structures like functions or structs.

- `in_function(code)` - Wrap statement code inside a function.
- `with_struct(name, fields, code)` - Add a struct definition before the main
  code.
- `with_functions(functions, main_code)` - Add helper functions before the main
  code.

### Example Usage

```rust
#[test]
fn test_let_statement() {
    // Simple inline test - this passes because `x` is used.
    assert_semantic_ok!(in_function("let x = 42; return x;"));
}

#[test]
fn test_unused_variable_error() {
    // To test for an unused variable warning, use `show_unused`.
    // This will fail if the warning is not produced.
    assert_semantic_err!(in_function("let x = 42; return ();"), show_unused);
}
```

## Benefits of the Current Structure

1.  **Clear Organization**: Easy to find tests related to a specific feature.
2.  **Reduced Boilerplate**: Parameterized tests and helper functions keep tests
    concise.
3.  **Focused Testing**: Unused variable warnings can be ignored to focus on
    specific errors.
4.  **High Maintainability**: Inline tests are self-contained and easy to read,
    modify, and debug.
5.  **Comprehensive Coverage**: A systematic, concern-oriented organization
    helps ensure all language features are tested thoroughly.

## Adding New Tests

1.  **Identify the appropriate test module** (e.g.,
    `expressions/binary_expressions.rs`).
2.  **Add a new test function** or extend an existing one.
3.  **Use `assert_semantic_parameterized!`** to add new valid (`ok`) and invalid
    (`err`) test cases.
4.  **Keep test cases small and focused** on a single semantic rule.

## Running Tests

```bash
# Run all semantic tests
cargo test -p cairo-m-compiler-semantic

# Run a specific test module
cargo test -p cairo-m-compiler-semantic --test expressions

# Run a specific test function
cargo test -p cairo-m-compiler-semantic --test expressions -- test_binary_operator_type_errors

# Run with output
cargo test -p cairo-m-compiler-semantic -- --nocapture

# Review snapshot changes
cargo insta review

# Accept all snapshot changes
cargo insta accept
```
