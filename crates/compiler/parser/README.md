# Cairo M Compiler Parser

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

1. Add a test function in `parser/tests/parser`:

2. Run the test - it will fail initially because no snapshot exists:

```bash
cargo test test_my_new_feature
```

3. Review and accept the new snapshot:

```bash
cargo insta review
```
