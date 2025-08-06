# Cairo-M Markdown Testing System

This directory contains markdown-based tests for the Cairo-M compiler and
runner. These tests serve dual purposes:

1. **Documentation**: Provide clear examples of Cairo-M language features
2. **Testing**: Automatically verify language behavior through differential
   testing

## Structure

Tests are organized by language feature category:

- `01-basics/`: Fundamental language constructs (literals, variables, functions,
  types, arithmetic)
- `02-control-flow/`: Control flow statements (if-else, loops, pattern matching)
- `03-types/`: Type system features (arrays, structs, tuples)
- `04-advanced/`: Advanced features (recursion, memory operations)
- `05-edge-cases/`: Boundary conditions and edge cases

### Naming Convention

Files are prefixed with numbers (e.g., `01-literals.md`) for logical ordering.
Since Rust module names cannot start with digits, the build system automatically
prefixes these with `m_` when generating test modules (e.g., `m_01_literals`).

## Test Format

Each markdown file follows this structure:

````markdown
# Feature Category

Optional configuration in TOML:

```toml
[config]
key = "value"
```
````

## Test Case Name

Description of what this test demonstrates.

```cairo-m
//! ignore: reason (optional - marks test as ignored)
//! expected: value (optional - expected output)
//! error: "message" (optional - expected error)
// Cairo-M code here
fn main() {
    // test code
}
```

```rust
// Optional Rust equivalent for differential testing
fn main() {
    // equivalent rust code
}
```

````

## Running Tests

Tests are automatically discovered and executed as individual Rust test functions:

```bash
# Run all mdtest tests
cargo test -p cairo-m-runner --test mdtest_generated

# Run specific test file
cargo test -p cairo-m-runner --test mdtest_generated m_01_basics

# Run individual test
cargo test -p cairo-m-runner --test mdtest_generated m_01_basics_m_01_literals::integer_literals
````

## How It Works

1. **Build-time Generation**: The `build.rs` script discovers all markdown files
   and generates individual test functions
2. **Test Extraction**: The parser extracts Cairo-M and optional Rust code
   blocks from markdown
3. **Differential Testing**: Each test compiles and runs both Cairo-M and Rust
   implementations, comparing outputs
4. **Automatic Discovery**: No manual test registration required - just add
   markdown files

## Adding New Tests

1. Create a markdown file in the appropriate category directory
2. Follow the test format above
3. Tests are automatically discovered on next build
4. Use `//! ignore: reason` to mark tests that aren't ready yet

## Test Annotations

- `//! expected: value` - Specify expected output value
- `//! error: "message"` - Test should produce this error
- `//! ignore: reason` - Skip this test with given reason
- `//! tags: [tag1, tag2]` - Tag tests for categorization
- `//! rust-equiv: name` - Reference to equivalent Rust function

## Implementation Details

The test infrastructure consists of:

- `crates/test_utils/src/mdtest/`: Core parsing and test extraction
- `crates/runner/build.rs`: Build script for test generation
- `crates/runner/tests/mdtest_generated.rs`: Entry point for generated tests
- `crates/runner/tests/common/`: Shared test utilities
