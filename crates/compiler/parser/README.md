# Cairo-M Parser Testing Framework

This document describes the comprehensive testing framework for the Cairo-M
parser, which provides granular testing capabilities similar to the Cairo
compiler's testing approach.

## Overview

The testing framework supports multiple testing modes and provides extensive
coverage for all language constructs. It uses `insta` for snapshot testing,
allowing us to capture and verify both successful parse results and diagnostic
messages for malformed code.

## Test Architecture

### Test Modes

The framework supports three testing modes:

1. **Parse Mode** (`mode: parse`): Tests successful parsing and snapshots the
   resulting AST
2. **Diagnostics Mode** (`mode: diagnostics`): Tests parsing errors and
   snapshots diagnostic messages
3. **Both Mode** (`mode: both`): Tests both successful parsing and error cases

### Test Case Structure

Tests are defined using the `test_case!` macro:

```rust
test_case!(
    name: "test_name",
    code: "cairo_code_here",
    mode: parse,
    construct: "optional_construct_tag"
)
```

### Test Organization

Tests are organized into logical groups:

## Test Categories

### 1. Expression Tests

Comprehensive testing of all expression types:

- **Literals**: Integer literals, edge cases like zero and max values
- **Identifiers**: Simple and complex identifier names
- **Binary Operations**: All operators with proper precedence testing
- **Function Calls**: Simple calls, calls with arguments, chained calls
- **Member Access**: Simple and nested member access
- **Index Access**: Array indexing, nested indexing
- **Struct Literals**: Simple, nested, and empty struct literals
- **Tuples**: Simple tuples, nested tuples, parenthesized expressions

### 2. Type Expression Tests

Testing of type annotations:

- **Named Types**: Basic type names like `felt`
- **Pointer Types**: Single and multiple pointer levels (`felt*`, `felt**`)
- **Tuple Types**: Type tuples like `(felt, felt)`
- **Complex Types**: Nested combinations

### 3. Statement Tests

All statement types with variations:

- **Let Statements**: Simple declarations and complex expressions
- **Local Statements**: With and without type annotations
- **Const Statements**: Simple constants and computed values
- **Assignments**: Variable, member, and index assignments
- **Return Statements**: With and without values
- **If Statements**: Simple if, if-else, nested conditionals
- **Blocks**: Simple and nested blocks

### 4. Top-Level Item Tests

Complete coverage of program structure:

- **Function Definitions**: Various parameter and return type combinations
- **Struct Definitions**: Simple, empty, and complex structs
- **Namespace Definitions**: Simple, nested, and mixed content namespaces
- **Import Statements**: Simple imports, imports with aliases, nested paths
- **Const Definitions**: Top-level constants with expressions

### 5. Diagnostic Tests

Comprehensive error testing:

- **Expression Diagnostics**: Missing semicolons, invalid operators, syntax
  errors
- **Function Diagnostics**: Missing names, invalid parameters, missing bodies
- **Struct Diagnostics**: Missing names, invalid field definitions
- **Statement Diagnostics**: Invalid control flow, missing targets
- **Import Diagnostics**: Invalid syntax, empty paths

### 6. Integration Tests

Real-world program testing:

- **Complete Programs**: Full programs with multiple constructs
- **Complex Expressions**: Intricate precedence and associativity testing
- **Mixed Content**: Programs combining all language features

### 7. Edge Case Tests

Boundary condition testing:

- **Empty Programs**: Empty and whitespace-only inputs
- **Deep Nesting**: Testing parser stack depth limits
- **Large Numbers**: Maximum value testing
- **Long Identifiers**: Stress testing identifier parsing
- **Complex Structures**: Multi-field structs and many-parameter functions

### 8. Regression Tests

Tests for previously problematic cases:

- **Trailing Commas**: Various contexts where trailing commas are allowed
- **Operator Associativity**: Left-associative operators like subtraction and
  division
- **Chained Operations**: Complex method chaining and member access
- **Tuple Disambiguation**: Single-element tuples vs parenthesized expressions

### 9. Boundary Tests

Testing parser limits:

- **Deep Nesting**: Maximum nesting levels for control structures
- **Large Numbers**: Edge cases for numeric literals
- **Precedence Chains**: Complex operator precedence combinations

### 10. Error Recovery Tests

Testing parser resilience:

- **Multiple Errors**: Programs with multiple syntax errors
- **Mixed Valid/Invalid**: Programs with both correct and incorrect syntax

## Specialized Test Modules

### Expression Tests Module

Focused testing of expression parsing:

- Literal expression edge cases
- Operator associativity verification
- Precedence rule validation

### Statement Tests Module

Detailed statement parsing verification:

- Control flow variations
- Variable declaration patterns
- Complex statement combinations

### Top-Level Tests Module

Program structure testing:

- Function definition variations
- Namespace organization patterns
- Import statement formats

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Categories

```bash
# Expression tests only
cargo test test_literals
cargo test test_binary_operations

# Diagnostic tests only
cargo test test_expression_diagnostics
cargo test test_function_diagnostics

# Edge cases
cargo test test_edge_cases
cargo test test_boundary_conditions
```

### Updating Snapshots

When parser output changes, update snapshots:

```bash
cargo insta review
```

## Test Naming Conventions

- Test function names: `test_<category>`
- Snapshot names: `<construct>_<test_name>` for successful parses
- Diagnostic snapshots: `<test_name>_diagnostic` for error cases
- Multiple diagnostics: `<test_name>_diagnostic_<index>`

## Adding New Tests

1. Identify the construct type (expression, statement, etc.)
2. Choose appropriate test mode (parse, diagnostics, both)
3. Use the `test_case!` macro with descriptive name
4. Add to appropriate test function
5. Run `cargo test` to generate snapshots
6. Review snapshots with `cargo insta review`

## Snapshot Organization

Snapshots are organized by:

- Construct type (expression, statement, function, etc.)
- Test name
- Whether it's a successful parse or diagnostic

Each snapshot contains both the source code and the result, making manual review
easy:

```rust
SnapshotEntry {
    code: "func add(a: felt, b: felt) -> felt { return a + b; }",
    result: ParseSuccess(
        [Function(FunctionDef { ... })]
    ),
}
```

For diagnostic snapshots:

```rust
SnapshotEntry {
    code: "let x = 5",
    result: ParseError(
        "[03] Error: found end of input expected ';'..."
    ),
}
```

This provides clear organization and makes it easy to understand what each
snapshot represents.

## Benefits

This testing approach provides:

1. **Comprehensive Coverage**: Every language construct is tested
2. **Regression Protection**: Changes that break parsing are immediately
   detected
3. **Documentation**: Tests serve as examples of valid Cairo-M syntax
4. **Diagnostic Quality**: Error messages are verified to be helpful
5. **Maintenance**: Easy to add new tests and update existing ones

The framework ensures the parser remains robust and provides excellent error
messages as the language evolves.
