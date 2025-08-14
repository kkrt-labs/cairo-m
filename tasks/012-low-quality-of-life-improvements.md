# Task: Quality of Life Improvements

## Priority

LOW

## Why

These small quality-of-life improvements address minor code quality issues that
don't affect functionality but reduce technical debt, improve maintainability,
and enhance the developer experience. While individually minor, collectively
they represent good engineering practices and attention to detail.

## What

1. **Unused extra block in cfg.rs test**: The `test_critical_edge_splitting`
   function creates 4 blocks and uses all of them appropriately (entry, b1, b2,
   merge), but the comment suggests this was flagged as creating an "unused
   extra block"

2. **FuseCmpBranch zero comparison brittleness**: The zero comparison
   optimization in lines 93-121 of `passes.rs` matches against
   `Value::Literal(Literal::Integer(0))` but may be too broad or miss edge cases
   with different zero representations

3. **Pretty printing inconsistencies**: Display implementations in `value.rs`,
   `instruction.rs`, and `mir_types.rs` all delegate to `pretty_print(0)` but
   may have formatting inconsistencies across different types

4. **Pre-opt pass duplicate use_counts computation**: In `pre_opt.rs`, the
   `calculate_value_use_counts` method is called multiple times (lines 33,
   83, 109) for different elimination phases, resulting in redundant computation

5. **DataLayout documentation**: While the `DataLayout` struct in `layout.rs`
   has good overall documentation, some methods like `alignment_of` and
   `struct_size_with_padding` could benefit from clearer documentation about
   future extensibility plans

6. **Logging inconsistency**: Extensive use of `eprintln!` throughout the MIR
   crate instead of proper logging infrastructure (found in `passes.rs`,
   `pre_opt.rs`, and `lowering/function.rs`)

## How

### 1. CFG Test Review

```rust
// In cfg.rs, verify if the test actually has an unused block or if this is a false positive
// If there is an issue, consolidate block creation to only what's needed
```

### 2. FuseCmpBranch Zero Comparison Robustness

```rust
// In passes.rs, lines 93-121
// Consider more robust zero detection:
fn is_zero_value(value: &Value) -> bool {
    matches!(value, Value::Literal(Literal::Integer(0)) | Value::Literal(Literal::Boolean(false)))
}

// And add validation for edge cases like comparing variables known to be zero
```

### 3. Pretty Printing Consistency

```rust
// Create a consistent pretty printing trait or ensure all Display impls
// use the same formatting conventions for similar constructs
// Consider adding integration tests for pretty printing consistency
```

### 4. Pre-opt Use Counts Optimization

```rust
// In pre_opt.rs, compute use_counts once and reuse:
impl PreOptimizationPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;
        let use_counts = self.calculate_value_use_counts(function);

        modified |= self.eliminate_dead_instructions_with_counts(function, &use_counts);
        // Recompute only if the function was modified
        let use_counts = if modified {
            self.calculate_value_use_counts(function)
        } else {
            use_counts
        };
        modified |= self.eliminate_dead_allocations_with_counts(function, &use_counts);

        modified
    }
}
```

### 5. DataLayout Documentation Enhancement

```rust
// In layout.rs, enhance method documentation:
/// Get the alignment requirement for a type (in slots)
///
/// Currently returns 1 for all types (no alignment padding).
/// Future versions may implement:
/// - Target-specific alignment requirements
/// - SIMD-friendly alignment for vector types
/// - Cache-line alignment for performance-critical structures
pub const fn alignment_of(&self, _ty: &MirType) -> usize {
    1 // All types are currently 1-slot aligned
}
```

### 6. Proper Logging Infrastructure

```rust
// Replace eprintln! with proper logging:
// 1. Add `log` dependency to Cargo.toml
// 2. Replace eprintln! with log::warn!, log::debug!, log::error!
// 3. Update environment variable checks to use log level filtering

// Example transformation:
// Before:
if std::env::var("RUST_LOG").is_ok() {
    eprintln!("[ERROR] MIR Validation failed: {}", err);
}

// After:
log::error!("MIR Validation failed for function '{}': {}", function.name, err);
```

## Testing

1. **CFG Tests**: Run existing cfg tests to ensure no regressions
2. **FuseCmpBranch**: Add test cases for zero comparison edge cases (different
   zero representations, boolean false, etc.)
3. **Pretty Printing**: Add integration tests comparing pretty printing output
   across different types for consistency
4. **Pre-opt Performance**: Benchmark use_counts computation before/after
   optimization
5. **Logging**: Verify log output appears correctly with different log levels
   and that old eprintln output is preserved

## Impact

- **Code Quality**: Eliminates inconsistencies and improves code organization
- **Performance**: Reduces redundant computation in pre-optimization pass
- **Maintainability**: Better documentation and consistent logging make the
  codebase easier to work with
- **Developer Experience**: Proper logging infrastructure provides better
  debugging capabilities
- **Technical Debt**: Addresses minor issues before they become larger problems

These improvements, while individually small, contribute to overall code quality
and make the codebase more professional and maintainable. They represent the
kind of attention to detail that distinguishes high-quality software
engineering.
