# Task 007: Fix Dead Store Elimination Soundness Issue [MEDIUM PRIORITY]

## Priority: MEDIUM - Correctness issue that could cause miscompilation

## Summary

The dead store elimination pass in `pre_opt.rs` has an unsoundness issue where
it assumes zero direct uses means a store can be eliminated, but GEP-derived
pointers can still access the memory location. This needs proper alias analysis
or conservative handling.

## Current State

- ❌ Unsound optimization that could eliminate live stores
- ⚠️ TODO comment acknowledges the issue
- ❌ No alias analysis implementation

## Affected Code

### Location: `crates/compiler/mir/src/passes/pre_opt.rs`

#### Line 217 - The TODO

```rust
// TODO: Enhance with alias analysis to handle GEP-derived pointers more aggressively
```

#### The Unsound Pattern

```rust
// Current implementation assumes:
// if store.uses().count() == 0 => store is dead
//
// But this is WRONG for:
// %ptr = alloc
// %gep = get_element_ptr %ptr, 0
// store %gep, value    ; <- zero direct uses but accessed via GEP!
// %loaded = load %gep  ; <- uses GEP, not store
```

## The Problem

### Example of Miscompilation

```rust
// MIR that could be miscompiled:
%base = alloc [2 x i32]
%ptr1 = get_element_ptr %base, 0
%ptr2 = get_element_ptr %base, 1
store %ptr1, 42  // Could be incorrectly eliminated!
store %ptr2, 100
%val = load %ptr1  // Would load uninitialized memory
```

### Root Cause

The pass doesn't track that:

1. GEP creates derived pointers to same memory
2. Stores through derived pointers affect base allocation
3. Loads through any derived pointer can observe stores through others

## Solution Options

### Option A: Conservative Fix (Quick, Safe)

Disable dead store elimination for any allocation that has GEP operations:

```rust
fn is_store_dead(&self, store: &Instruction) -> bool {
    let store_ptr = store.get_store_pointer();

    // Find the base allocation
    let base_alloc = self.find_base_allocation(store_ptr);

    // Conservative: if allocation has any GEPs, keep all stores
    if self.allocation_has_geps(base_alloc) {
        return false;
    }

    // Original logic for simple cases
    store.uses().count() == 0
}
```

### Option B: Basic Alias Analysis (Better Performance)

Implement must-alias/may-alias analysis:

```rust
pub struct AliasAnalysis {
    // Track which pointers may alias
    may_alias: HashMap<ValueId, HashSet<ValueId>>,
}

impl AliasAnalysis {
    fn may_alias(&self, ptr1: ValueId, ptr2: ValueId) -> bool {
        // Same pointer always aliases with itself
        if ptr1 == ptr2 { return true; }

        // Check if derived from same allocation
        let base1 = self.get_base_allocation(ptr1);
        let base2 = self.get_base_allocation(ptr2);

        base1 == base2
    }
}
```

### Option C: Full Data-Flow Analysis (Most Precise)

Track points-to sets and implement proper alias analysis:

```rust
pub struct PointsToAnalysis {
    // Maps each pointer to set of possible memory locations
    points_to: HashMap<ValueId, HashSet<MemoryLocation>>,
}
```

## Recommended Implementation (Option B)

### Step 1: Create Alias Analysis Module

Create `crates/compiler/mir/src/analysis/alias.rs`:

```rust
use crate::*;

pub struct AliasAnalysis<'a> {
    func: &'a Function,
    base_pointers: HashMap<ValueId, ValueId>,
}

impl<'a> AliasAnalysis<'a> {
    pub fn new(func: &'a Function) -> Self {
        let mut analysis = Self {
            func,
            base_pointers: HashMap::new(),
        };
        analysis.compute_base_pointers();
        analysis
    }

    fn compute_base_pointers(&mut self) {
        for inst in self.func.instructions() {
            if let InstructionKind::GetElementPtr { base, .. } = &inst.kind {
                self.base_pointers.insert(inst.id, *base);
            }
        }
    }

    pub fn may_alias(&self, ptr1: ValueId, ptr2: ValueId) -> bool {
        self.get_base(ptr1) == self.get_base(ptr2)
    }

    fn get_base(&self, ptr: ValueId) -> ValueId {
        let mut current = ptr;
        while let Some(&base) = self.base_pointers.get(&current) {
            current = base;
        }
        current
    }
}
```

### Step 2: Update Dead Store Elimination

```rust
impl PreOptPass {
    fn eliminate_dead_stores(&mut self, func: &mut Function) -> bool {
        let alias_analysis = AliasAnalysis::new(func);
        let mut changed = false;

        for inst in func.instructions_mut() {
            if let InstructionKind::Store { ptr, .. } = &inst.kind {
                if self.is_store_dead(inst, &alias_analysis) {
                    inst.mark_for_removal();
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_store_dead(
        &self,
        store: &Instruction,
        alias: &AliasAnalysis,
    ) -> bool {
        let store_ptr = store.get_store_pointer();

        // Check if any subsequent instruction could observe this store
        for inst in store.successors() {
            match &inst.kind {
                InstructionKind::Load { ptr } => {
                    if alias.may_alias(store_ptr, *ptr) {
                        return false; // Store is live
                    }
                }
                InstructionKind::Store { ptr, .. } => {
                    if alias.may_alias(store_ptr, *ptr) {
                        // Later store kills this one only if must-alias
                        // For now, be conservative
                        return false;
                    }
                }
                _ => {}
            }
        }

        true // No observing loads found
    }
}
```

## Testing Requirements

### Correctness Tests

```rust
#[test]
fn test_gep_derived_stores_not_eliminated() {
    // Test that stores through GEP-derived pointers aren't eliminated
    let mir = r#"
        %base = alloc [2 x i32]
        %ptr = get_element_ptr %base, 0
        store %ptr, 42  ; Should NOT be eliminated
        %val = load %ptr
    "#;

    let optimized = run_pre_opt(mir);
    assert!(optimized.contains("store"));
}

#[test]
fn test_independent_stores_eliminated() {
    // Test that truly dead stores are still eliminated
}

#[test]
fn test_aliasing_stores_preserved() {
    // Test that potentially aliasing stores are preserved
}
```

## Verification Checklist

- [ ] Alias analysis correctly identifies related pointers
- [ ] GEP-derived stores are not incorrectly eliminated
- [ ] Independent dead stores are still eliminated
- [ ] No miscompilations in test suite
- [ ] Performance impact measured and acceptable

## Success Criteria

1. No unsound optimizations
2. Test suite passes without miscompilations
3. Reasonable compile-time overhead (<5%)
4. Clear documentation of limitations
5. TODO comment removed or updated
