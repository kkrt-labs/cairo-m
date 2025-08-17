# Task 006: Refactor Optimization Pipeline for Aggregate-First Design [MEDIUM PRIORITY]

## Priority: MEDIUM - Performance and maintainability improvement

## Summary

Refactor the optimization pipeline to fully leverage the aggregate-first design.
Remove or update passes that assume memory-based aggregates, and enhance passes
that can benefit from value-based operations.

## Current State

- ✅ Conditional pass execution implemented
- ⚠️ `mem2reg_ssa` still needed for hybrid model
- ⚠️ SROA disabled but not removed
- ⚠️ Some passes not aggregate-aware

## Affected Code

### Primary File: `crates/compiler/mir/src/passes.rs`

### Current Pipeline Structure

```rust
pub fn standard_pipeline() -> PassManager {
    PassManager::new()
        .add_pass(pre_opt::PreOptPass::new())
        // SROA disabled
        .add_pass(inline::InlinePass::new())
        .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
        .add_pass(const_fold::ConstFoldPass::new())
        .add_pass(dce::DeadCodeElimination::new())
        .add_pass(Validation)
}
```

## Implementation Plan

### Phase 1: Remove Obsolete Passes

#### Remove SROA (see Task 004)

- Delete `passes/sroa.rs`
- Remove from pipeline and exports

#### Evaluate mem2reg_ssa Necessity

After Tasks 001-003 are complete:

- Most functions won't need mem2reg
- Keep for array-heavy functions only
- Consider renaming to `array_mem2reg`

### Phase 2: Enhance Aggregate-Aware Passes

#### Update Constant Folding (`const_fold.rs`)

Current: Already handles aggregate operations ✅ Enhancement opportunities:

```rust
// Add more patterns
impl ConstFoldPass {
    fn fold_nested_aggregates(&mut self) {
        // ExtractField(InsertField(base, field, value), field) -> value
        // MakeTuple(ExtractElement(t, 0), ExtractElement(t, 1)) -> t
    }
}
```

#### Update Dead Code Elimination (`dce.rs`)

Add aggregate-specific DCE:

```rust
// Remove unused aggregate creations
if let InstructionKind::MakeTuple { .. } = inst.kind {
    if !self.is_used(inst_id) {
        self.mark_for_removal(inst_id);
    }
}
```

### Phase 3: Create New Aggregate-Specific Passes

#### Add Aggregate Simplification Pass

Create `passes/aggregate_simplify.rs`:

```rust
pub struct AggregateSimplifyPass;

impl Pass for AggregateSimplifyPass {
    fn run_on_function(&mut self, func: &mut Function) -> bool {
        // Simplify patterns like:
        // - Consecutive Insert/Extract on same field
        // - Redundant Make/Extract sequences
        // - Identity transformations
    }
}
```

#### Add Tuple Flattening Pass

For functions that don't escape tuples:

```rust
pub struct TupleFlatteningPass;
// Converts tuple parameters/returns to individual values
// Beneficial for small tuples (2-3 elements)
```

### Phase 4: Pipeline Configuration

#### Update Pipeline Builder

```rust
pub struct PipelineConfig {
    pub enable_aggressive_opts: bool,
    pub target_memory_model: MemoryModel,
    pub aggregate_strategy: AggregateStrategy,
}

impl PassManager {
    pub fn configured_pipeline(config: PipelineConfig) -> Self {
        let mut pipeline = PassManager::new();

        // Always run
        pipeline.add_pass(pre_opt::PreOptPass::new());

        // Conditional based on strategy
        match config.aggregate_strategy {
            AggregateStrategy::ValueBased => {
                pipeline.add_pass(aggregate_simplify::AggregateSimplifyPass::new());
                pipeline.add_pass(tuple_flattening::TupleFlatteningPass::new());
            }
            AggregateStrategy::MemoryBased => {
                pipeline.add_pass(mem2reg_ssa::Mem2RegSsaPass::new());
            }
        }

        // Always run
        pipeline.add_pass(const_fold::ConstFoldPass::new());
        pipeline.add_pass(dce::DeadCodeElimination::new());
        pipeline.add_pass(Validation);

        pipeline
    }
}
```

### Phase 5: Update Helper Functions

#### Enhance `function_uses_memory`

```rust
pub fn function_uses_memory(func: &Function) -> bool {
    func.instructions().any(|inst| {
        matches!(
            inst.kind,
            InstructionKind::Alloc { .. } |
            InstructionKind::Load { .. } |
            InstructionKind::Store { .. } |
            // Don't count aggregate ops as memory use
            // InstructionKind::MakeTuple { .. } |  // <- Not memory
            // InstructionKind::ExtractField { .. } | // <- Not memory
        )
    })
}
```

## Testing Requirements

### Performance Benchmarks

```rust
#[bench]
fn bench_aggregate_pipeline_vs_memory_pipeline() {
    // Compare compilation time and resulting code quality
}
```

### Correctness Tests

```rust
#[test]
fn test_pipeline_preserves_semantics() {
    // Verify optimized code behaves identically
}

#[test]
fn test_conditional_pass_execution() {
    // Verify passes run only when needed
}
```

## Metrics to Track

1. **Compilation Speed**
   - Time per pass
   - Total pipeline time
   - Memory usage

2. **Code Quality**
   - Instruction count reduction
   - Memory operation reduction
   - Register pressure

3. **Pass Effectiveness**
   - How often each pass makes changes
   - Which passes provide most benefit

## Rollout Strategy

1. **Phase 1**: Remove obsolete passes (after Tasks 001-005)
2. **Phase 2**: Add new aggregate passes
3. **Phase 3**: Performance testing and tuning
4. **Phase 4**: Make default for new code
5. **Phase 5**: Migrate existing code

## Success Criteria

1. Pipeline correctly handles both value and memory aggregates
2. Measurable performance improvement (>10% faster compilation)
3. Reduced memory operations in output (>30% reduction)
4. No regression in code quality
5. Clear configuration options for different scenarios
