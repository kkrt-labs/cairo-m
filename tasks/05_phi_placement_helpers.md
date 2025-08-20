# Task 5: Add Phi Placement and Validation Helpers

## Goal

Add utilities for phi instruction placement, ordering, and validation to support
SSA construction.

## Files to Modify

- `mir/src/basic_block.rs` - Add phi placement helpers
- `mir/src/instruction.rs` - Add phi helper methods
- `mir/src/function.rs` - Update validation for phi ordering

## Current State

Phi instructions exist but there's no enforcement of phi-first ordering or
placement utilities.

## Required Changes

### 1. Add Phi Placement Methods to BasicBlock

Add to `impl BasicBlock` in `basic_block.rs`:

```rust
impl BasicBlock {
    /// Insert a phi instruction at the front of the block (after existing phis)
    /// Maintains the invariant that all phi instructions come first
    pub fn push_phi_front(&mut self, instruction: Instruction) {
        // Verify this is actually a phi instruction
        if !matches!(instruction.kind, InstructionKind::Phi { .. }) {
            panic!("push_phi_front called with non-phi instruction");
        }

        // Find insertion point (after existing phis)
        let insert_pos = self.instructions.iter()
            .position(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());

        self.instructions.insert(insert_pos, instruction);
    }

    /// Get the range of phi instructions at the start of this block
    /// Returns range [0, n) where n is the first non-phi instruction index
    pub fn phi_range(&self) -> std::ops::Range<usize> {
        let end = self.instructions.iter()
            .position(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }))
            .unwrap_or(self.instructions.len());
        0..end
    }

    /// Get all phi instructions in this block
    pub fn phi_instructions(&self) -> impl Iterator<Item = &Instruction> {
        let range = self.phi_range();
        self.instructions[range].iter()
    }

    /// Get all phi instructions mutably
    pub fn phi_instructions_mut(&mut self) -> impl Iterator<Item = &mut Instruction> {
        let range = self.phi_range();
        self.instructions[range].iter_mut()
    }

    /// Get all non-phi instructions in this block
    pub fn non_phi_instructions(&self) -> impl Iterator<Item = &Instruction> {
        let range = self.phi_range();
        self.instructions[range.end..].iter()
    }

    /// Count phi instructions
    pub fn phi_count(&self) -> usize {
        self.phi_range().len()
    }

    /// Check if this block has any phi instructions
    pub fn has_phis(&self) -> bool {
        self.phi_count() > 0
    }

    /// Find a phi instruction by its destination ValueId
    pub fn find_phi(&self, dest: ValueId) -> Option<&Instruction> {
        self.phi_instructions().find(|instr| {
            if let InstructionKind::Phi { dest: phi_dest, .. } = &instr.kind {
                *phi_dest == dest
            } else {
                false
            }
        })
    }

    /// Find a phi instruction mutably by its destination ValueId
    pub fn find_phi_mut(&mut self, dest: ValueId) -> Option<&mut Instruction> {
        self.phi_instructions_mut().find(|instr| {
            if let InstructionKind::Phi { dest: phi_dest, .. } = &instr.kind {
                *phi_dest == dest
            } else {
                false
            }
        })
    }

    /// Remove a phi instruction by its destination ValueId
    /// Returns true if a phi was removed
    pub fn remove_phi(&mut self, dest: ValueId) -> bool {
        let range = self.phi_range();
        if let Some(pos) = self.instructions[range.clone()].iter().position(|instr| {
            if let InstructionKind::Phi { dest: phi_dest, .. } = &instr.kind {
                *phi_dest == dest
            } else {
                false
            }
        }) {
            self.instructions.remove(range.start + pos);
            true
        } else {
            false
        }
    }
}
```

### 2. Add Phi Helper Methods to Instruction

Add to `impl Instruction` in `instruction.rs`:

```rust
impl Instruction {
    /// Create a new phi instruction
    pub fn phi(dest: ValueId, ty: MirType, sources: Vec<(BasicBlockId, Value)>) -> Self {
        Self {
            kind: InstructionKind::Phi { dest, ty, sources },
            comment: None,
            span: None,
        }
    }

    /// Create an empty phi instruction (operands to be filled later)
    pub fn empty_phi(dest: ValueId, ty: MirType) -> Self {
        Self::phi(dest, ty, Vec::new())
    }

    /// Check if this instruction is a phi
    pub fn is_phi(&self) -> bool {
        matches!(self.kind, InstructionKind::Phi { .. })
    }

    /// Get phi operands if this is a phi instruction
    pub fn phi_operands(&self) -> Option<&[(BasicBlockId, Value)]> {
        if let InstructionKind::Phi { sources, .. } = &self.kind {
            Some(sources)
        } else {
            None
        }
    }

    /// Get phi operands mutably if this is a phi instruction
    pub fn phi_operands_mut(&mut self) -> Option<&mut Vec<(BasicBlockId, Value)>> {
        if let InstructionKind::Phi { sources, .. } = &mut self.kind {
            Some(sources)
        } else {
            None
        }
    }

    /// Add an operand to a phi instruction
    /// Returns true if operand was added, false if not a phi
    pub fn add_phi_operand(&mut self, block: BasicBlockId, value: Value) -> bool {
        if let Some(sources) = self.phi_operands_mut() {
            sources.push((block, value));
            true
        } else {
            false
        }
    }

    /// Set all phi operands at once
    /// Returns true if successful, false if not a phi
    pub fn set_phi_operands(&mut self, operands: Vec<(BasicBlockId, Value)>) -> bool {
        if let Some(sources) = self.phi_operands_mut() {
            *sources = operands;
            true
        } else {
            false
        }
    }
}
```

### 3. Add Phi-Specific Validation to MirFunction

Extend `MirFunction::validate()` in `function.rs`:

```rust
impl MirFunction {
    pub fn validate(&self) -> Result<(), String> {
        // ... existing validation ...

        // NEW: Validate phi instruction placement and consistency
        for (block_id, block) in self.basic_blocks() {
            // Check phi-first invariant
            let mut seen_non_phi = false;
            for (i, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    InstructionKind::Phi { .. } => {
                        if seen_non_phi {
                            return Err(format!(
                                "Block {:?}: Phi instruction at position {} found after non-phi instruction",
                                block_id, i
                            ));
                        }
                    }
                    _ => {
                        seen_non_phi = true;
                    }
                }
            }

            // Validate each phi instruction
            for instruction in block.phi_instructions() {
                if let InstructionKind::Phi { dest, sources, ty } = &instruction.kind {
                    // Check that destination is defined exactly once
                    if !self.defined_values.contains(dest) {
                        return Err(format!(
                            "Block {:?}: Phi instruction destination {:?} not in defined_values",
                            block_id, dest
                        ));
                    }

                    // Check that each source block is actually a predecessor
                    for (source_block, _value) in sources {
                        if !block.preds.contains(source_block) {
                            return Err(format!(
                                "Block {:?}: Phi instruction has operand from block {:?} which is not a predecessor",
                                block_id, source_block
                            ));
                        }
                    }

                    // Check that we have operands from all predecessors (if sealed)
                    if block.sealed && sources.len() != block.preds.len() {
                        return Err(format!(
                            "Block {:?}: Sealed block has {} predecessors but phi has {} operands",
                            block_id, block.preds.len(), sources.len()
                        ));
                    }

                    // Check that destination type matches phi type
                    if let Some(dest_type) = self.get_value_type(*dest) {
                        if dest_type != ty {
                            return Err(format!(
                                "Block {:?}: Phi destination type {:?} doesn't match instruction type {:?}",
                                block_id, dest_type, ty
                            ));
                        }
                    }
                } else {
                    // Should never happen due to phi_instructions() filtering
                    unreachable!("Non-phi in phi_instructions()");
                }
            }
        }

        Ok(())
    }
}
```

### 4. Update MirFunction Phi Creation Helper

Update the `new_phi` method added in Task 2:

```rust
impl MirFunction {
    /// Create a new phi instruction at the front of the given block
    /// Returns the destination ValueId
    pub fn new_phi(&mut self, block_id: BasicBlockId, ty: MirType) -> ValueId {
        let dest = self.new_typed_value_id(ty.clone());

        // Mark as defined for SSA validation
        self.mark_as_defined(dest).expect("Phi destination should be unique");

        let phi_instr = Instruction::empty_phi(dest, ty);

        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.push_phi_front(phi_instr);
        }

        dest
    }

    /// Create a phi instruction with specific operands
    pub fn new_phi_with_operands(
        &mut self,
        block_id: BasicBlockId,
        ty: MirType,
        operands: Vec<(BasicBlockId, Value)>
    ) -> ValueId {
        let dest = self.new_typed_value_id(ty.clone());

        // Mark as defined for SSA validation
        self.mark_as_defined(dest).expect("Phi destination should be unique");

        let phi_instr = Instruction::phi(dest, ty, operands);

        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.push_phi_front(phi_instr);
        }

        dest
    }
}
```

## Legacy Code to Remove

AFTER this task completes:

- None (this is purely additive)

## Testing

- Test phi placement maintains ordering invariant
- Test phi validation catches ordering violations
- Test phi operand manipulation methods
- Test phi creation and removal
- Test validation of phi-predecessor relationships
- Test that regular instruction placement still works

## Success Criteria

- ✅ Phi instructions are always placed at block start
- ✅ `push_phi_front()` maintains phi-first ordering
- ✅ Validation enforces phi placement invariants
- ✅ Phi helper methods work correctly
- ✅ Existing instruction placement unaffected
- ✅ All tests pass
