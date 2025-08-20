# Braun-Style SSA Optimizations Implementation Checklist

## Overview

This checklist provides a step-by-step implementation guide for all six
optimization passes, with specific file locations, API usage, and integration
points.

## Phase 1: Foundation and Utilities

### 1.1 Add Utility Function to MirFunction

**File**: `crates/compiler/mir/src/function.rs`

**Add after line 426** (after `replace_all_uses`):

```rust
/// Set terminator while properly maintaining CFG edges
/// This is a helper for optimization passes that need to change control flow
pub fn set_terminator_with_edges(&mut self, block_id: BasicBlockId, new_term: Terminator) {
    // Get old target blocks
    let old_targets = if let Some(block) = self.basic_blocks.get(block_id) {
        block.terminator.target_blocks()
    } else {
        return; // Block doesn't exist
    };

    // Disconnect from old targets
    for target in old_targets {
        self.disconnect(block_id, target);
    }

    // Set new terminator
    if let Some(block) = self.basic_blocks.get_mut(block_id) {
        block.set_terminator(new_term.clone());
    }

    // Connect to new targets
    for target in new_term.target_blocks() {
        self.connect(block_id, target);
    }
}
```

**Validation**: Ensure `disconnect` and `connect` methods exist (lines 369-382)

## Phase 2: Core Optimization Passes

### 2.1 ArithmeticSimplify Pass

**Create**: `crates/compiler/mir/src/passes/arithmetic_simplify.rs`

**Key Implementation Points**:

- Pattern match on `InstructionKind::BinaryOp` and `InstructionKind::UnaryOp`
- Use `op.result_type()` for type consistency
- Rewrite instructions to `InstructionKind::Assign` in place
- Handle identity operations: `x + 0`, `x * 1`, `x - 0`
- Handle absorbing operations: `x * 0`
- Handle self-comparison: `x == x`, `x != x`
- Handle double negation: `!(!x) → x`

**Testing**: Create test in `passes/` directory with snapshot tests

### 2.2 ConstantFolding Pass

**Create**: `crates/compiler/mir/src/passes/constant_folding.rs`

**Key Implementation Points**:

- Pattern match on `Value::Literal` operands only
- Use safe arithmetic (saturating for felt, wrapping for U32)
- Never fold division by zero
- Handle both `BinaryOp` and `UnaryOp` variants
- Preserve operation result types using `op.result_type()`
- Convert to `InstructionKind::Assign` with computed literal

**Testing**: Test arithmetic, comparison, and boolean operations

### 2.3 CopyPropagation Pass

**Create**: `crates/compiler/mir/src/passes/copy_propagation.rs`

**Key Implementation Points**:

- Identify `InstructionKind::Assign` with `Value::Operand` source
- Use `function.replace_all_uses(dest, source)` for replacement
- Remove assignment instructions after replacement
- Verify type consistency before propagation
- Handle instruction removal in reverse order

**Testing**: Test basic copies, chained copies, and cross-block scenarios

### 2.4 LocalCSE Pass

**Create**: `crates/compiler/mir/src/passes/local_cse.rs`

**Key Implementation Points**:

- Create `PureExpressionKey` enum for instruction hashing
- Use `instr.is_pure()` to filter candidates
- Per-block hash map: `FxHashMap<PureExpressionKey, ValueId>`
- Skip `Load` instructions (conservative approach)
- Only handle all-operand expressions (skip mixed literal/operand)
- Use `function.replace_all_uses()` for CSE replacement
- Remove redundant instructions using use counts

**Testing**: Test common subexpressions within blocks, verify block locality

### 2.5 SimplifyBranches Pass

**Create**: `crates/compiler/mir/src/passes/simplify_branches.rs`

**Key Implementation Points**:

- Pattern match on `Terminator::If` and `Terminator::BranchCmp`
- Evaluate constant conditions and comparisons
- Use `function.set_terminator_with_edges()` for CFG consistency
- Handle boolean, integer, and U32 comparisons
- Convert to `Terminator::jump()` when condition is constant

**Testing**: Test constant boolean/integer conditions, comparison folding

### 2.6 SimplifyPhi Pass

**Create**: `crates/compiler/mir/src/passes/simplify_phi.rs`

**Key Implementation Points**:

- Pattern match on `InstructionKind::Phi`
- Analyze phi operands for trivial patterns
- Handle self-references vs. unique operands
- Use `function.replace_all_uses()` for phi elimination
- Remove phi instructions after replacement
- Conservative approach for degenerate cases

**Testing**: Test identical operands, self-references, mixed cases

## Phase 3: Integration

### 3.1 Update Module Exports

**File**: `crates/compiler/mir/src/passes/mod.rs`

**Add**:

```rust
pub mod arithmetic_simplify;
pub mod constant_folding;
pub mod copy_propagation;
pub mod local_cse;
pub mod simplify_branches;
pub mod simplify_phi;

pub use arithmetic_simplify::ArithmeticSimplify;
pub use constant_folding::ConstantFolding;
pub use copy_propagation::CopyPropagation;
pub use local_cse::LocalCSE;
pub use simplify_branches::SimplifyBranches;
pub use simplify_phi::SimplifyPhi;
```

### 3.2 Update Main Passes Module

**File**: `crates/compiler/mir/src/passes.rs`

**Update imports around line 61**:

```rust
pub use passes::{
    arithmetic_simplify::ArithmeticSimplify,
    constant_folding::ConstantFolding,
    copy_propagation::CopyPropagation,
    dead_code_elimination::DeadCodeElimination,
    fuse_cmp::FuseCmpBranch,
    local_cse::LocalCSE,
    simplify_branches::SimplifyBranches,
    simplify_phi::SimplifyPhi,
    MirPass, PassManager, Validation,
};
```

### 3.3 Update Pipeline Configurations

**File**: `crates/compiler/mir/src/pipeline.rs`

**Update `PassManager` implementations around lines 670-695**:

```rust
impl PassManager {
    /// Create a basic optimization pipeline (minimal optimizations)
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(SimplifyBranches::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa())
    }

    /// Create a standard optimization pipeline (default)
    pub fn standard_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(LocalCSE::new())
            .add_pass(SimplifyBranches::new())
            .add_pass(SimplifyPhi::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa())
    }

    /// Create an aggressive optimization pipeline
    pub fn aggressive_pipeline() -> Self {
        // Run to fixpoint for maximum optimization
        let mut pm = Self::standard_pipeline();
        pm.add_pass(SimplifyPhi::new()); // Second phi pass for cascading elimination
        pm
    }
}
```

## Phase 4: Testing and Validation

### 4.1 Unit Tests

**Location**: Each pass directory (`passes/arithmetic_simplify/`, etc.)

**Test Structure**:

- Create small MIR functions using `testing.rs` helpers
- Apply pass and check modifications
- Use snapshot testing with `insta`
- Test both positive and negative cases

### 4.2 Integration Tests

**File**: `crates/compiler/mir/src/pipeline_tests.rs`

**Add tests**:

- Test complete pipeline on synthetic functions
- Verify pass ordering effects
- Test fixed-point iteration for aggressive pipeline
- Ensure validation passes after optimization

### 4.3 Performance Tests

**Considerations**:

- Measure compilation time impact
- Test on functions with varying complexity
- Verify IR size reduction
- Check for optimization effectiveness

## Phase 5: Documentation and Debugging

### 5.1 Debug Environment Variable

**File**: `crates/compiler/mir/src/passes.rs`

**Add debug output in `PassManager::run()` around line 658**:

```rust
pub fn run(&mut self, function: &mut MirFunction) -> bool {
    let mut modified = false;

    for pass in &mut self.passes {
        let pass_modified = pass.run(function);
        if pass_modified {
            modified = true;
            if std::env::var("CAIRO_M_DEBUG").is_ok() {
                eprintln!(
                    "Pass '{}' modified function '{}' - {} instructions, {} blocks",
                    pass.name(),
                    function.name,
                    function.instruction_count(),
                    function.basic_blocks.len()
                );
            }
        }
    }

    modified
}
```

### 5.2 Fixed-Point Iteration Utility

**File**: `crates/compiler/mir/src/passes.rs`

**Add method to `PassManager`**:

```rust
impl PassManager {
    /// Run passes to a fixed point (for aggressive optimization)
    pub fn run_to_fixpoint(&mut self, function: &mut MirFunction) -> bool {
        let mut overall_modified = false;
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

        loop {
            let modified = self.run(function);
            if !modified || iteration >= MAX_ITERATIONS {
                break;
            }
            overall_modified = true;
            iteration += 1;

            if std::env::var("CAIRO_M_DEBUG").is_ok() {
                eprintln!("Fixed-point iteration {}: modified", iteration);
            }
        }

        if std::env::var("CAIRO_M_DEBUG").is_ok() && iteration > 0 {
            eprintln!("Fixed-point converged after {} iterations", iteration);
        }

        overall_modified
    }
}
```

## Implementation Order

### Recommended Implementation Sequence

1. **Phase 1**: Add utility function to `MirFunction`
2. **ArithmeticSimplify**: Start with simplest optimizations
3. **ConstantFolding**: Build on arithmetic simplification
4. **CopyPropagation**: Leverage existing `replace_all_uses`
5. **SimplifyBranches**: Use new terminator utility
6. **LocalCSE**: More complex with hashing and analysis
7. **SimplifyPhi**: Most complex, benefits from other passes
8. **Phase 3**: Integration and pipeline updates
9. **Phase 4**: Comprehensive testing
10. **Phase 5**: Debug features and documentation

### Validation at Each Step

- Run `cargo test` after each pass implementation
- Use `cargo test -p cairo-m-compiler-mir` for focused testing
- Run existing MIR tests to ensure no regressions
- Test pipeline integration incrementally

## Success Metrics

- [ ] All passes implement `MirPass` trait correctly
- [ ] Pipeline configurations build and run
- [ ] `cargo test` passes with new tests
- [ ] Validation passes succeed on optimized MIR
- [ ] IR size reduction measurable on test cases
- [ ] No performance regressions in compilation pipeline
- [ ] Integration with existing passes (FuseCmpBranch, DeadCodeElimination)

## Debugging Tips

### Environment Variables

```bash
# Enable pass debug output
export CAIRO_M_DEBUG=1

# Run with verbose MIR output
cargo run --bin cairo-m-compiler -- -i test.cm -v

# Run specific pass tests
cargo test -p cairo-m-compiler-mir arithmetic_simplify
```

### Common Issues

1. **Borrow Checker**: Use two-phase approach (collect, then modify)
2. **Index Invalidation**: Remove instructions in reverse order
3. **CFG Consistency**: Always use `set_terminator_with_edges`
4. **Type Mismatches**: Verify types before optimizations
5. **Infinite Loops**: Limit fixed-point iterations

### Debugging Functions

Add to each pass for detailed logging:

```rust
fn debug_log(&self, message: &str) {
    if std::env::var("CAIRO_M_DEBUG").is_ok() {
        eprintln!("[{}] {}", self.name(), message);
    }
}
```
