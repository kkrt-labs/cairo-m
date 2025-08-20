# Task 9: Cleanup and Documentation

## Goal

Remove all legacy code, add comprehensive documentation, and finalize the SSA
refactor implementation.

## Files to Modify

- All task-modified files - Add final documentation
- `mir/src/lib.rs` - Update module documentation
- `CLAUDE.md` - Update project documentation
- Various files - Remove any remaining legacy code

## Current State

SSA refactor is functionally complete but needs cleanup and documentation.

## Required Changes

### 1. Final Legacy Code Removal

Remove any remaining legacy code that wasn't cleaned up in previous tasks:

#### In `mir/src/cfg.rs`

```rust
// REMOVE: Old recomputation-based functions, replace with direct access
// OLD:
pub fn get_successors(function: &MirFunction, block_id: BasicBlockId) -> Vec<BasicBlockId> {
    if let Some(block) = function.basic_blocks.get(block_id) {
        block.terminator.target_blocks() // RECOMPUTATION
    } else {
        vec![]
    }
}

// NEW:
pub fn get_successors(function: &MirFunction, block_id: BasicBlockId) -> Vec<BasicBlockId> {
    if let Some(block) = function.basic_blocks.get(block_id) {
        block.succs.clone() // DIRECT ACCESS
    } else {
        vec![]
    }
}

// OLD:
pub fn get_predecessors(function: &MirFunction, target_id: BasicBlockId) -> Vec<BasicBlockId> {
    let mut predecessors = Vec::new();
    for (block_id, block) in function.basic_blocks.iter_enumerated() {
        if block.terminator.target_blocks().contains(&target_id) {
            predecessors.push(block_id);
        }
    }
    predecessors
}

// NEW:
pub fn get_predecessors(function: &MirFunction, target_id: BasicBlockId) -> Vec<BasicBlockId> {
    if let Some(block) = function.basic_blocks.get(target_id) {
        block.preds.clone() // DIRECT ACCESS
    } else {
        vec![]
    }
}
```

#### In `mir/src/builder/cfg_builder.rs`

```rust
// UPDATE: Method name for consistency
impl<'f> CfgBuilder<'f> {
    // RENAME: is_terminated -> has_terminator for consistency
    pub fn has_terminator(&self) -> bool {
        self.is_terminated || self.current_block().has_terminator()
    }

    // DEPRECATED: Keep old name for compatibility during transition
    #[deprecated(note = "Use has_terminator() instead")]
    pub fn is_terminated(&self) -> bool {
        self.has_terminator()
    }
}
```

### 2. Add Comprehensive Module Documentation

#### Update `mir/src/lib.rs`

````rust
//! # Cairo-M MIR (Middle-Level Intermediate Representation)
//!
//! This crate provides the MIR layer for the Cairo-M compiler, implementing
//! a Static Single Assignment (SSA) form intermediate representation.
//!
//! ## Architecture Overview
//!
//! The MIR consists of:
//! - **Functions**: Control flow graphs of basic blocks
//! - **Basic Blocks**: Sequences of instructions with explicit pred/succ edges
//! - **Instructions**: Three-address code operations
//! - **Values**: SSA values with unique definitions
//! - **Types**: MIR type system mirroring semantic types
//!
//! ## SSA Construction
//!
//! The MIR uses Braun et al.'s SSA construction algorithm:
//! - Per-block variable definitions (`currentDef[var][block]`)
//! - Sealed blocks (predecessor set finalized)
//! - Filled blocks (all local statements processed)
//! - Lazy phi insertion with trivial phi elimination
//!
//! ## Key Modules
//!
//! - [`ssa`]: SSA builder implementing Braun algorithm
//! - [`basic_block`]: Basic block structure with explicit edges
//! - [`function`]: Function-level MIR with edge maintenance
//! - [`instruction`]: MIR instruction types and phi support
//! - [`lowering`]: AST to MIR lowering with SSA construction
//! - [`passes`]: Optimization and analysis passes
//! - [`value_numbering`]: Local common subexpression elimination
//!
//! ## Usage
//!
//! ```rust
//! use cairo_m_compiler_mir::*;
//!
//! // Create a function
//! let mut function = MirFunction::new("example".to_string());
//!
//! // Build SSA form
//! let mut ssa = SSABuilder::new(&mut function);
//! ssa.write_variable(var_id, block_id, value_id);
//! let value = ssa.read_variable(var_id, block_id);
//! ssa.seal_block(block_id);
//! ```

// Re-export main types for easy access
pub use basic_block::BasicBlock;
pub use function::{MirFunction, MirDefinitionId};
pub use instruction::{Instruction, InstructionKind};
pub use ssa::SSABuilder;
pub use value_numbering::{PureKey, FunctionValueNumbering};
````

#### Add documentation to `mir/src/ssa.rs`

```rust
//! # SSA Builder - Braun et al. Algorithm Implementation
//!
//! This module implements the SSA construction algorithm from:
//! Braun, M., Buchwald, S., Hack, S., Leißa, R., Mallon, C., & Zwinkau, A. (2013).
//! "Simple and Efficient Construction of Static Single Assignment Form"
//!
//! ## Algorithm Overview
//!
//! The algorithm maintains:
//! - `currentDef[var][block]`: Current definition of variable in block
//! - Sealed blocks: Blocks whose predecessor set is final
//! - Filled blocks: Blocks whose local statements have been processed
//!
//! ## Key Operations
//!
//! - `writeVariable(var, block, value)`: Define variable in block
//! - `readVariable(var, block)`: Read variable, creating phi if needed
//! - `sealBlock(block)`: Finalize predecessor set, complete incomplete phis
//!
//! ## Phi Node Handling
//!
//! - Incomplete phis: Created for unsealed blocks, completed on sealing
//! - Trivial phi elimination: Removes phis where all operands are identical
//! - Phi placement: Always at block start, maintaining SSA form
//!
//! ## Integration
//!
//! The SSA builder integrates with MIR lowering to automatically construct
//! SSA form during AST-to-MIR translation, eliminating the need for separate
//! SSA conversion passes.
```

#### Add documentation to `mir/src/basic_block.rs`

```rust
//! # MIR Basic Block with Explicit CFG Edges
//!
//! Basic blocks are the fundamental units of control flow in MIR. Each block:
//! - Contains a sequence of instructions (phi nodes first)
//! - Has exactly one terminator
//! - Maintains explicit predecessor and successor lists
//! - Tracks SSA construction state (sealed/filled)
//!
//! ## SSA Construction States
//!
//! - **Sealed**: Predecessor set is finalized, phi operands can be completed
//! - **Filled**: All local statements processed, successors can be added
//!
//! ## Phi Instruction Ordering
//!
//! All phi instructions must appear at the start of the block before any
//! regular instructions. This invariant is maintained by `push_phi_front()`
//! and validated during function validation.
```

### 3. Update CLAUDE.md Documentation

Add section about SSA construction:

````markdown
## MIR and SSA Construction

Cairo-M's MIR uses Static Single Assignment (SSA) form constructed during
lowering using Braun et al.'s algorithm. Key features:

### SSA Construction

- Per-block variable tracking replaces global definition maps
- Lazy phi insertion with trivial phi elimination
- Sealed/filled block states for disciplined CFG construction

### Usage in Lowering

```rust
// Variable binding
builder.bind_variable("x", span, value)?;

// Variable reading (creates phis automatically)
let value = builder.read_variable_ssa("x", span)?;

// Block sealing (complete phi nodes)
builder.seal_block(merge_block);
```
````

### Debugging SSA

Set `DEBUG_MIR=1` to see SSA form in compiler output.

````

### 4. Add API Documentation Examples

#### In `mir/src/ssa.rs`
```rust
impl<'f> SSABuilder<'f> {
    /// Write a variable in a block (Algorithm 1, writeVariable)
    ///
    /// # Examples
    /// ```rust
    /// let mut function = MirFunction::new("test".to_string());
    /// let mut ssa = SSABuilder::new(&mut function);
    /// let value = function.new_typed_value_id(MirType::Felt);
    /// ssa.write_variable(var_id, block_id, value);
    /// ```
    pub fn write_variable(&mut self, var: MirDefinitionId, block: BasicBlockId, value: ValueId) {
        // ... implementation
    }

    /// Read a variable from a block, creating phi nodes as needed
    ///
    /// This implements the core SSA algorithm. For unsealed blocks,
    /// creates incomplete phi nodes. For sealed blocks with multiple
    /// predecessors, creates complete phi nodes with trivial elimination.
    ///
    /// # Examples
    /// ```rust
    /// let value = ssa.read_variable(var_id, block_id);
    /// // May create phi node if block has multiple predecessors
    /// ```
    pub fn read_variable(&mut self, var: MirDefinitionId, block: BasicBlockId) -> ValueId {
        // ... implementation
    }
}
````

### 5. Performance Documentation

Add performance notes:

```rust
//! ## Performance Characteristics
//!
//! - Variable reads: O(1) for local definitions, O(depth) for recursive reads
//! - Phi creation: O(predecessors) per phi node
//! - Trivial phi elimination: O(operands) per phi
//! - Block sealing: O(pending phis × predecessors)
//!
//! The algorithm is linear in the size of the CFG and avoids expensive
//! dominator tree computation.
```

### 6. Migration Guide

Create `docs/ssa_migration.md`:

````markdown
# SSA Migration Guide

This guide explains the changes from the previous global variable tracking to
the new SSA-based approach.

## What Changed

### Before (Global Tracking)

```rust
// Global map in MirState
definition_to_value: FxHashMap<MirDefinitionId, ValueId>

// Direct binding
state.definition_to_value.insert(var_id, value_id);

// Direct lookup
let value = state.definition_to_value.get(&var_id);
```
````

### After (SSA Tracking)

```rust
// Per-block tracking in SSABuilder
ssa.write_variable(var_id, block_id, value_id);

// SSA reads with phi creation
let value = ssa.read_variable(var_id, block_id);
```

## Key Benefits

1. **Automatic phi insertion**: No separate SSA conversion pass needed
2. **Better optimization**: SSA form enables more optimizations
3. **Cleaner semantics**: Per-block variable tracking is more precise
4. **Standard algorithm**: Uses well-established Braun et al. approach

## Migration Checklist

- [ ] Replace direct `definition_to_value` access with SSA methods
- [ ] Add block sealing at appropriate control flow points
- [ ] Update tests to expect phi nodes at merge points
- [ ] Verify validation passes with new SSA invariants

````

### 7. Code Quality Improvements

#### Add debug assertions
```rust
impl BasicBlock {
    pub fn add_pred(&mut self, pred: BasicBlockId) {
        debug_assert!(!self.sealed, "Cannot add predecessor to sealed block");
        if !self.preds.contains(&pred) {
            self.preds.push(pred);
        }
    }
}
````

#### Add helpful error messages

```rust
impl MirFunction {
    pub fn connect(&mut self, pred: BasicBlockId, succ: BasicBlockId) {
        let pred_block = self.basic_blocks.get_mut(pred)
            .unwrap_or_else(|| panic!("Cannot connect: predecessor block {:?} does not exist", pred));

        let succ_block = self.basic_blocks.get_mut(succ)
            .unwrap_or_else(|| panic!("Cannot connect: successor block {:?} does not exist", succ));

        pred_block.add_succ(succ);
        succ_block.add_pred(pred);
    }
}
```

## Legacy Code to Remove

AFTER this task completes:

1. **Remove deprecated methods**:
   - `CfgBuilder::is_terminated()` (replace with `has_terminator()`)

2. **Remove old CFG utilities**:
   - Recomputation-based `get_predecessors()`/`get_successors()` implementations

3. **Remove any remaining global variable tracking**:
   - Any lingering references to global `definition_to_value` patterns

4. **Remove temporary compatibility code**:
   - Any `#[deprecated]` items added during migration

## Testing

- Verify all documentation examples compile and work
- Run full test suite to ensure no regressions
- Test that performance is acceptable
- Verify debug output is helpful

## Success Criteria

- ✅ All legacy code removed
- ✅ Comprehensive documentation added
- ✅ API examples work correctly
- ✅ Migration guide is clear
- ✅ Performance is documented
- ✅ Debug output is helpful
- ✅ All tests pass
