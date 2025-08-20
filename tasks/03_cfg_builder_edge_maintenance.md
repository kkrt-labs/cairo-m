# Task 3: Update CfgBuilder for Edge Maintenance

## Goal

Ensure `CfgBuilder` automatically maintains explicit pred/succ edges when
setting terminators.

## Files to Modify

- `mir/src/builder/cfg_builder.rs` - Primary changes

## Current State

`CfgBuilder` sets terminators but doesn't update the explicit pred/succ edges
added in Task 1.

## Required Changes

### 1. Update Terminator Setting Methods

Modify these methods in `CfgBuilder` to call `MirFunction::connect()`:

```rust
impl<'f> CfgBuilder<'f> {
    /// Terminates the current block with the given terminator
    /// NOW ALSO: Updates pred/succ edges based on terminator targets
    pub fn terminate(&mut self, terminator: Terminator) -> CfgState {
        if self.is_terminated() {
            panic!("Attempting to terminate an already terminated block");
        }

        // NEW: Connect edges to all target blocks
        let target_blocks = terminator.target_blocks();
        for target in target_blocks {
            self.function.connect(self.current_block_id, target);
        }

        let block = self.current_block_mut();
        block.set_terminator(terminator);
        self.is_terminated = true;
        self.state()
    }
}
```

### 2. Update Block Terminator Replacement

Modify `set_block_terminator()` to handle edge updates for already-terminated
blocks:

```rust
impl<'f> CfgBuilder<'f> {
    /// Sets the terminator for a specific block
    /// NOW ALSO: Updates edges when replacing an existing terminator
    pub fn set_block_terminator(&mut self, block_id: BasicBlockId, terminator: Terminator) {
        if let Some(block) = self.get_block_mut(block_id) {
            // If block already has a terminator, remove old edges
            if block.has_terminator() {
                let old_targets = block.terminator.target_blocks();
                for old_target in old_targets {
                    self.function.disconnect(block_id, old_target);
                }
            }

            // Set new terminator
            block.set_terminator(terminator.clone());

            // Connect new edges
            let new_targets = terminator.target_blocks();
            for new_target in new_targets {
                self.function.connect(block_id, new_target);
            }
        }
    }
}
```

### 3. Add Edge Maintenance to Utility Methods

Update methods that modify control flow:

```rust
impl<'f> CfgBuilder<'f> {
    /// Switches to a block and optionally marks the previous block as filled
    pub fn switch_to_block(&mut self, block_id: BasicBlockId) -> CfgState {
        // NEW: Optionally mark the current block as filled before switching
        // This can be configurable based on SSA construction needs
        self.current_block_id = block_id;
        self.is_terminated = false;
        self.state()
    }

    /// Mark a block as filled (all local statements processed)
    pub fn mark_block_filled(&mut self, block_id: BasicBlockId) {
        if let Some(block) = self.get_block_mut(block_id) {
            block.mark_filled();
        }
    }

    /// Mark a block as sealed (no more predecessors)
    /// This is used by SSA construction - when called, it means the predecessor set is final
    pub fn seal_block(&mut self, block_id: BasicBlockId) {
        let block = self.get_block_mut(block_id).unwrap_or_else(|| panic!("Block {:?} not found", block_id));
        block.seal();
        // NOTE: SSA builder will also need to track sealed blocks in its own set
        // This method is just for marking the BasicBlock itself
    }
}
```

### 4. Update Critical Edge Splitting

Since `cfg.rs::split_critical_edge` will use this builder, ensure it maintains
edges:

The existing `split_critical_edge` function in `cfg.rs` should be updated to use
the new edge maintenance:

```rust
// This change goes in cfg.rs, but CfgBuilder should support it
pub fn split_critical_edge(
    function: &mut MirFunction,
    pred_id: BasicBlockId,
    succ_id: BasicBlockId,
) -> BasicBlockId {
    // Create edge block
    let edge_block = BasicBlock {
        name: Some(format!("edge_{:?}_{:?}", pred_id, succ_id)),
        instructions: Vec::new(),
        terminator: Terminator::Jump { target: succ_id },
        preds: Vec::new(),
        succs: Vec::new(),
        sealed: false,
        filled: false,
    };
    let edge_block_id = function.basic_blocks.push(edge_block);

    // Update edges using new infrastructure
    function.replace_edge(pred_id, succ_id, edge_block_id);
    function.connect(edge_block_id, succ_id);

    // Update predecessor's terminator
    if let Some(pred_block) = function.basic_blocks.get_mut(pred_id) {
        pred_block.terminator.replace_target(succ_id, edge_block_id);
    }

    edge_block_id
}
```

### 5. Validation Helper

Add method to verify edge consistency during building:

```rust
impl<'f> CfgBuilder<'f> {
    /// Debug helper: verify edge consistency
    #[cfg(debug_assertions)]
    pub fn validate_edges(&self) -> Result<(), String> {
        self.function.validate()
    }
}
```

## Legacy Code to Remove

AFTER this task completes:

- None (this modifies existing methods but doesn't remove them)

## Integration Points

### Update cfg.rs Functions

These functions in `cfg.rs` should be updated to use the new edge
infrastructure:

1. `split_critical_edge()` - use `function.replace_edge()` and
   `function.connect()`
2. `split_all_critical_edges()` - will benefit from above change

### Maintain Backward Compatibility

Existing `get_predecessors()` and `get_successors()` functions in `cfg.rs`
should be updated to use the explicit edge storage:

```rust
// In cfg.rs - replace the recomputation with direct access
pub fn get_successors(function: &MirFunction, block_id: BasicBlockId) -> Vec<BasicBlockId> {
    let block = function.basic_blocks.get(block_id).unwrap_or_else(|| panic!("Block {:?} not found", block_id));
    block.succs.clone()
}

pub fn get_predecessors(function: &MirFunction, target_id: BasicBlockId) -> Vec<BasicBlockId> {
    let block = function.basic_blocks.get(target_id).unwrap_or_else(|| panic!("Block {:?} not found", target_id));
    block.preds.clone()
}
```

## Testing

- Test that setting terminators creates correct pred/succ edges
- Test that replacing terminators updates edges correctly
- Test critical edge splitting with new edge maintenance
- Verify existing CFG tests still pass
- Add edge consistency checks to CfgBuilder tests

## Success Criteria

- ✅ All terminator-setting methods in `CfgBuilder` update pred/succ edges
- ✅ `set_block_terminator()` correctly handles edge replacement
- ✅ Critical edge splitting maintains edge consistency
- ✅ `cfg.rs` functions use explicit edges instead of recomputation
- ✅ Edge consistency validation works
- ✅ All existing tests pass
