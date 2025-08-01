# Parser Test Infrastructure

This directory contains the test suite for the Cairo-M parser. The tests are
organized to ensure comprehensive coverage of all parser features while
maintaining clarity and ease of maintenance.

## Test Organization

Tests are organized into modules based on the parser features they test:

- `expressions.rs` - Expression parsing (literals, operators, function calls)
- `statements.rs` - Statement parsing (let bindings, control flow)
- `toplevel.rs` - Top-level items (functions, structs, imports)
- `types.rs` - Type parsing and validation
- `integration.rs` - File-based tests for complete programs
- `macro_tests.rs` - Tests for the test infrastructure itself

## Test Macros

The test suite provides several macros for different testing patterns:

### Basic Assertions

```rust
// Assert that code parses successfully
assert_parses_ok!("fn test() {}");

// Assert that code fails to parse
assert_parses_err!("fn test(");

// Wrap statements in a function (since statements aren't top-level)
assert_parses_ok!(&in_function("let x = 42;"));
```

### Parameterized Testing

Test multiple inputs with a single test function:

```rust
#[test]
fn binary_operations() {
    assert_parses_parameterized! {
        ok: [
            in_function("a + b;"),
            in_function("a - b;"),
            in_function("a * b;"),
        ],
        err: [
            in_function("a +;"),
            in_function("+ b;"),
        ]
    }
}
```

You can also test only successful or only failing cases:

```rust
assert_parses_parameterized! {
    ok: ["fn test() {}", "fn main() {}"]
}

assert_parses_parameterized! {
    err: ["fn test(", "let x = ;"]
}
```

### File-Based Testing

Parse all files matching a pattern in a directory:

```rust
#[test]
fn test_cases_files() {
    // Parse all .cm files in test_cases/
    assert_parses_files!("../test_cases");

    // Parse with custom pattern
    assert_parses_files!("../test_cases", "*.cm");

    // Parse specific subdirectory
    assert_parses_files!("../test_cases/control_flow");
}
```

## Snapshot Testing

All tests use `insta` for snapshot testing. This captures the exact AST
structure or diagnostic output and compares it against saved snapshots.

### Working with Snapshots

When tests fail due to changed output:

```bash
# Review snapshot changes interactively
cargo insta review

# Accept all snapshot changes
cargo insta accept

# Run tests and review inline
cargo test && cargo insta review
```

## Adding New Tests

### For a New Language Feature

1. Identify the appropriate test module (or create a new one)
2. Add positive test cases showing valid syntax
3. Add negative test cases showing invalid syntax
4. Use parameterized tests when testing variations of the same pattern
5. Run tests and review snapshots

Example:

```rust
// In statements.rs
#[test]
fn while_loop() {
    assert_parses_ok!(&in_function("while x < 10 { x = x + 1; }"));
}

#[test]
fn while_loop_variations() {
    assert_parses_parameterized! {
        ok: [
            &in_function("while true { }"),
            &in_function("while x < 10 { break; }"),
            &in_function("while let Some(x) = iter.next() { }"),
        ],
        err: [
            &in_function("while { }"),
            &in_function("while x < 10"),
        ]
    }
}
```

### For Complete Programs

Add `.cm` files to `test_cases/` directory. These are automatically tested by
the integration tests. Organize them in subdirectories by feature:

```bash
test_cases/
  control_flow/
    loops.cm
    conditionals.cm
  functions/
    recursion.cm
    closures.cm
```

## Best Practices

1. **Test Organization**: Keep related tests together. Use clear test names that
   describe what's being tested.

2. **Coverage**: For each parser rule, include:
   - Basic positive cases
   - Edge cases
   - Error cases with helpful diagnostics

3. **Parameterized Tests**: Use these when testing multiple variations of the
   same pattern to reduce boilerplate.

4. **File Tests**: Use for integration testing of complete features or
   real-world code examples.

5. **Snapshot Names**: The macros automatically generate meaningful snapshot
   names. Don't override unless necessary.

6. **Error Testing**: When testing parse errors, ensure the diagnostics are
   helpful to users.

## Running Tests

```bash
# Run all parser tests
cargo test -p cairo-m-compiler-parser

# Run specific test module
cargo test -p cairo-m-compiler-parser expressions

# Run with output
cargo test -p cairo-m-compiler-parser -- --nocapture

# Run a specific test
cargo test -p cairo-m-compiler-parser test_name
```

## Debugging Failed Tests

1. Check the snapshot diff to understand what changed
2. Use `--nocapture` to see debug output
3. Add `dbg!()` statements in the parser code if needed
4. Use `cargo expand` to see macro expansions if debugging test macros
