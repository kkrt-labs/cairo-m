# Cairo M Compiler

The Cairo M Compiler is responsible for parsing and compiling Cairo code into
Cairo Assembly for execution in the Cairo M virtual machine.

## Overview

The compiler consists of several components and in this crate are implemented:

- **Lexer**: Tokenizes Cairo source code into a stream of tokens
- **Parser**: Parses the token stream into an Abstract Syntax Tree (AST)

## Testing with Insta

The compiler uses [insta](https://insta.rs/) for snapshot testing, which allows
us to capture the output of parser operations and automatically compare them
against saved snapshots. This approach is particularly useful for testing
parsers because it lets us verify the exact structure of ASTs and error
messages.

### What is Snapshot Testing?

Snapshot testing captures the output of your code and saves it to a file (a
"snapshot"). On subsequent test runs, the output is compared against the saved
snapshot. If they differ, the test fails and you can review the changes.

### Running Insta Tests

To run the insta-based tests for the parser:

```bash
# Run all tests in the compiler crate
cargo test -p cairo-m-compiler-parser

# Run only parser tests
cargo test -p cairo-m-compiler-parser parser::tests

# Run a specific test
cargo test -p cairo-m-compiler-parser test_simple_let_declaration
```

### Working with Snapshots

#### Reviewing Failed Snapshots

When a test fails because the output doesn't match the snapshot, insta will show
you a diff.

Install [cargo-insta](https://insta.rs/docs/cli/) if you haven't already

You can review and accept changes using:

```bash
# Review all pending snapshots
cargo insta review

# Accept all changes (use with caution)
cargo insta accept

# Review changes for a specific crate
cargo insta review -p cairo-m-compiler-parser
```

#### Creating New Tests

To add a new parser test with snapshot testing:

1. Add a test function in `parser/src/parser.rs`:

```rust
#[test]
fn test_my_new_feature() {
    assert_parse_snapshot!("let x = 42;", "my_new_feature");
}
```

2. Run the test - it will fail initially because no snapshot exists:

```bash
cargo test test_my_new_feature
```

3. Review and accept the new snapshot:

```bash
cargo insta review
```

### Test Macro: `assert_parse_snapshot!`

The parser tests use a custom macro `assert_parse_snapshot!` that:

1. Parses the input string using the parser
2. Creates snapshots for both successful parse results and parsing errors
3. Handles error formatting with nice diagnostic messages using `ariadne`

Usage patterns:

```rust
// Simple usage - snapshot name will be the input string
assert_parse_snapshot!("let x = 3;");

// Custom snapshot name
assert_parse_snapshot!("let x = 3;", "simple_let_declaration");
```

### Snapshot Files

Snapshots are stored in `parser/src/snapshots/` with names following the
pattern:

```text
{crate_name}__{module}__{test_name}.snap
```

For example:

- `cairo_m_compiler_parser__parser__tests__simple_let_error.snap`

### Error Snapshots

When parsing fails, the macro creates formatted error messages using `ariadne`
for beautiful error reporting. These error snapshots help ensure that:

1. Error messages are helpful and consistent
2. Error reporting doesn't regress over time
3. Error positions are accurate

### Best Practices

1. **Descriptive Test Names**: Use clear, descriptive names for your tests and
   snapshots
2. **Small, Focused Tests**: Each test should focus on a specific parsing
   scenario
3. **Review Changes Carefully**: Always review snapshot changes to ensure
   they're intentional
4. **Version Control**: Commit snapshot files alongside your code changes

### Example Test Structure

```rust
#[test]
fn test_function_declaration() {
    assert_parse_snapshot!(
        "fn add(x: felt, y: felt) -> felt { return x + y; }",
        "function_declaration"
    );
}

#[test]
fn test_invalid_syntax() {
    // This will capture the error message in a snapshot
    assert_parse_snapshot!(
        "let x = ;", // Missing value
        "missing_value_error"
    );
}
```

## Development Workflow

1. Write your test with `assert_parse_snapshot!`
2. Run the test (it will fail initially)
3. Use `cargo insta review` to examine the generated snapshot
4. Accept the snapshot if it looks correct
5. Commit both your code and the snapshot file

This workflow ensures that any changes to parser behavior are intentional and
properly reviewed.
