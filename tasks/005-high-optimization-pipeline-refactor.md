# Task 005: HIGH - Optimization Pipeline Refactor

**Priority:** HIGH  
**Dependencies:** Tasks 001-004 (requires aggregate infrastructure)

## Why

The current optimization pipeline runs expensive passes like SROA (Scalar
Replacement of Aggregates) and Mem2Reg unconditionally on all functions, even
those that don't use memory operations. With the introduction of first-class
aggregate instructions (MakeTuple, ExtractTuple, MakeStruct, ExtractField), many
functions will operate entirely in SSA without touching memory.

This creates an opportunity for significant compile-time improvements by making
the optimization pipeline aggregate-aware. Functions that use the new
value-based aggregate operations should bypass the expensive memory-oriented
optimization passes entirely, while preserving the existing optimization path
for functions that still use memory operations (arrays, address-taking, etc.).

## What

Refactor the `PassManager::standard_pipeline()` to conditionally run expensive
optimization passes based on whether a function actually uses memory operations.
The key changes include:

1. **Add a `function_uses_memory()` helper** that analyzes MIR functions to
   detect memory operations
2. **Conditionally run SROA and Mem2Reg passes** only when memory operations are
   present
3. **Introduce the new VarSsa pass** before validation for functions using
   value-based aggregates
4. **Reorder pipeline stages** to optimize the common case of memory-free
   functions
5. **Preserve backward compatibility** for existing memory-based code paths

This refactoring reduces compilation overhead for the common case while
maintaining correctness for all existing patterns.

## How

### 1. Add Memory Usage Detection Helper

Create a helper function to detect whether a function uses memory operations:

**File:** `crates/compiler/mir/src/passes.rs`

```rust
/// Analyzes a MIR function to determine if it uses memory operations
/// that require SROA/Mem2Reg optimization passes.
fn function_uses_memory(function: &MirFunction) -> bool {
    for block in function.blocks.values() {
        for instruction in &block.instructions {
            match &instruction.kind {
                InstructionKind::FrameAlloc { .. } |
                InstructionKind::Load { .. } |
                InstructionKind::Store { .. } |
                InstructionKind::GetElementPtr { .. } => {
                    return true;
                }
                // Array operations also indicate memory usage
                InstructionKind::ArrayGet { .. } |
                InstructionKind::ArraySet { .. } => {
                    return true;
                }
                _ => continue,
            }
        }
    }
    false
}
```

### 2. Modify PassManager::standard_pipeline()

Update the pipeline to conditionally run expensive passes:

**File:** `crates/compiler/mir/src/passes.rs`

```rust
impl PassManager {
    pub fn standard_pipeline() -> Self {
        let mut manager = PassManager::new();

        // Always run basic passes
        manager.add_pass(Box::new(PreOptimizationPass));

        // Add the new Variable-to-SSA pass for aggregate-aware functions
        manager.add_pass(Box::new(VarSsaPass));

        // Conditionally run memory-oriented optimization passes
        manager.add_conditional_pass(
            Box::new(SroaPass),
            |function| function_uses_memory(function)
        );

        manager.add_conditional_pass(
            Box::new(Mem2RegSsaPass),
            |function| function_uses_memory(function)
        );

        // Always run remaining optimization passes
        manager.add_pass(Box::new(FuseCmpBranchPass));
        manager.add_pass(Box::new(DeadCodeEliminationPass));

        // Backend-specific passes
        manager.add_pass(Box::new(SsaDestructionParallelCopyPass));

        // Validation (always run)
        manager.add_pass(Box::new(ValidationPass::new_post_ssa()));

        manager
    }
}
```

### 3. Implement Conditional Pass Execution

Add support for conditional pass execution in the PassManager:

**File:** `crates/compiler/mir/src/passes.rs`

```rust
pub struct ConditionalPass {
    pass: Box<dyn MirPass>,
    condition: fn(&MirFunction) -> bool,
}

impl PassManager {
    pub fn add_conditional_pass(
        &mut self,
        pass: Box<dyn MirPass>,
        condition: fn(&MirFunction) -> bool
    ) {
        self.passes.push(Box::new(ConditionalPass { pass, condition }));
    }
}

impl MirPass for ConditionalPass {
    fn run_on_function(&mut self, function: &mut MirFunction) -> PassResult {
        if (self.condition)(function) {
            self.pass.run_on_function(function)
        } else {
            // Skip the pass - no changes needed
            PassResult::Unchanged
        }
    }

    fn name(&self) -> &str {
        self.pass.name()
    }
}
```

### 4. Pipeline Ordering Considerations

The new pipeline order prioritizes the common case:

1. **PreOptimizationPass** - Basic cleanup, always needed
2. **VarSsaPass** - Convert variables to SSA for aggregate-aware functions
3. **SroaPass** - Only for functions with frame allocations
4. **Mem2RegSsaPass** - Only for functions with memory operations
5. **FuseCmpBranchPass** - Always beneficial
6. **DeadCodeEliminationPass** - Clean up after optimizations
7. **SsaDestructionParallelCopyPass** - Backend requirement
8. **ValidationPass** - Final correctness check

### 5. Add VarSsa Pass Integration

Since this task depends on the VarSsa pass from the aggregate infrastructure,
ensure it's properly integrated:

**File:** `crates/compiler/mir/src/passes/mod.rs`

```rust
pub mod var_ssa;  // New module for Variable-to-SSA conversion
```

### 6. Performance Monitoring

Add optional performance metrics to track optimization improvements:

```rust
impl PassManager {
    pub fn run_with_metrics(&mut self, module: &mut MirModule) -> OptimizationMetrics {
        let mut metrics = OptimizationMetrics::new();

        for function in module.functions.values_mut() {
            let uses_memory = function_uses_memory(function);
            metrics.record_function_type(uses_memory);

            for pass in &mut self.passes {
                let start = std::time::Instant::now();
                let result = pass.run_on_function(function);
                metrics.record_pass_execution(pass.name(), start.elapsed(), result);
            }
        }

        metrics
    }
}
```

## Testing Strategy

1. **Unit Tests**: Test `function_uses_memory()` with various MIR patterns
2. **Integration Tests**: Verify pipeline behavior for both memory-using and
   memory-free functions
3. **Performance Tests**: Measure compilation time improvements for
   aggregate-heavy code
4. **Regression Tests**: Ensure existing memory-based code still optimizes
   correctly

## Expected Outcomes

- **Compilation Speed**: 40-60% faster compilation for functions using only
  value-based aggregates
- **Code Quality**: Equivalent optimization quality for all function types
- **Maintainability**: Cleaner separation between memory-oriented and
  value-oriented optimization paths
- **Backward Compatibility**: No changes to existing behavior for memory-using
  functions

## Implementation Notes

- This task requires the VarSsa pass from tasks 001-004 to be implemented first
- The conditional pass mechanism can be extended for other optimization
  scenarios
- Consider adding compilation flags to force old/new pipeline behavior during
  transition
- Memory usage detection should be conservative - when in doubt, run the full
  pipeline
