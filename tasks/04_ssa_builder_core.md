# Task 4: Create SSA Builder with Braun Algorithm

## Goal

Implement the core SSA builder that implements Braun et al.'s algorithm with
per-block variable tracking.

## Files to Create

- `mir/src/ssa.rs` - New module (main implementation)

## Files to Modify

- `mir/src/lib.rs` - Add `pub mod ssa;`

## Current State

Variable tracking uses global
`definition_to_value: FxHashMap<MirDefinitionId, ValueId>` in `MirState`.

## Required Changes

### 1. Create SSA Builder Module (`mir/src/ssa.rs`)

```rust
//! # SSA Builder - Braun et al. Algorithm Implementation
//!
//! This module implements the SSA construction algorithm from Braun et al.
//! "Simple and Efficient Construction of Static Single Assignment Form".
//!
//! Key concepts from the paper:
//! - currentDef[var][block] -> ValueId (Algorithm 1)
//! - incompletePhis[block][variable] -> ValueId (Algorithm 2)
//! - sealedBlocks: set of blocks whose predecessor set is final (Algorithm 2)
//! - Incomplete phi nodes: created for unsealed blocks, completed on sealing (Algorithm 4)

use rustc_hash::FxHashMap;
use std::collections::HashSet;
use crate::{BasicBlockId, MirDefinitionId, MirFunction, MirType, Value, ValueId, Instruction, InstructionKind};

/// SSA Builder implementing Braun et al.'s algorithm
///
/// This builder maintains per-block variable definitions and handles
/// phi node creation, sealing, and trivial phi elimination.
pub struct SSABuilder<'f> {
    func: &'f mut MirFunction,

    // currentDef[var][block] -> ValueId (Algorithm 1)
    current_def: FxHashMap<(MirDefinitionId, BasicBlockId), ValueId>,

    // incompletePhis[block][variable] -> ValueId (Algorithm 2)
    incomplete_phis: FxHashMap<BasicBlockId, FxHashMap<MirDefinitionId, ValueId>>,

    // Track which blocks are sealed (sealedBlocks in paper)
    sealed_blocks: std::collections::HashSet<BasicBlockId>,
}

impl<'f> SSABuilder<'f> {
    /// Create a new SSA builder for the given function
    pub fn new(func: &'f mut MirFunction) -> Self {
        Self {
            func,
            current_def: FxHashMap::default(),
            incomplete_phis: FxHashMap::default(),
            sealed_blocks: std::collections::HashSet::new(),
        }
    }

    /// Write a variable in a block (Algorithm 1, writeVariable)
    pub fn write_variable(&mut self, var: MirDefinitionId, block: BasicBlockId, value: ValueId) {
        self.current_def.insert((var, block), value);
    }

    /// Read a variable from a block (Algorithm 1, readVariable)
    pub fn read_variable(&mut self, var: MirDefinitionId, block: BasicBlockId) -> ValueId {
        if let Some(&value) = self.current_def.get(&(var, block)) {
            return value;
        }

        // Variable not defined locally, use recursive algorithm
        self.read_variable_recursive(var, block)
    }

    /// Recursive variable reading (Algorithm 2, readVariableRecursive)
    fn read_variable_recursive(&mut self, var: MirDefinitionId, block: BasicBlockId) -> ValueId {
        if !self.sealed_blocks.contains(&block) {
            // Incomplete CFG: val ← new Phi(block)
            let val = self.new_phi(block, var);
            // incompletePhis[block][variable] ← val
            self.incomplete_phis.entry(block).or_default().insert(var, val);
            val
        } else if self.func.basic_blocks[block].preds.len() == 1 {
            // Optimize the common case of one predecessor: No phi needed
            // val ← readVariable(variable, block.preds[0])
            let pred = self.func.basic_blocks[block].preds[0];
            let val = self.read_variable(var, pred);
            val
        } else {
            // Break potential cycles with operandless phi
            // val ← new Phi(block)
            let val = self.new_phi(block, var);
            // writeVariable(variable, block, val)
            self.write_variable(var, block, val);
            // val ← addPhiOperands(variable, val)
            let val = self.add_phi_operands(var, val);
            // writeVariable(variable, block, val)
            self.write_variable(var, block, val);
            val
        }
    }

    /// Create a new phi instruction (helper for both complete and incomplete phis)
    fn new_phi(&mut self, block: BasicBlockId, var: MirDefinitionId) -> ValueId {
        let var_type = self.get_variable_type(var);
        self.func.new_phi(block, var_type)
    }

    /// Add phi operands from all predecessors (Algorithm 2, addPhiOperands)
    fn add_phi_operands(&mut self, var: MirDefinitionId, phi: ValueId) -> ValueId {
        // Get the block containing this phi
        let phi_block = self.find_phi_block(phi).expect("Phi must exist in a block");

        // Determine operands from predecessors
        let preds = self.func.basic_blocks[phi_block].preds.clone();
        let mut sources = Vec::new();

        for pred in preds {
            let operand = self.read_variable(var, pred);
            sources.push((pred, Value::Operand(operand)));
        }

        // Update phi instruction with operands
        self.update_phi_operands(phi, sources);

        // Try to remove trivial phi and return result
        self.try_remove_trivial_phi(phi)
    }

    /// Seal a block: complete all incomplete phi nodes (Algorithm 4, sealBlock)
    pub fn seal_block(&mut self, block: BasicBlockId) {
        // for variable in incompletePhis[block]:
        if let Some(incomplete_block_phis) = self.incomplete_phis.remove(&block) {
            for (variable, phi_value) in incomplete_block_phis {
                // addPhiOperands(variable, incompletePhis[block][variable])
                let final_value = self.add_phi_operands(variable, phi_value);
                // Update current definition if phi was eliminated
                if final_value != phi_value {
                    self.write_variable(variable, block, final_value);
                }
            }
        }

        // sealedBlocks.add(block)
        self.sealed_blocks.insert(block);

        // Also mark block as sealed in the MirFunction (for consistency)
        if let Some(block_ref) = self.func.basic_blocks.get_mut(block) {
            block_ref.seal();
        }
    }

    /// Check if a block is sealed (used by CFG construction)
    pub fn is_block_sealed(&self, block: BasicBlockId) -> bool {
        self.sealed_blocks.contains(&block)
    }

    /// Try to eliminate a trivial phi node (Algorithm 3, tryRemoveTrivialPhi)
    fn try_remove_trivial_phi(&mut self, phi: ValueId) -> ValueId {
        // Get phi operands
        let phi_sources = if let Some(phi_instr) = self.find_phi_instruction(phi) {
            if let InstructionKind::Phi { sources, .. } = &phi_instr.kind {
                sources.clone()
            } else {
                return phi; // Not a phi
            }
        } else {
            return phi; // Phi not found
        };

        // same ← None
        let mut same: Option<ValueId> = None;

        // for op in phi.operands:
        for (_block, value) in &phi_sources {
            if let Value::Operand(op) = value {
                // if op = same || op = phi: continue # Unique value or self−reference
                if Some(*op) == same || *op == phi {
                    continue;
                }
                // if same ≠ None: return phi # The phi merges at least two values: not trivial
                if same.is_some() {
                    return phi;
                }
                // if same = None: same ← op
                same = Some(*op);
            } else {
                // Non-operand values make phi non-trivial
                return phi;
            }
        }

        // if same = None: same ← new Undef() # The phi is unreachable or in the start block
        let same = same.unwrap_or_else(|| {
            // Create undefined value
            self.func.new_typed_value_id(self.get_variable_type_from_phi(phi))
        });

        // users ← phi.users.remove(phi) # Remember all users except the phi itself
        // phi.replaceBy(same) # Reroute all uses of phi to same and remove phi
        self.func.replace_all_uses(phi, same);
        self.remove_phi_instruction(phi);

        // Try to recursively remove all phi users, which might have become trivial
        // for use in users:
        //     if use is a Phi:
        //         tryRemoveTrivialPhi(use)
        // (Note: This recursive elimination is complex to implement safely with borrowing,
        //  so we'll implement a simpler version initially)

        same
    }

    /// Get the type of a variable (placeholder - needs integration with semantic analysis)
    fn get_variable_type(&self, _var: MirDefinitionId) -> MirType {
        // TODO: This needs to be implemented with actual type lookup
        // For now, assume felt type
        MirType::Felt
    }

    /// Get type from existing phi instruction
    fn get_variable_type_from_phi(&self, phi: ValueId) -> MirType {
        if let Some(phi_instr) = self.find_phi_instruction(phi) {
            if let InstructionKind::Phi { ty, .. } = &phi_instr.kind {
                return ty.clone();
            }
        }
        MirType::Felt // Fallback
    }

    /// Find which block contains a phi instruction
    fn find_phi_block(&self, phi_value: ValueId) -> Option<BasicBlockId> {
        for (block_id, block) in self.func.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                if let InstructionKind::Phi { dest, .. } = &instruction.kind {
                    if *dest == phi_value {
                        return Some(block_id);
                    }
                }
            }
        }
        None
    }

    /// Update phi instruction operands
    fn update_phi_operands(&mut self, phi_value: ValueId, sources: Vec<(BasicBlockId, Value)>) {
        // Find and update the phi instruction
        for (_block_id, block) in self.func.basic_blocks.iter_enumerated_mut() {
            for instruction in &mut block.instructions {
                if let InstructionKind::Phi { dest, sources: phi_sources, .. } = &mut instruction.kind {
                    if *dest == phi_value {
                        *phi_sources = sources;
                        return;
                    }
                }
            }
        }
    }

    /// Find a phi instruction by its destination value
    fn find_phi_instruction(&self, phi_value: ValueId) -> Option<&Instruction> {
        for (_block_id, block) in self.func.basic_blocks.iter_enumerated() {
            for instruction in &block.instructions {
                if let InstructionKind::Phi { dest, .. } = &instruction.kind {
                    if *dest == phi_value {
                        return Some(instruction);
                    }
                }
            }
        }
        None
    }

    /// Remove a phi instruction
    fn remove_phi_instruction(&mut self, phi_value: ValueId) {
        for (_block_id, block) in self.func.basic_blocks.iter_enumerated_mut() {
            block.instructions.retain(|instr| {
                if let InstructionKind::Phi { dest, .. } = &instr.kind {
                    *dest != phi_value
                } else {
                    true
                }
            });
        }
    }
}

/// Public API for integration with MirBuilder
impl<'f> SSABuilder<'f> {
    /// Check if a variable is defined in a block
    pub fn is_defined_in_block(&self, var: MirDefinitionId, block: BasicBlockId) -> bool {
        self.current_def.contains_key(&(var, block))
    }

    /// Get all variables defined in a block
    pub fn variables_in_block(&self, block: BasicBlockId) -> Vec<MirDefinitionId> {
        self.current_def
            .keys()
            .filter_map(|(var, blk)| if *blk == block { Some(*var) } else { None })
            .collect()
    }

    /// Mark a block as filled (all local statements processed)
    pub fn mark_block_filled(&mut self, block: BasicBlockId) {
        if let Some(block_ref) = self.func.basic_blocks.get_mut(block) {
            block_ref.mark_filled();
        }
    }
}
```

### 2. Add Module to lib.rs

```rust
// In mir/src/lib.rs, add:
pub mod ssa;
pub use ssa::SSABuilder;
```

## Legacy Code to Remove

AFTER this task completes:

- The global `definition_to_value` field in `MirState` (Task 7 will handle this)

## Dependencies

This task requires:

- Task 1 (BasicBlock edges and states)
- Task 2 (MirFunction edge helpers and replace_all_uses)

## Integration Notes

- The SSA builder will be integrated with `MirBuilder` in Task 7
- Variable type lookup needs semantic integration (placeholder for now)
- Error handling for undefined variables is basic (placeholder)

## Testing

- Unit tests for `write_variable`/`read_variable` cycles
- Test sealed vs unsealed block behavior
- Test trivial phi elimination cases
- Test multiple predecessors creating phi nodes
- Test recursive variable reading

## Success Criteria

- ✅ `SSABuilder` implements Braun algorithm exactly
- ✅ Per-block variable tracking with `currentDef` map
- ✅ Incomplete phi creation for unsealed blocks
- ✅ Phi completion upon sealing
- ✅ Trivial phi elimination works correctly
- ✅ All core SSA operations pass unit tests
