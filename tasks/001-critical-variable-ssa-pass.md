# Task 001: Implement Variable-SSA Pass [CRITICAL]

## Priority: CRITICAL - Blocks value-based aggregate completion

## Summary

Implement the missing Variable-SSA pass to convert mutable variables from
memory-based operations to proper SSA form with Phi nodes. This is the critical
missing piece preventing full adoption of value-based aggregates.

## Current State

- ❌ No `var_ssa.rs` file exists in `crates/compiler/mir/src/passes/`
- ❌ Variables still use memory slots for state management
- ❌ Assignments use `store` operations instead of SSA rebinding
- ❌ No Phi node insertion for variables at control flow merges

## Impact

Without this pass:

- Assignment statements remain memory-based
  (`crates/compiler/mir/src/lowering/stmt.rs:279-281`)
- Return statements use memory loads
  (`crates/compiler/mir/src/lowering/stmt.rs:189-197`)
- The aggregate-first design benefits are not fully realized
- Performance overhead from unnecessary memory operations

## Implementation Plan

### 1. Create `crates/compiler/mir/src/passes/var_ssa.rs`

```rust
pub struct VarSsaPass {
    // Track variables (MirDefinitionId) that need promotion
    promoted_vars: HashSet<MirDefinitionId>,
}
```

### 2. Core Implementation Components

#### Phase 1: Analysis

- Identify all variables that are assigned multiple times
- Track definition sites per basic block
- Build def-use chains for MirDefinitionIds

#### Phase 2: Phi Placement

- Reuse dominance analysis from `crates/compiler/mir/src/analysis/dominance.rs`
- Calculate dominance frontiers for variable definition sites
- Insert Phi nodes at control flow merge points

#### Phase 3: Variable Renaming

- DFS traversal over dominator tree
- Maintain stacks for current SSA values per variable
- Update all uses to reference renamed SSA values
- Pop stacks on backtrack

### 3. Integration Points

#### Update Lowering (`crates/compiler/mir/src/lowering/stmt.rs`)

- Modify `lower_assignment_statement` to:
  - For simple variables: Create new SSA value binding
  - For field assignments: Use `InsertField` instruction
  - For tuple elements: Use `InsertTuple` instruction
  - Mark blocks as definition sites for Variable-SSA

#### Update Return Handling

- Modify `lower_return_statement` to use `ExtractTupleElement` on SSA values
- Remove memory-based tuple element loading

### 4. Pipeline Integration

In `crates/compiler/mir/src/passes.rs`:

```rust
pub fn standard_pipeline() -> PassManager {
    PassManager::new()
        // ... existing passes ...
        .add_pass(var_ssa::VarSsaPass::new())  // Add before validation
        .add_pass(Validation)
        // ...
}
```

## Testing Requirements

### Unit Tests

- Test Phi node insertion for if-else branches
- Test loop variable updates
- Test nested control flow
- Test aggregate member updates

### Integration Tests

- Verify correct SSA form generation
- Ensure SSA destruction produces valid code
- Test with optimization pipeline enabled/disabled

## Verification Checklist

- [ ] Variables in simple assignments use SSA rebinding
- [ ] Field/tuple updates use Insert instructions
- [ ] Phi nodes correctly placed at dominance frontiers
- [ ] All variable uses reference correct SSA values
- [ ] SSA destruction pass handles new Phi nodes
- [ ] No regression in existing tests

## Dependencies

- Requires existing dominance analysis infrastructure ✅
- Requires Phi instruction support ✅
- Requires SSA destruction pass ✅

## References

- Original design: `crates/compiler/mir/MIR_REPORT.md` Issue #6
- Dominance analysis: `crates/compiler/mir/src/analysis/dominance.rs`
- Similar pattern: `crates/compiler/mir/src/passes/mem2reg_ssa.rs`

## Success Criteria

1. All variable assignments become SSA rebinding (no memory stores)
2. Aggregate member updates use Insert instructions
3. Function can be marked as "uses_no_memory" after pass
4. Performance improvement measurable in benchmark suite
