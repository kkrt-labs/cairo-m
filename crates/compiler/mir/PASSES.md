# MIR Pass Pipeline Documentation

## Pass Execution Order

The MIR optimization pipeline is carefully ordered to respect pass dependencies
and invariants. Passes are executed in the following order:

### SSA-Based Passes (require SSA form)

1. **PreOptimizationPass**: Basic cleanup (dead code elimination)
   - Removes unused instructions and allocations
   - Note: Dead store elimination is currently disabled due to aliasing
     unsoundness

2. **SroaPass**: Scalar Replacement of Aggregates
   - Phase 1: Alloca splitting (doesn't require SSA)
   - Phase 2: SSA aggregate scalarization (requires SSA)
   - Replaces aggregate allocations with individual scalar allocations

3. **Mem2RegSsaPass**: Promote allocas to SSA registers
   - Creates SSA form with Phi nodes
   - Promotes memory allocations to SSA values where possible
   - Critical for performance and enabling further optimizations

### SSA Destruction

4. **SsaDestructionPass**: Convert Phi nodes to explicit assignments
   - MUST run after all SSA-requiring passes
   - Destroys SSA single-assignment property
   - Prepares code for backends that don't support Phi nodes

### Post-SSA Passes (work without SSA)

5. **FuseCmpBranch**: Combine compare and branch instructions
   - Pattern matches cmp+branch sequences
   - Fuses them into single BranchCmp instructions

6. **DeadCodeElimination**: Remove unreachable code
   - Eliminates blocks not reachable from entry
   - Removes instructions after terminators

7. **Validation**: Final structural validation
   - Verifies IR invariants
   - Checks for type consistency
   - Ensures no malformed instructions

## Pass Invariants

### Before Mem2RegSsaPass

- Memory operations for locals
- No Phi nodes
- Allocations for all local variables

### After Mem2RegSsaPass, Before SsaDestruction

- SSA form with Phi nodes at dominance frontiers
- Each ValueId defined exactly once (SSA property)
- Promotable allocas eliminated
- Non-promotable allocas remain (aggregates, address-taken variables)

### After SsaDestruction

- No Phi nodes
- Values may have multiple definitions via assignments
- Ready for code generation
- Critical edges may be split for assignment placement

## Adding New Passes

When adding optimization passes to the pipeline:

- **SSA-requiring passes**: Add before SsaDestruction
- **General cleanup**: Add to PreOptimization or after SsaDestruction
- **Validation**: Always last
- **Consider dependencies**: Document what invariants your pass requires and
  preserves

## Known Issues

### Dead Store Elimination

The dead store elimination pass in PreOptimization is currently disabled due to
unsoundness with pointer aliasing through GEP instructions. The pass incorrectly
assumes that if a pointer has zero direct uses, stores through it can be
eliminated. However, the same memory location may be accessed through different
GEP-derived pointers.

Example of incorrect elimination:

```mir
%base = framealloc Rectangle
%field1 = getelementptr %base, 0
store %field1, 42              // Would be incorrectly eliminated
%field2 = getelementptr %base, 0  // Same memory location!
%value = load felt %field2     // Would load undefined value
```

A proper implementation requires alias analysis to track which pointers may
refer to the same memory location.

## Testing Pass Correctness

Each pass should have:

1. Unit tests for individual transformations
2. Integration tests showing interaction with other passes
3. Snapshot tests for IR transformations
4. Regression tests for fixed bugs

Use the `PassManager::new().add_pass()` API to test passes in isolation or
specific combinations.
