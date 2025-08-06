# PRD: Markdown-Based Testing Infrastructure for Cairo-M

## 1. Executive Summary

This document outlines the requirements for a comprehensive markdown-based
testing infrastructure for the Cairo-M compiler and runtime. The system will
combine documentation with executable tests, ensuring all language features are
both documented and verified through automated testing.

## 2. Goals & Objectives

### Primary Goals

1. **Unified Documentation & Testing**: Create a single source of truth where
   language documentation and tests coexist
2. **Feature Coverage Verification**: Ensure 100% of documented language
   features have corresponding tests
3. **Differential Testing**: Compare Cairo-M execution results against Rust
   reference implementations
4. **Developer Experience**: Provide clear, discoverable tests that serve as
   learning materials

### Success Metrics

- All language features documented in markdown files
- 100% of code snippets in documentation are executable tests
- All tests pass in CI/CD pipeline
- Differential tests validate Cairo-M against Rust implementations

## 3. System Architecture

### 3.1 Directory Structure

```
cairo-m/
├── crates/
│   └── test_utils/
│       ├── src/
│       │   ├── lib.rs          # Existing test utilities
│       │   └── mdtest/         # New mdtest module
│       │       ├── mod.rs
│       │       ├── parser.rs   # Markdown parsing
│       │       ├── runner.rs   # Test execution
│       │       └── config.rs   # Configuration types
│       └── Cargo.toml
├── mdtest/                      # Markdown test suites
│   ├── 01-basics/
│   │   ├── 01-literals.md
│   │   ├── 02-variables.md
│   │   ├── 03-functions.md
│   │   └── 03-primitive-types.md
│   ├── 02-control-flow/
│   │   ├── 01-if-else.md
│   │   ├── 02-loops.md
│   │   └── 03-break-continue.md
│   ├── 03-types/
│   │   ├── 01-structs.md
│   │   └── 02-arrays.md
│   ├── 04-advanced/
│   │   ├── 01-recursion.md
│   │   ├── 02-memory-model.md
│   │   └── 03-optimizations.md
│   └── reference/
│       └── complete-reference.md
└── tests/
    └── mdtest.rs                # Test harness
```

### 3.2 Component Overview

#### Cairo-M Test Utils Extension

The existing `cairo-m-test-utils` crate will be extended with mdtest
functionality:

```rust
// crates/test_utils/src/mdtest/mod.rs
pub mod parser;
pub mod runner;
pub mod config;

pub use parser::extract_tests;
pub use runner::{run_test, TestResult};
pub use config::MdTestConfig;
```

## 4. Markdown Test Format Specification

### 4.1 Basic Test Structure

````markdown
# Feature Name

Description of a feature of the language.

```cairo-m
fn compute() -> felt {
    return 40 + 2;
}
```

> Note: we can add an equivalent rust implementation of the function for
> differential testing. This is optional; if not provided, we will run the rust
> code by changing all `felt` occurrences to `i32`.

```rust
fn compute() -> i32 {
    return 40 + 2;
}
```
````

### 4.2 Test Annotations

Tests use comment-based annotations at the start of code blocks:

```cairo-m
//! expected: <value>              # For run tests - if not provided, we will run against a rust code.
//! error: <error-message>          # For compile-fail tests
//! rust-equiv: <path>::<function>  # For differential testing
//! tags: [tag1, tag2]             # For categorization
//! ignore: <reason>               # Skip test with reason
```

## 5. Differential Testing Framework

### 5.1 Rust Equivalence Tests

For each Cairo-M test, an optional Rust implementation can verify correctness.
This is optional; if not provided, we will run the rust code by changing all
`felt` occurrences to `i32`. If provided, we expect its location to be in the
same markdown section, under a ```rust code block, as demonstrated above.

### 5.2 Automatic Differential Testing

The framework will automatically:

1. Parse Cairo-M code from markdown
2. Compile and run it
3. Find matching Rust equivalent.
4. Compare outputs
5. Report discrepancies

## 6. Implementation Requirements

### 6.1 Parser Requirements

```rust
pub struct MdTest {
    pub name: String,
    pub source: String,
    pub files: HashMap<String, String>,
    pub metadata: TestMetadata,
    pub location: Location,
}

pub struct TestMetadata {
    pub test_types: Vec<TestType>,
    pub expected_output: Option<String>,
    pub expected_error: Option<String>,
    pub rust_equiv: Option<String>,
    pub tags: Vec<String>,
    pub ignore: Option<String>,
    pub config: Option<MdTestConfig>,
}

pub fn extract_tests(markdown_path: &Path) -> Result<Vec<MdTest>, Error> {
    // Parse markdown file
    // Extract code blocks with cairo-m language tag
    // Parse annotations from comments
    // Group multi-file tests
    // Return structured test data
}
```

### 6.2 Runner Requirements

```rust
pub enum TestResult {
    Pass,
    Fail(String),
    Ignored(String),
}

pub fn run_test(test: &MdTest) -> TestResult {
    // Based on test_types, run appropriate phases:
    // 1. Parse
    // 2. Semantic analysis
    // 3. Code generation
    // 4. Execution
    // 5. Differential testing if rust_equiv specified
    // Return consolidated result
}
```

### 6.3 Test Discovery

```rust
// tests/mdtest.rs
use cairo_m_test_utils::mdtest;
use dir_test::{dir_test, Fixture};

#[dir_test(
    dir: "$CARGO_MANIFEST_DIR/../mdtest",
    glob: "**/*.md"
)]
fn mdtest(fixture: Fixture<&str>) {
    let path = Path::new(fixture.path());
    let tests = mdtest::extract_tests(path).unwrap();

    for test in tests {
        let result = mdtest::run_test(&test);
        match result {
            TestResult::Pass => {},
            TestResult::Fail(msg) => panic!("{}: {}", test.name, msg),
            TestResult::Ignored(reason) => {
                println!("Ignored {}: {}", test.name, reason);
            }
        }
    }
}
```

## 7. Integration Points

### 7.1 CI/CD Integration

- All markdown tests run as part of `cargo test`
- Test results reported in standard format
- Coverage reports include markdown tests

### 7.2 IDE Integration

- Markdown files recognized as test files
- Code blocks get syntax highlighting
- Run individual tests from IDE

### 7.3 Documentation Generation

- Markdown files can be processed into documentation
- Test status badges automatically added
- Examples guaranteed to be correct

## 8. Migration Strategy

### Phase 1: Infrastructure (Week 1-2)

1. Implement markdown parser in `cairo-m-test-utils`
2. Create basic test runner
3. Set up test discovery with `dir_test`

### Phase 2: Initial Tests (Week 3-4)

1. Convert existing `test_data/*.cm` files to markdown format
2. Organize into logical categories
3. Add documentation for each test

### Phase 3: Differential Testing (Week 5-6)

1. Implement Rust equivalence framework
2. Write Rust reference implementations
3. Integrate differential testing into runner

### Phase 4: Complete Coverage (Week 7-8)

1. Document all language features
2. Ensure 100% test coverage
3. Remove legacy test infrastructure

## 9. Success Criteria

1. **Functional Requirements**
   - [ ] All Cairo-M language features documented in markdown
   - [ ] Every code snippet is an executable test
   - [ ] Differential testing against Rust implementations
   - [ ] Tests organized in learning-friendly structure

2. **Technical Requirements**
   - [ ] Tests run via `cargo test`
   - [ ] Snapshot testing with `insta` integration
   - [ ] Configurable test execution
   - [ ] Multi-file test support

3. **Quality Requirements**
   - [ ] Tests complete in < 30 seconds
   - [ ] Clear error messages for failures
   - [ ] Documentation auto-generated from tests
   - [ ] No test duplication

## 10. Future Enhancements

1. **Watch Mode**: Auto-run tests on file changes
2. **Incremental Testing**: Test compiler incrementality
3. **Performance Benchmarks**: Track performance over time
4. **Property-Based Testing**: Generate test inputs automatically
5. **Fuzzing Integration**: Discover edge cases
6. **Web Playground**: Interactive documentation with runnable examples

## 11. Example Test Suite

````markdown
# Cairo-M Arithmetic Operations

Cairo-M uses the M31 field (2^31 - 1) for all arithmetic operations.

## Basic Addition

Addition in Cairo-M follows field arithmetic rules:

```cairo-m
//! test: parse, semantic, codegen, run
//! expected: 7
//! rust-equiv: tests/equiv/arithmetic.rs::test_addition
fn test_addition() -> felt {
    let a = 3;
    let b = 4;
    return a + b;
}
```

## Field Overflow

When values exceed the field size, they wrap around:

```cairo-m
//! test: run
//! expected: 1
//! rust-equiv: tests/equiv/arithmetic.rs::test_overflow
fn test_overflow() -> felt {
    let max = 2147483646;  // 2^31 - 2
    return max + 3;        // Wraps to 1
}
```

## Division by Zero

Division by zero is caught at compile time:

```cairo-m
//! test: semantic-fail
//! error: "Division by zero"
fn test_div_zero() -> felt {
    return 5 / 0;  // error: [div-by-zero]
}
```
````

This PRD provides a comprehensive blueprint for implementing the markdown-based
testing infrastructure, combining the best practices from Ruff's mdtest with
Cairo-M's specific requirements.
