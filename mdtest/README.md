# Cairo-M Markdown Testing System

This directory contains markdown-based tests for the Cairo-M compiler and
runner. These tests serve multiple purposes:

1. **Documentation**: Provide clear examples of Cairo-M language features
2. **Differential Testing**: Verify Cairo-M behavior against Rust equivalents
3. **Snapshot Testing**: Generate snapshots for MIR and Codegen compilation
   phases
4. **Regression Testing**: Ensure language features work correctly across
   compiler updates

## Structure

Tests are organized by language feature category:

- `01-basics/`: Fundamental language constructs (literals, variables, functions,
  types, arithmetic, arrays, expressions)
- `02-control-flow/`: Control flow statements (if-else, loops, pattern matching)
- `03-types/`: Type system features (tuples, structs)
- `04-advanced/`: Advanced features (recursion, multiple functions, mutual
  recursion, optimization)
- `05-edge-cases/`: Boundary conditions and error handling
- `06-internals/`: Low-level features (opcodes, instructions)

### Naming Convention

Files are prefixed with numbers (e.g., `01-literals.md`) for logical ordering.
Since Rust module names cannot start with digits, the build system automatically
prefixes these with `m_` when generating test modules (e.g., `m_01_literals`).

## Test Format

Each markdown file follows this hierarchical structure:

````markdown
# H1: Feature Category (e.g., "Literals in Cairo-M")

Optional global configuration:

```toml
[config]
key = "value"
```

## H2: Test Section (e.g., "Integer Literals")

Description of what this section demonstrates.

### H3: Specific Test Case (optional, e.g., "Positive Numbers")

More specific test description.

```cairo-m
//! ignore: reason (optional - marks test as ignored)
//! expected: value (optional - expected output value)
//! error: "message" (optional - expected error message)
//! tags: [tag1, tag2] (optional - test categorization)
//! rust-equiv: name (optional - reference to equivalent Rust function)

// Cairo-M code here
fn main() -> felt {
    return 42;
}
```

```rust
// Optional Rust equivalent for differential testing
fn main() -> i64 {
    42
}
```
````

### Multiple Tests Per Section

You can have multiple Cairo-M code blocks in the same section. Each will create
a separate test:

````markdown
## Basic Operations

First test:

```cairo-m
fn compute() -> felt { return 5; }
```

Second test:

```cairo-m
fn calculate() -> felt { return 10; }
```
````

Tests in the same section are automatically numbered: "Basic Operations" and
"Basic Operations 2".

## Test Annotations

Tests can be annotated with special comments starting with `//!`:

| Annotation               | Description                           | Example                                 |
| ------------------------ | ------------------------------------- | --------------------------------------- |
| `//! ignore: reason`     | Skip test execution with reason       | `//! ignore: U32Eq not implemented yet` |
| `//! expected: value`    | Specify expected output value         | `//! expected: 42`                      |
| `//! error: "message"`   | Test should produce this error        | `//! error: "Division by zero"`         |
| `//! tags: [tag1, tag2]` | Tag tests for categorization          | `//! tags: [arithmetic, optimization]`  |
| `//! rust-equiv: name`   | Reference to Rust equivalent function | `//! rust-equiv: compute_sum`           |

## Running Tests

### Runner Tests (Differential Testing)

Tests are compiled to Rust test functions and can be executed:

```bash
# Run all mdtest tests against a Rust impl
cargo test -p cairo-m-runner --test mdtest_generated

# Run specific test file (module)
cargo test -p cairo-m-runner --test mdtest_generated m_01_basics_m_01_literals

# Run individual test (function)
cargo test -p cairo-m-runner --test mdtest_generated m_01_basics_m_01_literals::integer_literals

# List all available tests
cargo test -p cairo-m-runner --test mdtest_generated -- --list
```

### Snapshot Tests (MIR and Codegen)

MDTests are also used for snapshot testing of compiler phases:

```bash
# Run MIR snapshot tests
cargo test -p cairo-m-compiler-mir --test mdtest_snapshots

# Run Codegen snapshot tests
cargo test -p cairo-m-compiler-codegen --test mdtest_snapshots

# Review snapshot changes
cargo insta review

# Accept all snapshot changes
cargo insta accept
```

## How It Works

### 1. Build-Time Test Generation

The system uses a multi-stage process to convert markdown files into executable
tests:

#### For Runner Tests (`crates/runner/build.rs`)

1. **Discovery**: Walks the `mdtest/` directory to find all `.md` files
2. **Parsing**: Uses `mdtest::parser::extract_tests()` to extract Cairo-M and
   Rust code blocks
3. **Module Generation**: Creates Rust modules with sanitized names (e.g.,
   `01-basics` → `m_01_basics`)
4. **Function Generation**: Creates test functions with unique names, handling
   duplicates by appending numbers
5. **Test Lookup**: Each test function calls `get_test_by_name()` with the
   unique test name

#### For Snapshot Tests (MIR/Codegen)

1. Uses the same parser to extract Cairo-M code
2. Compiles through the respective phase (MIR generation or Codegen)
3. Captures output as snapshots using the `insta` crate
4. Compares against previous snapshots for regression detection

### 2. Test Name Generation

Test names are hierarchically constructed from markdown headings:

- **H1 only**: `"Feature Category"`
- **H1 + H2**: `"Feature Category - Test Section"`
- **H1 + H2 + H3**: `"Feature Category - Test Section - Specific Test"`

When multiple tests exist in the same section, they're numbered:

- First test: `"Feature Category - Test Section"`
- Second test: `"Feature Category - Test Section 2"`
- Third test: `"Feature Category - Test Section 3"`

The test function names are derived from the last component (H3 > H2 > H1):

- `test_section()`, `test_section_2()`, `test_section_3()`

### 3. Differential Testing Process

For tests with both Cairo-M and Rust code:

1. **Cairo-M Compilation**: Compiles Cairo-M source using the Cairo-M compiler
2. **Cairo-M Execution**: Runs the compiled program with the Cairo-M VM
3. **Rust Compilation**: Compiles Rust code with `rustc`
4. **Rust Execution**: Runs the Rust binary
5. **Output Comparison**: Compares outputs, failing if they differ

Special handling for `main` functions:

- If the Rust function is named `main`, it's automatically renamed to `main_` to
  avoid conflicts with the test wrapper

### 4. Snapshot Testing Process

For MIR and Codegen phases:

1. **Source Extraction**: Extracts Cairo-M code from markdown
2. **Compilation**: Compiles through the target phase (parsing → semantic → MIR
   → codegen)
3. **Snapshot Capture**: Serializes the output (MIR or assembly code)
4. **Comparison**: Uses `insta` to compare with stored snapshots
5. **Review**: Developers can review and accept changes with
   `cargo insta review`

## Adding New Tests

### 1. Choose the Right Category

Select the appropriate directory based on your test:

- `01-basics/`: Language fundamentals (literals, variables, functions, types)
- `02-control-flow/`: Conditionals and loops
- `03-types/`: Data structures (tuples, structs)
- `04-advanced/`: Complex features (recursion, optimization)
- `05-edge-cases/`: Error cases and boundaries
- `06-internals/`: Low-level implementation details

### 2. Create Your Test File

Create a new `.md` file or add to an existing one:

```bash
# Create a new test file
touch mdtest/01-basics/07-new-feature.md
```

### 3. Write Your Test

#### Simple Test (Cairo-M only)

````markdown
## Simple Addition

Basic addition test:

```cairo-m
fn add_numbers() -> felt {
    return 5 + 3;
}
```
````

If no Rust code is provided, we will auto-generate it based on the Cairo-M code.
If the Rust code cannot be auto-generated, you will need to implement it
manually.

#### Differential Test (with Rust)

````markdown
## Addition with Differential Testing

Test addition with Rust comparison:

```cairo-m
fn add_numbers() -> felt {
    return 5 + 3;
}
```

```rust
fn add_numbers() -> i64 {
    5 + 3
}
```
````

#### Test with Annotations

````markdown
## Expected Output Test

```cairo-m
//! expected: 8
fn add_numbers() -> felt {
    return 5 + 3;
}
```
````

### 5. Run Your Tests

```bash
# Build and run your new test
cargo test -p cairo-m-runner --test mdtest_generated

# If using snapshots, update them
cargo insta review
```

## Implementation Details

The mdtest infrastructure is implemented across several components:

### Core Components

- **Parser** (`crates/test_utils/src/mdtest/parser.rs`):
  - Extracts tests from markdown using `pulldown_cmark`
  - Handles test annotations and metadata
  - Supports multiple tests per section with automatic numbering

- **Runner** (`crates/test_utils/src/mdtest/runner.rs`):
  - Generic test runner for different compilation phases
  - Configurable processor for MIR/Codegen snapshot generation

- **Build Scripts**:
  - `crates/runner/build.rs`: Generates differential test functions

- **Test Utilities** (`crates/runner/tests/common/mod.rs`):
  - Differential testing logic
  - Cairo-M compilation and execution
  - Rust code compilation and execution
  - Output comparison and formatting

### Special Handling

- **Module name prefixing**: Numeric prefixes are converted (e.g., `01-basics` →
  `m_01_basics`)
- **Function name deduplication**: Duplicate names get numeric suffixes (`_2`,
  `_3`, etc.)
- **Main function renaming**: Rust `main` functions are renamed to `main_` to
  avoid conflicts
- **Boolean conversion**: Rust `true`/`false` outputs are converted to `1`/`0`
  for comparison

## Best Practices

1. **Keep tests focused**: Each test should demonstrate one specific feature
2. **Use descriptive names**: Clear H2/H3 headings help identify test purpose
3. **Document edge cases**: Add comments explaining non-obvious behavior
4. **Group related tests**: Use sections to organize related test cases
5. **Prefer differential testing**: Include Rust code when possible for
   validation, or ensure that the cairo code can be "converted" to rust easily.
6. **Use annotations**: Mark incomplete tests with `//! ignore:` rather than
   commenting out
7. **Test both success and failure**: Include tests for error conditions
