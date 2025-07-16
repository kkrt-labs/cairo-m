# Semantic Validation Tests

This directory contains the reorganized semantic validation tests for the
Cairo-M compiler. The tests are structured by concern to provide clear
visibility into what semantic features are implemented and validated.

## Test Organization

### By Concern

- **`scoping/`** - Variable scoping, visibility, and declaration tests

  - `undeclared_variables.rs` - Tests for undeclared variable detection
  - `duplicate_definitions.rs` - Tests for duplicate definition detection
  - `unused_variables.rs` - Tests for unused variable warnings
  - `scope_visibility.rs` - Tests for scope visibility rules
  - `nested_scopes.rs` - Tests for complex nested scope scenarios

- **`control_flow/`** - Control flow analysis tests

  - `unreachable_code.rs` - Tests for unreachable code detection
  - `missing_returns.rs` - Tests for missing return statement detection
  - `control_flow_paths.rs` - Tests for control flow path analysis

- **`functions/`** - Function-related validation tests

  - `function_calls.rs` - Tests for function call validation
  - `parameter_validation.rs` - Tests for parameter validation
  - `return_types.rs` - Tests for return type validation

- **`statements/`** - Statement-level validation tests

  - `let_statements.rs` - Tests for let statement validation
  - `assignments.rs` - Tests for assignment validation
  - `expression_statements.rs` - Tests for expression statement validation

- **`types/`** - Type system validation tests

  - `type_resolution_tests.rs` - Tests for basic type resolution
  - `definition_type_tests.rs` - Tests for definition type resolution
  - `expression_type_tests.rs` - Tests for expression type inference
  - `function_signature_tests.rs` - Tests for function signature resolution
  - `struct_type_tests.rs` - Tests for struct type resolution
  - `type_compatibility_tests.rs` - Tests for type compatibility checks
  - `recursive_and_error_types_tests.rs` - Tests for recursive types and error
    handling

- **`structures/`** - Struct validation tests (TODO)
- **`expressions/`** - Expression validation tests (TODO)
- **`integration/`** - End-to-end integration tests
- **`test_data/`** - Complex scenarios using .cm files

## Test Utilities

### Assertion Macros

By default, unused variable warnings are ignored by the assertion macros to
allow tests to focus on other errors. The `show_unused` flag can be used to make
these warnings visible to the assertions.

- `assert_semantic_ok!(code)` - Asserts successful validation. Unused variable
  warnings are ignored.
- `assert_semantic_ok!(code, show_unused)` - Asserts successful validation.
  Fails if there are _any_ diagnostics, including unused variable warnings.
- `assert_semantic_err!(code)` - Asserts validation failure. Unused variable
  warnings are ignored.
- `assert_semantic_err!(code, show_unused)` - Asserts validation failure. Unused
  variable warnings will be treated as an error.
- `assert_diagnostics_snapshot!(file, name)` - Snapshot test for .cm files.
- `test_fixture_clean!(file)` - Assert .cm file validates without errors.

### Helper Functions

- `in_function(code)` - Wrap statement code inside a function
- `in_function_with_return(code, return_type)` - Wrap code in function with
  return type
- `in_function_with_params(code, params)` - Wrap code in function with
  parameters
- `in_function_with_params_and_return(code, params, return_type)` - Full
  function wrapper
- `with_struct(name, fields, code)` - Add struct definition before code
- `with_functions(functions, main_code)` - Add helper functions before main code

### Example Usage

```rust
#[test]
fn test_let_statement() {
    // Simple inline test - this passes because `x` is used.
    assert_semantic_ok!(&in_function("let x = 42; return x;"));
}

#[test]
fn test_unused_variable_error() {
    // To test for an unused variable warning, use `show_unused`.
    // This will fail if the warning is not produced.
    assert_semantic_err!(&in_function("let x = 42; return ();"), show_unused);
}

#[test]
fn test_with_helper_function() {
    assert_semantic_ok!(&with_functions(
        "fn helper() -> felt { return 42; }",
        &in_function("let x = helper(); return x;")
    ));
}

#[test]
fn test_complex_scenario() {
    // Use .cm file for complex scenarios
    assert_diagnostics_snapshot!("complex_program.cm", "complex_program_diagnostics");
}
```

## Migration from Old Structure

The old test structure in `src/validation/tests/` used primarily `.cm` files
with snapshot testing. The new structure:

1. **Migrates simple tests to inline strings** using `assert_semantic_ok!` and
   `assert_semantic_err!`
2. **Organizes tests by semantic concern** rather than by file
3. **Provides helper functions** to reduce boilerplate
4. **Adds unused variable warning filtering** to focus tests on specific
   features
5. **Keeps .cm files for complex scenarios** that benefit from separate files

## Benefits

1. **Clear test organization** - Easy to see what's tested and what's missing
2. **Reduced boilerplate** - Helper functions eliminate repetitive code
3. **Focused testing** - Mute irrelevant warnings to focus on specific features
4. **Better maintainability** - Inline tests are easier to read and modify
5. **Comprehensive coverage** - Systematic organization ensures complete
   coverage

## Running Tests

```bash
# Run all semantic tests
cargo test --test semantic_tests

# Run specific test modules
cargo test --test semantic_tests scoping
cargo test --test semantic_tests control_flow
cargo test --test semantic_tests integration

# Run with snapshot review
cargo insta review
```
