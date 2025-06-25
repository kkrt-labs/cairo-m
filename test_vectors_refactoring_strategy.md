# Test Vectors Refactoring Strategy

## What

Consolidate and organize all Cairo M test programs (.cm files) across the cairo-m repository into a centralized `cairo_m_programs` directory while maintaining support for crate-specific test vectors. This addresses the current duplication and inconsistency of test programs across different crates (runner, prover, compiler components).

## Why

### Current Problems

1. **Duplication**: Multiple identical programs exist across crates:
   - `fibonacci.cm` exists identically in both `crates/runner/tests/test_data/` and `crates/prover/tests/test_data/`
   - Similar programs like `fib.cm` and `fib_loop.cm` are duplicated across MIR and codegen test directories
   - Different fibonacci implementations exist (recursive vs iterative) in different locations

2. **Inconsistent Organization**: Test programs are scattered with no standard structure:
   - Runner: `tests/test_data/` and `benches/` directories
   - Prover: `tests/test_data/` directory
   - Compiler MIR: `tests/test_cases/` with subdirectories
   - Compiler Codegen: `tests/test_cases/` with subdirectories
   - Compiler Semantic: Primarily inline strings with some `.cm` files

3. **Maintenance Overhead**: 
   - Updates to test programs require changes in multiple locations
   - Risk of programs diverging when updated in only some locations
   - Difficulty discovering existing test programs when creating new tests

4. **Unclear Intent**: Programs with similar names but different implementations create confusion about which version is canonical

### Benefits of Consolidation

1. **Single Source of Truth**: All test programs maintained in one location
2. **Reduced Duplication**: Eliminate redundant copies of identical programs
3. **Better Discoverability**: Developers can easily find existing test programs
4. **Easier Maintenance**: Updates propagate automatically to all consumers
5. **Consistent Testing**: Same programs used across different crates ensure consistency
6. **Clear Categorization**: Separate good programs, bad programs, and benchmarks

## How

### Phase 1: Analysis and Inventory

#### 1.1 Catalog Existing Test Programs

**Current Distribution:**
- **Runner**: 13 programs in `tests/test_data/` + 1 in `benches/`
- **Prover**: 1 program in `tests/test_data/`
- **Compiler MIR**: ~30+ programs across 7 subdirectories in `test_cases/`
- **Compiler Codegen**: ~20+ programs across 6 subdirectories in `test_cases/`
- **Compiler Semantic**: Minimal `.cm` files, mostly inline tests

**Identified Duplications:**
- `fibonacci.cm`: Identical in runner and prover
- `fib.cm` and `fib_loop.cm`: Similar across MIR and codegen
- Various arithmetic and control flow programs duplicated across compiler crates

#### 1.2 Classify Test Programs

**Good Programs** (should compile and run successfully):
- Arithmetic operations
- Control flow constructs
- Function definitions and calls
- Data structures (structs, arrays)
- Benchmarking programs

**Bad Programs** (should fail at specific compilation stages):
- Parser errors (syntax issues)
- Semantic errors (type mismatches, undeclared variables)
- MIR generation errors
- Codegen errors

**Benchmark Programs** (performance testing):
- High-iteration programs for performance measurement
- Memory-intensive programs
- Computation-heavy algorithms

### Phase 2: Design New Structure

#### 2.1 Proposed Directory Structure

```
cairo_m_programs/
├── good/                          # Programs that should compile and run successfully
│   ├── arithmetic/               # Basic arithmetic operations
│   │   ├── add_two_numbers.cm
│   │   ├── subtract_numbers.cm
│   │   ├── equality.cm
│   │   └── ...
│   ├── control_flow/             # If statements, loops, jumps
│   │   ├── simple_if.cm
│   │   ├── if_else.cm
│   │   ├── nested_loops.cm
│   │   └── ...
│   ├── functions/                # Function definitions and calls
│   │   ├── simple_call.cm
│   │   ├── fibonacci_recursive.cm
│   │   ├── fibonacci_iterative.cm
│   │   └── ...
│   ├── structures/               # Structs, arrays, aggregates
│   │   ├── struct_literal.cm
│   │   ├── array_access.cm
│   │   └── ...
│   ├── expressions/              # Expression evaluation
│   │   ├── binary_ops.cm
│   │   ├── comparison_ops.cm
│   │   └── ...
│   └── integration/              # Complex end-to-end programs
│       ├── factorial.cm
│       ├── ackermann.cm
│       └── ...
├── bad/                          # Programs that should fail compilation
│   ├── parser/                   # Syntax errors
│   │   ├── missing_semicolon.cm
│   │   ├── unmatched_braces.cm
│   │   └── ...
│   ├── semantic/                 # Semantic analysis errors
│   │   ├── undeclared_variable.cm
│   │   ├── type_mismatch.cm
│   │   ├── duplicate_function.cm
│   │   └── ...
│   ├── mir/                      # MIR generation errors (if any)
│   └── codegen/                  # Codegen-specific errors (if any)
├── bench/                        # Performance benchmarking programs
│   ├── fibonacci_1m.cm           # 1 million iterations
│   ├── factorial_large.cm
│   └── ...
└── README.md                     # Documentation and usage guidelines
```

#### 2.2 Naming Conventions

**File Naming:**
- Use descriptive names that indicate the feature being tested
- For similar algorithms with different approaches, include the approach:
  - `fibonacci_recursive.cm`
  - `fibonacci_iterative.cm`
- Use snake_case consistently
- Include complexity or scale for benchmark programs:
  - `fibonacci_1m.cm` (1 million iterations)

**Program Naming:**
- Maintain consistent function names where possible
- Use `main()` as the entry point for complete programs
- Use descriptive function names for specific feature tests

### Phase 3: Implementation Plan

#### 3.1 Step 1: Create Central Directory Structure

```bash
mkdir -p cairo_m_programs/{good,bad,bench}
mkdir -p cairo_m_programs/good/{arithmetic,control_flow,functions,structures,expressions,integration}
mkdir -p cairo_m_programs/bad/{parser,semantic,mir,codegen}
```

#### 3.2 Step 2: Migrate and Deduplicate Programs

**Migration Strategy:**
1. **Identify canonical versions**: For duplicated programs, choose the most complete/correct version
2. **Resolve naming conflicts**: Rename programs to reflect their specific purpose
3. **Preserve crate-specific variants**: Some programs may need to remain crate-specific for specialized testing

**Example Migration:**
- `runner/tests/test_data/fibonacci.cm` → `cairo_m_programs/good/functions/fibonacci_recursive.cm`
- `runner/tests/test_data/fibonacci_loop.cm` → `cairo_m_programs/good/functions/fibonacci_iterative.cm`
- `runner/benches/fibonacci_loop.cm` → `cairo_m_programs/bench/fibonacci_1m.cm`

#### 3.3 Step 3: Update Build System and Test Infrastructure

**Cargo.toml Updates:**
```toml
# Add to workspace Cargo.toml or individual crates
[package.metadata.cairo-m]
shared_programs_dir = "cairo_m_programs"
```

**Test Helper Functions:**
```rust
// Add to cairo-m-common or a new cairo-m-test-utils crate
pub fn load_shared_program(category: &str, name: &str) -> String {
    let path = format!("cairo_m_programs/{}/{}", category, name);
    std::fs::read_to_string(path).expect("Failed to load shared program")
}

pub fn load_good_program(name: &str) -> String {
    load_shared_program("good", name)
}

pub fn load_bad_program(name: &str) -> String {
    load_shared_program("bad", name)
}

pub fn load_bench_program(name: &str) -> String {
    load_shared_program("bench", name)
}
```

#### 3.4 Step 4: Update Individual Crate Tests

**Runner Updates:**
```rust
// In diff_tests.rs
fn compile_cairo_file(cairo_file: &str) -> Result<Program, String> {
    let source_path = format!("cairo_m_programs/good/{}", cairo_file);
    // ... rest of implementation
}

// In vm_benchmark.rs
fn fibonacci_1m_benchmark(c: &mut Criterion) {
    let source_path = "cairo_m_programs/bench/fibonacci_1m.cm";
    // ... rest of implementation
}
```

**Prover Updates:**
```rust
// Use shared programs instead of local copies
let program_source = load_good_program("functions/fibonacci_recursive.cm");
```

**Compiler Crate Updates:**
```rust
// MIR tests
mir_test!(fibonacci_recursive, "functions/fibonacci_recursive");

// Codegen tests  
codegen_test!(fibonacci_recursive, "functions/fibonacci_recursive");
```

#### 3.5 Step 5: Maintain Crate-Specific Test Vectors

**Keep Local Tests For:**
1. **Compiler Error Testing**: Parser, semantic, and other compilation phase specific errors
2. **Feature-Specific Tests**: Tests that verify specific compiler features
3. **Inline Tests**: Simple tests that are better kept as inline strings

**Example Structure After Migration:**
```
crates/
├── runner/
│   └── tests/
│       └── integration/          # Runner-specific integration tests
├── prover/
│   └── tests/
│       └── prover_specific/      # Prover-specific test cases
├── compiler/
│   ├── parser/tests/             # Parser-specific error tests (inline)
│   ├── semantic/tests/           # Semantic analysis tests (inline + some .cm)
│   ├── mir/tests/                # MIR-specific tests (mostly uses shared)
│   └── codegen/tests/            # Codegen-specific tests (mostly uses shared)
```

### Phase 4: Implementation Steps

#### Step 1: Inventory and Analysis (1-2 days)
- [ ] Complete catalog of all .cm files across crates
- [ ] Identify exact duplications and near-duplications  
- [ ] Classify programs by category (good/bad/bench)
- [ ] Document current usage patterns

#### Step 2: Directory Setup (1 day)
- [ ] Create `cairo_m_programs` directory structure
- [ ] Add README.md with usage guidelines
- [ ] Set up basic CI checks for the new directory

#### Step 3: Migration Phase 1 - Good Programs (2-3 days)
- [ ] Migrate unique programs from runner test_data
- [ ] Migrate unique programs from prover test_data  
- [ ] Resolve duplications by choosing canonical versions
- [ ] Migrate and organize compiler test programs

#### Step 4: Migration Phase 2 - Bad Programs (2-3 days)
- [ ] Identify programs that should fail compilation
- [ ] Create bad program categories based on failure points
- [ ] Migrate existing bad programs
- [ ] Create additional bad programs for comprehensive coverage

#### Step 5: Migration Phase 3 - Benchmark Programs (1 day)  
- [ ] Migrate performance testing programs
- [ ] Ensure benchmark programs are appropriately scaled
- [ ] Document performance expectations

#### Step 6: Update Infrastructure (2-3 days)
- [ ] Create test utility functions for loading shared programs
- [ ] Update runner tests to use shared programs
- [ ] Update prover tests to use shared programs  
- [ ] Update compiler crate tests to use shared programs

#### Step 7: Cleanup and Validation (1-2 days)
- [ ] Remove old duplicated files
- [ ] Verify all tests still pass
- [ ] Update documentation
- [ ] Add CI checks to prevent duplication

### Phase 5: Maintenance and Guidelines

#### 5.1 Guidelines for New Test Programs

**Adding New Programs:**
1. Check if a similar program already exists in `cairo_m_programs/`
2. If adding a variant, use descriptive naming to distinguish purpose
3. Add programs to the appropriate category (good/bad/bench)
4. Update relevant test files to reference the new program
5. Document the program's purpose in comments

**Modifying Existing Programs:**
1. Consider impact on all crates that use the program
2. If modification is crate-specific, create a variant instead
3. Update version in central location rather than copying
4. Run full test suite to verify no regressions

#### 5.2 CI Integration

**Automated Checks:**
- Prevent duplication by scanning for identical file contents
- Ensure all shared programs are syntactically valid
- Verify that removing any shared program doesn't break tests
- Check that new .cm files are added to the shared directory

**Example CI Check:**
```yaml
name: Test Vector Validation
on: [push, pull_request]
jobs:
  check-duplicates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Check for duplicate test programs
        run: |
          find . -name "*.cm" -not -path "./cairo_m_programs/*" | while read file; do
            echo "Found .cm file outside shared directory: $file"
            exit 1
          done
```

### Expected Outcomes

1. **Elimination of Duplication**: All identical programs consolidated to single canonical versions
2. **Improved Maintainability**: Single location for updates to test programs
3. **Better Test Coverage**: Systematic organization reveals gaps in testing
4. **Clearer Intent**: Programs categorized by purpose and expected behavior
5. **Easier Development**: Developers can easily find and reuse existing test programs
6. **Consistent Testing**: All crates use the same validated test programs

This refactoring will establish a solid foundation for test program management while maintaining the flexibility for crate-specific testing needs.