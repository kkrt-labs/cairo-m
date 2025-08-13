# PR3: Pass Pipeline & Pre-opt Tweaks

## Objective

Fix pass ordering, enforce SSA invariants, and make dead store elimination safe.

## Tasks

### Task 1: Pass Pipeline Assessment (No Changes Needed)

**Current Pipeline Location:** `crates/compiler/mir/src/passes.rs` lines 467-478

**Current Order (CORRECT):**

1. PreOptimizationPass
2. SroaPass (requires SSA for phase 2)
3. Mem2RegSsaPass (creates SSA form)
4. SsaDestructionPass (destroys SSA form)
5. FuseCmpBranch (doesn't need SSA)
6. DeadCodeElimination (doesn't need SSA)
7. Validation

**Finding:** Current ordering is actually correct - SSA destruction happens
after all SSA-requiring passes and before non-SSA passes.

**Action:** Document this is correct, no changes needed to ordering.

### Task 2: Fix Dead Store Elimination (CRITICAL)

**File:** `crates/compiler/mir/src/passes/pre_opt.rs`

**Current Bug (lines 38-41):**

```rust
if let InstructionKind::Store { address, .. } = &instr.kind
    && let Value::Operand(dest) = address
    && use_counts.get(dest).copied().unwrap_or(0) == 0
```

**Problem:** Only checks if the direct pointer has zero uses, ignores aliasing
through GEPs.

**Example of Incorrect Elimination:**

```mir
%base = framealloc Rectangle
%field1 = getelementptr %base, 0
store %field1, 42              // INCORRECTLY ELIMINATED if %field1 unused
%field2 = getelementptr %base, 0  // Same memory location!
%value = load felt %field2     // Loads undefined value after elimination
```

#### Immediate Fix:

- [ ] **Line 155:** Comment out dead store elimination

```rust
// DISABLED: Unsound with GEP aliasing (see issue #XXX)
// modified |= self.eliminate_dead_stores(function);
```

#### Long-term Fix Options:

1. **Conservative Version:** Only eliminate stores to:
   - Local variables never address-taken
   - Allocations with no GEP instructions
2. **Alias Analysis:** Track memory locations, not pointer values

### Task 3: Enforce SSA Invariants

**File:** `crates/compiler/mir/src/function.rs`

**Unused Mechanism (lines 144-154):**

```rust
pub fn mark_as_defined(&mut self, dest: ValueId) -> Result<(), String> {
    if !self.defined_values.insert(dest) {
        return Err(format!(
            "SSA violation: ValueId {:?} is being defined multiple times",
            dest
        ));
    }
    Ok(())
}
```

**Currently:** No calls to `mark_as_defined` in codebase!

#### Implementation:

- [ ] `crates/compiler/mir/src/lowering/builder.rs` - Add to instruction
      creation:
  - In `add_instruction()` method, after creating instruction with dest
  - Check instruction type, if has destination, call
    `function.mark_as_defined(dest)?`
- [ ] Allow SSA destruction pass to bypass:
  - Add flag `bypass_ssa_check: bool` to MirFunction
  - Set true only during SSA destruction pass

### Task 4: Clean up Unused Fields

**File:** `crates/compiler/mir/src/function.rs`

**Potentially Unused Field (line 41):**

```rust
locals: FxHashMap<MirDefinitionId, ValueId>
```

**Usage Analysis:**

- Only used in `map_definition()` and `lookup_definition()` (lines 156-164)
- Used during initial lowering, not by optimization passes

**Decision:** Keep but document its purpose:

```rust
/// Maps semantic variable definitions to MIR values during lowering.
/// Not used by optimization passes, which work directly with ValueIds.
locals: FxHashMap<MirDefinitionId, ValueId>
```

### Task 5: Document Pass Pipeline

**Create:** `crates/compiler/mir/PASSES.md`

**Content to Include:**

```markdown
# MIR Pass Pipeline Documentation

## Pass Execution Order

### SSA-Based Passes (require SSA form)

1. **PreOptimizationPass**: Basic cleanup (dead code elimination)
2. **SroaPass**: Scalar Replacement of Aggregates
   - Phase 1: Alloca splitting (doesn't require SSA)
   - Phase 2: SSA aggregate scalarization (requires SSA)
3. **Mem2RegSsaPass**: Promote allocas to SSA registers
   - Creates SSA form with Phi nodes

### SSA Destruction

4. **SsaDestructionPass**: Convert Phi nodes to explicit assignments
   - MUST run after all SSA-requiring passes
   - Destroys SSA single-assignment property

### Post-SSA Passes (work without SSA)

5. **FuseCmpBranch**: Combine compare and branch instructions
6. **DeadCodeElimination**: Remove unreachable code
7. **Validation**: Final structural validation

## Pass Invariants

### Before Mem2RegSsaPass

- Memory operations for locals
- No Phi nodes

### After Mem2RegSsaPass, Before SsaDestruction

- SSA form with Phi nodes
- Each ValueId defined exactly once
- Promotable allocas eliminated

### After SsaDestruction

- No Phi nodes
- Values may have multiple definitions via assignments
- Ready for code generation

## Adding New Passes

- SSA-requiring passes: Add before SsaDestruction
- General cleanup: Add to PreOptimization or after SsaDestruction
- Validation: Always last
```

## Implementation Order

1. **URGENT**: Disable dead store elimination (prevents miscompilation)
2. Document pass pipeline (clarify for team)
3. Enforce SSA invariants with mark_as_defined
4. Add tests for GEP aliasing patterns
5. Design proper alias analysis for dead store elimination

## Testing Strategy

- Add test cases showing GEP aliasing issues:
  ```mir
  // Test that store is NOT eliminated when accessed via different GEP
  %base = framealloc Struct
  %ptr1 = getelementptr %base, 0
  store %ptr1, 42
  %ptr2 = getelementptr %base, 0
  %val = load %ptr2
  // CHECK: val == 42
  ```
- Verify SSA invariants with mark_as_defined active
- Test that Phi nodes are only present between mem2reg and ssa_destruction
