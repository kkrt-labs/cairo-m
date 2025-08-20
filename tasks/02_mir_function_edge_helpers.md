# Task 2: Add Edge Maintenance to MirFunction

## Goal

Add centralized edge maintenance and def-use utilities to `MirFunction` to
support SSA construction.

## Files to Modify

- `mir/src/function.rs` - Primary changes

## Current State

`MirFunction` has basic block management but no centralized edge maintenance or
def-use rewriting.

## Required Changes

### 1. Add Edge Maintenance Methods

Add these methods to `impl MirFunction`:

```rust
impl MirFunction {
    /// Connect two blocks by adding pred/succ edges
    /// This is the canonical way to add CFG edges
    pub fn connect(&mut self, pred: BasicBlockId, succ: BasicBlockId) {
        // Get mutable reference to predecessor block - panic if not found
        let pred_block = self.basic_blocks.get_mut(pred)
            .unwrap_or_else(|| panic!("Predecessor block {:?} does not exist", pred));
        pred_block.add_succ(succ);

        // Get mutable reference to successor block - panic if not found
        let succ_block = self.basic_blocks.get_mut(succ)
            .unwrap_or_else(|| panic!("Successor block {:?} does not exist", succ));
        succ_block.add_pred(pred);
    }

    /// Replace an edge from pred->old_succ with pred->new_succ
    /// Updates both pred's succ list and old_succ/new_succ pred lists
    pub fn replace_edge(&mut self, pred: BasicBlockId, old_succ: BasicBlockId, new_succ: BasicBlockId) {
        // Get mutable reference to predecessor block - panic if not found
        let pred_block = self.basic_blocks.get_mut(pred)
            .unwrap_or_else(|| panic!("Predecessor block {:?} does not exist", pred));
        pred_block.remove_succ(old_succ);

        // Get mutable reference to old successor block - panic if not found
        let old_succ_block = self.basic_blocks.get_mut(old_succ)
            .unwrap_or_else(|| panic!("Old successor block {:?} does not exist", old_succ));
        old_succ_block.remove_pred(pred);

        // Add new edge
        self.connect(pred, new_succ);
    }

    /// Disconnect two blocks by removing pred/succ edges
    pub fn disconnect(&mut self, pred: BasicBlockId, succ: BasicBlockId) {
        // Get mutable reference to predecessor block - panic if not found
        let pred_block = self.basic_blocks.get_mut(pred)
            .unwrap_or_else(|| panic!("Predecessor block {:?} does not exist", pred));
        pred_block.remove_succ(succ);

        // Get mutable reference to successor block - panic if not found
        let succ_block = self.basic_blocks.get_mut(succ)
            .unwrap_or_else(|| panic!("Successor block {:?} does not exist", succ));
        succ_block.remove_pred(pred);
    }
}
```

### 2. Add Def-Use Rewriting Support

Add method to replace all uses of one value with another:

```rust
impl MirFunction {
    /// Replace all occurrences of `from` value with `to` value throughout the function
    /// This is needed for trivial phi elimination
    pub fn replace_all_uses(&mut self, from: ValueId, to: ValueId) {
        if from == to {
            return; // No-op
        }

        for (_block_id, block) in self.basic_blocks.iter_enumerated_mut() {
            // Replace in all instructions
            for instruction in &mut block.instructions {
                instruction.replace_value_uses(from, to);
            }

            // Replace in terminator
            block.terminator.replace_value_uses(from, to);
        }

        // Update parameter list if needed
        for param in &mut self.parameters {
            if *param == from {
                *param = to;
            }
        }

        // Update return values if needed
        for ret_val in &mut self.return_values {
            if *ret_val == from {
                *ret_val = to;
            }
        }

        // Remove the old value from type information
        if let Some(ty) = self.value_types.remove(&from) {
            // If `to` doesn't have a type, give it the type from `from`
            self.value_types.entry(to).or_insert(ty);
        }

        // Remove from defined_values
        self.defined_values.remove(&from);
    }
}
```

### 3. Add PHI Creation Helper

Add convenience method for creating phi instructions:

```rust
impl MirFunction {
    /// Create a new phi instruction at the front of the given block
    /// Returns the destination ValueId
    pub fn new_phi(&mut self, block_id: BasicBlockId, ty: MirType) -> ValueId {
        let dest = self.new_typed_value_id(ty.clone());
        let phi_instr = Instruction {
            kind: InstructionKind::Phi {
                dest,
                ty,
                sources: Vec::new(), // Initially empty, filled later
            },
            comment: None,
            span: None,
        };

        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.push_phi_front(phi_instr);
        }

        dest
    }
}
```

### 4. Update Validation

Extend `MirFunction::validate()` to check edge consistency:

```rust
impl MirFunction {
    pub fn validate(&self) -> Result<(), String> {
        // ... existing validation ...

        // NEW: Validate edge consistency
        for (block_id, block) in self.basic_blocks() {
            // Check that all successors in terminator match succ list
            let terminator_targets = block.terminator.target_blocks();
            for target in &terminator_targets {
                if !block.succs.contains(target) {
                    return Err(format!(
                        "Block {:?} has terminator target {:?} not in succ list",
                        block_id, target
                    ));
                }
            }

            // Check that succ list only contains terminator targets
            for succ in &block.succs {
                if !terminator_targets.contains(succ) {
                    return Err(format!(
                        "Block {:?} has succ {:?} not in terminator targets",
                        block_id, succ
                    ));
                }
            }

            // Check that all predecessors have this block in their succ list
            for pred in &block.preds {
                if let Some(pred_block) = self.basic_blocks.get(*pred) {
                    if !pred_block.succs.contains(&block_id) {
                        return Err(format!(
                            "Block {:?} claims {:?} as predecessor, but {:?} doesn't have it as successor",
                            block_id, pred, pred
                        ));
                    }
                } else {
                    return Err(format!(
                        "Block {:?} has non-existent predecessor {:?}",
                        block_id, pred
                    ));
                }
            }

            // Check that all successors have this block in their pred list
            for succ in &block.succs {
                if let Some(succ_block) = self.basic_blocks.get(*succ) {
                    if !succ_block.preds.contains(&block_id) {
                        return Err(format!(
                            "Block {:?} claims {:?} as successor, but {:?} doesn't have it as predecessor",
                            block_id, succ, succ
                        ));
                    }
                } else {
                    return Err(format!(
                        "Block {:?} has non-existent successor {:?}",
                        block_id, succ
                    ));
                }
            }
        }

        Ok(())
    }
}
```

## Required Dependencies

### Add Methods to Instruction and Terminator

These need to be added for `replace_all_uses` to work:

In `instruction.rs`:

```rust
impl Instruction {
    pub fn replace_value_uses(&mut self, from: ValueId, to: ValueId) {
        // Implementation depends on InstructionKind variants
        // Replace any Value::Operand(from) with Value::Operand(to)
    }
}
```

In `terminator.rs`:

```rust
impl Terminator {
    pub fn replace_value_uses(&mut self, from: ValueId, to: ValueId) {
        // Implementation depends on Terminator variants
        // Replace any Value::Operand(from) with Value::Operand(to)
    }
}
```

### Add Phi Placement Helper to BasicBlock

In `basic_block.rs`:

```rust
impl BasicBlock {
    /// Insert a phi instruction at the front of the block (after existing phis)
    pub fn push_phi_front(&mut self, instruction: Instruction) {
        // Find where to insert (after existing phi instructions)
        let insert_pos = self.instructions.iter()
            .position(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());
        self.instructions.insert(insert_pos, instruction);
    }

    /// Get the range of phi instructions at the start of this block
    pub fn phi_range(&self) -> std::ops::Range<usize> {
        let end = self.instructions.iter()
            .position(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());
        0..end
    }
}
```

## Legacy Code to Remove

AFTER this task completes:

- None (this is purely additive)

## Testing

- Unit tests for edge maintenance methods
- Test edge consistency validation
- Test `replace_all_uses` with various instruction types
- Test phi creation and placement

## Success Criteria

- ✅ `MirFunction::connect()` and `replace_edge()` maintain bidirectional edges
- ✅ `MirFunction::replace_all_uses()` correctly updates all value references
- ✅ `MirFunction::new_phi()` creates phi instructions at block start
- ✅ Validation checks edge consistency
- ✅ All tests pass
