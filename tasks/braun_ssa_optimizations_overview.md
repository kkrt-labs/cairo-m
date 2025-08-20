# Braun-Style Local SSA Optimizations Implementation Plan

## Overview

This document outlines the implementation of Braun-style local SSA optimizations
as MIR passes for the Cairo-M compiler. These optimizations are conservative,
local optimizations that reduce IR size, simplify control flow, and improve
downstream analyses with minimal analysis cost.

## Background

The Cairo-M compiler already implements SSA construction using the Braun et al.
algorithm. The current MIR implementation includes:

- **SSA Construction**: Located in `mir/src/function.rs:75-86` with state
  tracking
- **Value Replacement**: `MirFunction::replace_all_uses()` at line 386
- **Terminator Management**: `target_blocks()` and edge management in
  `terminator.rs`
- **Existing Passes**: `FuseCmpBranch` and `DeadCodeElimination` in `passes/`

## Key MIR API Reference

### Core Structures

```rust
// From mir/src/instruction.rs
pub enum InstructionKind {
    Assign { dest: ValueId, source: Value, ty: MirType },
    BinaryOp { op: BinaryOp, dest: ValueId, left: Value, right: Value },
    UnaryOp { op: UnaryOp, dest: ValueId, source: Value },
    // ... aggregate operations (MakeTuple, ExtractTuple, etc.)
}

impl Instruction {
    pub fn has_side_effects(&self) -> bool;
    pub fn is_pure(&self) -> bool;
    pub fn replace_value_uses(&mut self, from: ValueId, to: ValueId);
    pub fn destinations(&self) -> Vec<ValueId>;
}
```

### Function and Value Management

```rust
// From mir/src/function.rs
impl MirFunction {
    pub fn replace_all_uses(&mut self, from: ValueId, to: ValueId);
    pub fn new_typed_value_id(&mut self, ty: MirType) -> ValueId;
    pub fn get_value_type(&self, id: ValueId) -> Option<&MirType>;
}
```

### Terminator and CFG Management

```rust
// From mir/src/terminator.rs and basic_block.rs
impl Terminator {
    pub fn target_blocks(&self) -> Vec<BasicBlockId>;
    pub fn replace_value_uses(&mut self, from: ValueId, to: ValueId);
}

impl BasicBlock {
    pub fn set_terminator(&mut self, terminator: Terminator);
}
```

### Pass Infrastructure

```rust
// From mir/src/passes.rs
pub trait MirPass {
    fn run(&mut self, function: &mut MirFunction) -> bool;
    fn name(&self) -> &'static str;
}

impl PassManager {
    pub fn add_pass<P: MirPass + 'static>(self, pass: P) -> Self;
    pub fn run(&mut self, function: &mut MirFunction) -> bool;
}
```

## Planned Optimization Passes

1. **ArithmeticSimplify** - Algebraic simplifications (`x + 0 → x`, etc.)
2. **ConstantFolding** - Evaluate operations with all literal operands
3. **CopyPropagation** - Remove redundant assignments in SSA
4. **LocalCSE** - Per-block common subexpression elimination
5. **SimplifyBranches** - Constant condition branch elimination
6. **SimplifyPhi** - Trivial phi node elimination

## Integration Points

### Pipeline Integration

The passes will be integrated into `PassManager` pipelines in `pipeline.rs`:

- **Basic**:
  `ArithmeticSimplify → ConstantFolding → CopyPropagation → SimplifyBranches → FuseCmpBranch → DeadCodeElimination`
- **Standard**: Add `LocalCSE` before `SimplifyBranches`
- **Aggressive**: Run to fixed point with iteration until no modifications

### Utility Functions

A new utility function will be added to `MirFunction`:

```rust
impl MirFunction {
    pub fn set_terminator_with_edges(&mut self, block_id: BasicBlockId, new_term: Terminator) {
        let old_targets = self.basic_blocks[block_id].terminator.target_blocks();
        for t in old_targets { self.disconnect(block_id, t); }
        self.basic_blocks[block_id].set_terminator(new_term.clone());
        for t in new_term.target_blocks() { self.connect(block_id, t); }
    }
}
```

## Implementation Strategy

1. Each pass implements the `MirPass` trait independently
2. Passes operate only on SSA-form IR (post-construction)
3. Conservative approach: skip optimizations when unsure
4. Maintain CFG invariants using existing edge management
5. Comprehensive testing with snapshot tests following existing patterns

## Testing Strategy

- Unit tests for each pass in `passes/` directory
- Integration tests in `pipeline_tests.rs`
- Snapshot testing using `insta` crate
- Validation using existing `Validation` pass

## Files to Create/Modify

### New Files

- `passes/arithmetic_simplify.rs`
- `passes/constant_folding.rs`
- `passes/copy_propagation.rs`
- `passes/local_cse.rs`
- `passes/simplify_branches.rs`
- `passes/simplify_phi.rs`

### Modified Files

- `passes/mod.rs` - Export new passes
- `passes.rs` - Add utility function
- `pipeline.rs` - Update pipeline configurations
- `function.rs` - Add `set_terminator_with_edges` utility

## Success Criteria

- All passes integrate cleanly with existing `PassManager`
- `cargo test` passes including new tests
- `module.validate()` succeeds before and after optimization
- IR size reduction on synthetic test cases
- No performance regressions on compilation pipeline

## Next Steps

The following detailed implementation plans are available:

1. `arithmetic_simplify_implementation.md`
2. `constant_folding_implementation.md`
3. `copy_propagation_implementation.md`
4. `local_cse_implementation.md`
5. `simplify_branches_implementation.md`
6. `simplify_phi_implementation.md`
