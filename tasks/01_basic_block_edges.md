# Task 1: Add Explicit Edge Storage to BasicBlock

## Goal

Add explicit predecessor/successor storage and SSA-related states to
`BasicBlock` structure.

## Files to Modify

- `mir/src/basic_block.rs` - Primary changes
- `mir/src/function.rs` - Update validation if needed

## Current State

```rust
// In basic_block.rs
pub struct BasicBlock {
    pub name: Option<String>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}
```

## Required Changes

### 1. Extend BasicBlock Structure

Add these fields to `BasicBlock`:

```rust
pub struct BasicBlock {
    pub name: Option<String>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,

    // NEW: Explicit CFG edges
    pub preds: Vec<BasicBlockId>,
    pub succs: Vec<BasicBlockId>,

    // NEW: SSA construction states
    pub sealed: bool,   // Final set of predecessors known (Braun)
    pub filled: bool,   // All statements locally processed (Braun §2.1)
}
```

### 2. Add Helper Methods

Add these methods to `impl BasicBlock`:

```rust
impl BasicBlock {
    /// Add a predecessor, avoiding duplicates
    pub fn add_pred(&mut self, pred: BasicBlockId) {
        // Can only add pred if the block is not sealed.
        if self.sealed {
            panic!("Can only add pred if the block is not sealed");
        }
        if !self.preds.contains(&pred) {
            self.preds.push(pred);
        }
    }

    /// Add a successor, avoiding duplicates
    pub fn add_succ(&mut self, succ: BasicBlockId) {
        // Can only add succ if the block is filled.
        if !self.filled {
            panic!("Can only add succ if the block is filled");
        }
        if !self.succs.contains(&succ) {
            self.succs.push(succ);
        }
    }

    /// Remove a predecessor
    pub fn remove_pred(&mut self, pred: BasicBlockId) {
        self.preds.retain(|&p| p != pred);
    }

    /// Remove a successor
    pub fn remove_succ(&mut self, succ: BasicBlockId) {
        self.succs.retain(|&s| s != succ);
    }

    /// Mark this block as sealed (no more predecessors will be added)
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Mark this block as filled (all local statements processed)
    pub fn mark_filled(&mut self) {
        self.filled = true;
    }

    /// Rename the existing method for clarity
    pub fn has_terminator(&self) -> bool {
        !matches!(self.terminator, Terminator::Unreachable)
    }
}
```

### 3. Update Constructors

Update all `BasicBlock` constructors to initialize new fields:

```rust
impl BasicBlock {
    pub const fn new() -> Self {
        Self {
            name: None,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            preds: Vec::new(),
            succs: Vec::new(),
            sealed: false,
            filled: false,
        }
    }

    pub const fn with_name(name: String) -> Self {
        Self {
            name: Some(name),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            preds: Vec::new(),
            succs: Vec::new(),
            sealed: false,
            filled: false,
        }
    }

    pub const fn with_terminator(terminator: Terminator) -> Self {
        Self {
            name: None,
            instructions: Vec::new(),
            terminator,
            preds: Vec::new(),
            succs: Vec::new(),
            sealed: false,
            filled: false,
        }
    }
}
```

### 4. Rename Method (Optional but Recommended)

Change `is_terminated() -> bool` to `has_terminator() -> bool` to avoid
confusion with sealed state.

### 5. Update Validation

In `validate()` method, add checks:

- Verify sealed blocks don't get new predecessors
- Verify filled blocks maintain their invariants

## Legacy Code to Remove

AFTER this task completes:

- None (this is purely additive)

## Dependencies

- Requires updating any code that constructs `BasicBlock` directly (should be
  minimal since constructors are used)
- May need to update serialization derives if present

## Testing

- Unit tests for new helper methods
- Verify existing tests still pass
- Add tests for sealed/filled state tracking

## Success Criteria

- ✅ `BasicBlock` has explicit `preds`/`succs` vectors
- ✅ `BasicBlock` has `sealed` and `filled` boolean states
- ✅ All constructors initialize new fields correctly
- ✅ Helper methods work correctly with deduplication
- ✅ Existing functionality unchanged
- ✅ All tests pass
