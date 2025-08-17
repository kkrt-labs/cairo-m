# Task 004: Remove or Fix SROA Pass [HIGH PRIORITY]

## Priority: HIGH - Currently disabled due to IR corruption

## Summary

The SROA (Scalar Replacement of Aggregates) pass is currently disabled due to an
IR corruption bug. With the new aggregate-first design, this pass should either
be completely removed (preferred) or fixed if there's a specific need for it
with arrays.

## Current State

- ⚠️ SROA pass exists but is disabled
- ❌ Known IR corruption bug with `constant_geps` population
- ❌ Incompatible with aggregate-first design philosophy
- ⚠️ Referenced in pipeline but commented out

## Affected Code Locations

### Primary Files

1. `crates/compiler/mir/src/passes/sroa.rs` - The SROA implementation
2. `crates/compiler/mir/src/passes.rs` (line 894) - Disabled in pipeline

### Current Pipeline Configuration

```rust
// SROA pass temporarily disabled due to IR corruption bug
// TODO: Re-enable once constant_geps population is fixed
// .add_pass(sroa::SroaPass::new())
```

## Decision Tree

### Option A: Remove SROA Completely (RECOMMENDED)

**Rationale:**

- SROA is designed for memory-based aggregate handling
- New design uses value-based aggregates (MakeTuple, MakeStruct)
- No longer needed when aggregates are first-class values
- Simplifies codebase and reduces maintenance burden

**Implementation:**

1. Delete `crates/compiler/mir/src/passes/sroa.rs`
2. Remove all references in `passes.rs`
3. Remove from module exports
4. Update documentation

### Option B: Fix and Retain for Arrays

**Rationale (if needed):**

- Arrays still use memory path
- Might benefit from scalar replacement in specific cases

**Known Bug to Fix:**

```rust
// In populate_constant_geps (line causing corruption)
// Issue: Incorrect handling of GEP chains leading to invalid IR
```

## Implementation Steps (Option A - Recommended)

### Step 1: Remove SROA Files

```bash
rm crates/compiler/mir/src/passes/sroa.rs
```

### Step 2: Clean Module References

In `crates/compiler/mir/src/passes/mod.rs`:

```rust
// Remove:
// pub mod sroa;
// pub use sroa::SroaPass;
```

### Step 3: Remove Pipeline References

In `crates/compiler/mir/src/passes.rs`:

```rust
// Delete these lines:
// SROA pass temporarily disabled due to IR corruption bug
// TODO: Re-enable once constant_geps population is fixed
// .add_pass(sroa::SroaPass::new())
```

### Step 4: Update Documentation

- Remove SROA references from optimization documentation
- Update `MIR_REFACTORING_COMPLETED.md` to note removal
- Document in migration guide

## Implementation Steps (Option B - If Fixing)

### Step 1: Debug IR Corruption

```rust
// Fix in populate_constant_geps method
fn populate_constant_geps(&mut self, func: &Function) {
    // Ensure proper traversal order
    // Fix chain following logic
    // Validate IR after population
}
```

### Step 2: Add Aggregate-Aware Guards

```rust
// Skip value-based aggregates
if self.is_value_based_aggregate(alloc_type) {
    continue;  // Don't process tuples/structs
}
```

### Step 3: Comprehensive Testing

- Add IR validation after each transformation
- Test with complex GEP chains
- Verify no corruption with nested aggregates

## Testing Requirements

### For Removal (Option A)

```rust
#[test]
fn test_pipeline_without_sroa() {
    // Verify optimization pipeline works without SROA
    // Check that aggregate operations are unaffected
}
```

### For Fix (Option B)

```rust
#[test]
fn test_sroa_no_ir_corruption() {
    // Test case that previously caused corruption
}

#[test]
fn test_sroa_skips_value_aggregates() {
    // Verify SROA ignores MakeTuple/MakeStruct
}
```

## Verification Checklist

### For Removal

- [ ] SROA file deleted
- [ ] No references in module system
- [ ] Pipeline runs without SROA
- [ ] All tests pass
- [ ] Documentation updated

### For Fix

- [ ] IR corruption bug identified and fixed
- [ ] Aggregate-aware guards in place
- [ ] Comprehensive test coverage
- [ ] IR validation passes
- [ ] Re-enabled in pipeline

## Risk Assessment

### Removal Risks (Low)

- None significant - SROA is already disabled
- Arrays continue using existing memory path

### Fix Risks (Medium)

- May introduce new bugs
- Complexity for diminishing returns
- Maintenance burden for deprecated approach

## Recommendation

**Remove SROA completely.** The aggregate-first design makes SROA obsolete for
tuples/structs, and the complexity of maintaining it for arrays alone doesn't
justify the effort, especially given the known corruption issues.

## Success Criteria

1. No SROA code in codebase (if removed)
2. OR: SROA works without IR corruption (if fixed)
3. Pipeline optimization performance unchanged or improved
4. All tests pass
5. Clean git history with clear commit message
