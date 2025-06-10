# Cairo-M Semantic Analysis Crate

This crate implements semantic analysis for the Cairo-M language, providing
incremental compilation through Salsa, symbol resolution, scope analysis, and
comprehensive validation framework.

## 🎯 What This Crate Does

The semantic crate transforms parsed Cairo-M AST into a rich semantic model that
understands:

- **Scopes and Symbols**: Track all named entities and their containing scopes
- **Definitions**: Link AST nodes to semantic entities
- **Use-Def Analysis**: Resolve identifier uses to their definitions
- **Validation**: Comprehensive semantic validation with detailed diagnostics
- **Incremental Compilation**: Salsa-based caching for fast re-compilation

## 🏗️ Architecture

The crate follows a layered approach inspired by Ruff and rust-analyzer:

```text
┌─────────────────────────────────────┐
│           Validation Layer          │ ← Semantic rules & diagnostics
├─────────────────────────────────────┤
│          Semantic Index             │ ← Main query interface
├─────────────────────────────────────┤
│      Definitions & Use-Def          │ ← Symbol resolution
├─────────────────────────────────────┤
│        Places & Scopes              │ ← Scope tracking
├─────────────────────────────────────┤
│          Parser AST                 │ ← From parser crate
└─────────────────────────────────────┘
```

### Core Components

- **`semantic_index.rs`** - Main entry point, produces complete semantic model
- **`place.rs`** - Scope and place tracking system
- **`definition.rs`** - Definition linking and symbol resolution
- **`validation/`** - Extensible validation framework with diagnostics
- **`db.rs`** - Salsa database implementation for incremental compilation

### Main Query

The primary entry point is `semantic_index(db, file)` which produces a complete
semantic model for a source file, cached by Salsa for incremental compilation.

## 🔧 How to Contribute

### Adding New Validation Rules

1. **Create a new validator** in `src/validation/`:

   ```rust
   pub struct MyValidator;

   impl Validator for MyValidator {
       fn validate(&self, index: &SemanticIndex, diagnostics: &mut DiagnosticCollection) {
           // Your validation logic here
       }
   }
   ```

2. **Add diagnostic codes** to `DiagnosticCode` enum in `diagnostics.rs`

3. **Register your validator** in the appropriate place (usually
   `semantic_index.rs`)

4. **Write comprehensive tests** (see testing section below)

### Extending Semantic Analysis

- **New semantic queries**: Add to `semantic_index.rs` with `#[salsa::tracked]`
- **New place types**: Extend `ScopeKind` in `place.rs`
- **New definition kinds**: Extend `DefinitionKind` in `definition.rs`

### Code Style Guidelines

- Use `#[salsa::tracked]` for cacheable queries
- Prefer immutable data structures (SmolStr, IndexMap)
- Document public APIs with examples
- Use meaningful diagnostic messages with source spans
- Follow existing naming conventions (CamelCase for types, snake_case for
  functions)

## 🧪 Testing Framework

The crate implements a **unified snapshot-driven testing framework** that
provides robust, maintainable tests for semantic validation. All tests use
fixture files and snapshot testing for consistency and ease of maintenance.

### Testing Philosophy

Our testing framework follows modern compiler testing practices (similar to
`rustc`'s UI tests):

- **Single Source of Truth**: `.cm` fixture files contain the test cases
- **Visual Verification**: Beautiful error reports with `ariadne` formatting
- **Zero Brittleness**: No line numbers or fragile expectations
- **Easy Maintenance**: `insta` handles all diffs and updates

### Core Testing Approach

#### **Fixture-Based Snapshot Testing** (Unified Approach)

All semantic validation tests use this pattern:

1. **Create a `.cm` file** in `test_data/` with the code to be tested
2. **Add a test function** that calls `assert_diagnostics_snapshot!()`
3. **Run `cargo insta review`** to generate and review the snapshot
4. **Commit both** the `.cm` file and the generated `.snap` file

**Example**:

```rust
#[test]
fn test_undeclared_variables() {
    assert_diagnostics_snapshot!("undeclared_variables.cm", "undeclared_variables_diagnostics");
}
```

### Test Data Structure

```text
semantic/
├── src/validation/tests/
│   ├── mod.rs                    # Core testing infrastructure
│   ├── integration_tests.rs      # All validation tests
│   ├── diagnostic_tests.rs       # Diagnostic system tests
│   └── snapshots/               # Generated snapshot files
└── test_data/                   # Test fixture files
    ├── fib.cm                   # Clean Fibonacci program
    ├── clean_program.cm         # Complex clean program
    ├── scope_errors.cm          # Comprehensive error cases
    ├── undeclared_variables.cm  # Undeclared variable tests
    ├── unused_variables.cm      # Unused variable tests
    ├── duplicate_definitions.cm # Duplicate definition tests
    └── ... (more test cases)
```

### Available Test Fixtures

**Clean Programs** (no diagnostics expected):

- `fib.cm` - Fibonacci implementation
- `clean_program.cm` - Complex program with structs, functions, and scopes

**Error Test Cases**:

- `scope_errors.cm` - Comprehensive scope validation errors
- `undeclared_variables.cm` - Undeclared variable detection
- `unused_variables.cm` - Unused variable warnings
- `duplicate_definitions.cm` - Duplicate definition errors
- `control_flow_scoping.cm` - If/else scope issues
- `deeply_nested_scopes.cm` - Complex nested scope scenarios

### Test Categories

#### 1. **Clean Validation Tests**

```rust
#[test]
fn test_fib_program_is_clean() {
    test_fixture_clean!("fib.cm");
}
```

#### 2. **Diagnostic Snapshot Tests**

```rust
#[test]
fn test_scope_errors() {
    assert_diagnostics_snapshot!("scope_errors.cm", "scope_errors_diagnostics");
}
```

### Example Snapshot Output

```text
Fixture: undeclared_variables.cm
============================================================
Source code:
func test() {
    let x = y; // Should error: Undeclared variable 'y'
}

============================================================
Found 2 diagnostic(s):

--- Diagnostic 1 ---
[1001] Error: Undeclared variable 'y'
   ╭─[ <unknown>:2:13 ]
   │
 2 │     let x = y; // Should error: Undeclared variable 'y'
   │             ┬
   │             ╰── Undeclared variable 'y'
───╯

--- Diagnostic 2 ---
[1002] Warning: Unused variable 'x'
   ╭─[ <unknown>:2:9 ]
   │
 2 │     let x = y; // Should error: Undeclared variable 'y'
   │         ┬
   │         ╰── Unused variable 'x'
───╯
```

### Running Tests

```bash
# Run all validation tests
cargo test validation::tests

# Run specific test
cargo test test_undeclared_variables

# Update snapshots (when output format changes)
cargo insta review

# Accept all pending snapshots
cargo insta review --accept-all

# Run with detailed output
cargo test -- --nocapture
```

### Adding New Tests

#### For New Validation Rules

1. **Create a test fixture**:

   ```bash
   # Create test_data/my_new_feature.cm
   func test_my_feature() {
       // Your test code here
   }
   ```

2. **Add a test function**:

   ```rust
   #[test]
   fn test_my_new_feature() {
       assert_diagnostics_snapshot!("my_new_feature.cm", "my_new_feature_diagnostics");
   }
   ```

3. **Generate the snapshot**:

   ```bash
   cargo test test_my_new_feature
   cargo insta review
   ```

4. **Commit both files**:
   - `test_data/my_new_feature.cm`
   - `src/validation/tests/snapshots/cairo_m_compiler_semantic__validation__tests__my_new_feature_diagnostics.snap`

### Available Test Helpers

- `assert_diagnostics_snapshot!(fixture, snapshot_name)` - Generate/verify
  diagnostic snapshots
- `test_fixture_clean!(fixture)` - Assert no diagnostics are produced
- Core infrastructure in `src/validation/tests/mod.rs` handles all validation
  and formatting

### Benefits of This Approach

- **Robust**: No brittle line numbers or manual diagnostic matching
- **Visual**: Beautiful error reports make it easy to verify correctness
- **Maintainable**: `insta` handles all the complexity of diffing and updating
- **Scalable**: Easy to add new test cases and validation rules
- **Consistent**: All tests follow the same pattern
