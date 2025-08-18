# Fix MIR Pipeline Ordering Issue

## Problem Statement

The current MIR optimization pipeline has a critical ordering issue for CASM
backends. The `LowerAggregatesPass` runs AFTER `Mem2RegSsaPass`, but the CASM
backend requires all aggregate operations to be lowered to memory operations
before code generation. This creates a dependency conflict because:

1. `Mem2RegSsaPass` expects to operate on memory-based operations
   (load/store/framealloc/gep)
2. `LowerAggregatesPass` converts value-based aggregates TO memory operations
3. CASM backend cannot handle value-based aggregate instructions

The current pipeline fails because `Mem2RegSsaPass` cannot optimize aggregate
operations that haven't been lowered yet, and the CASM backend rejects any
remaining value-based aggregate instructions.

## Current Pipeline Analysis

### Current Pipeline Order (Standard)

From `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs`, lines
886-906:

```rust
pub fn standard_pipeline() -> Self {
    Self::new()
        // 1. Initial cleanup
        .add_pass(pre_opt::PreOptimizationPass::new())
        // 2. High-level aggregate optimizations (runs EARLY)
        .add_pass(const_fold::ConstFoldPass::new())
        // 3. (Conditional) Promote memory variables to SSA registers
        .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
        // 4. Validate SSA form before destruction
        .add_pass(Validation::new())
        // 5. Eliminate Phi nodes
        .add_pass(ssa_destruction::SsaDestructionPass::new())
        // 6. **PROBLEM**: Lower aggregates AFTER mem2reg
        .add_pass(lower_aggregates::LowerAggregatesPass::new())
        // 7. Final cleanup
        .add_pass(FuseCmpBranch::new())
        .add_pass(DeadCodeElimination::new())
        // 8. Validate final MIR
        .add_pass(Validation::new_post_ssa())
}
```

### Root Cause Analysis

1. **Dependency Violation**: `LowerAggregatesPass` creates new memory operations
   (framealloc, store, gep, load) that `Mem2RegSsaPass` could have optimized if
   it ran later.

2. **Backend Incompatibility**: The CASM backend validation in
   `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/codegen/src/generator.rs`
   explicitly rejects aggregate instructions:

   ```rust
   InstructionKind::MakeTuple { .. }
   | InstructionKind::ExtractTupleElement { .. }
   | InstructionKind::MakeStruct { .. }
   | InstructionKind::ExtractStructField { .. }
   | InstructionKind::InsertField { .. }
   | InstructionKind::InsertTuple { .. } => {
       return Err(CodegenError::InvalidMir(
           "Aggregate value operations should be lowered before code generation"
               .to_string(),
   ```

3. **Conditional Execution Issue**: `Mem2RegSsaPass` runs conditionally only
   when `function_uses_memory()` returns true, but value-based aggregates don't
   trigger this condition until after they're lowered.

4. **Optimization Loss**: Memory operations created by aggregate lowering miss
   optimization opportunities because mem2reg has already run.

## Proposed Solution: Backend-Aware Pipeline Configuration

### New Architecture

Introduce backend-specific pipeline configurations to handle different target
requirements:

```rust
/// Backend targets for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendTarget {
    /// CASM backend (requires memory-based operations)
    Casm,
    /// Future VM backend that might support value-based aggregates
    Vm,
    /// Generic/testing backend
    Generic,
}

/// Enhanced pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Backend target (determines lowering strategy)
    pub backend_target: BackendTarget,
    /// Enable debug output
    pub debug: bool,
}
```

### New Pipeline Architecture

#### CASM-Optimized Pipeline

```rust
pub fn casm_pipeline() -> Self {
    Self::new()
        // 1. Initial cleanup
        .add_pass(pre_opt::PreOptimizationPass::new())
        // 2. High-level aggregate optimizations (while still value-based)
        .add_pass(const_fold::ConstFoldPass::new())
        // 3. **EARLY LOWERING**: Convert aggregates to memory operations FIRST
        .add_pass(lower_aggregates::LowerAggregatesPass::new())
        // 4. **UNCONDITIONAL MEM2REG**: Now all functions use memory operations
        .add_pass(mem2reg_ssa::Mem2RegSsaPass::new())
        // 5. Validate SSA form
        .add_pass(Validation::new())
        // 6. Eliminate Phi nodes
        .add_pass(ssa_destruction::SsaDestructionPass::new())
        // 7. Final optimizations
        .add_pass(FuseCmpBranch::new())
        .add_pass(DeadCodeElimination::new())
        // 8. Validate final MIR (post-SSA)
        .add_pass(Validation::new_post_ssa())
}
```

#### Value-Based Pipeline (for future backends)

```rust
pub fn value_based_pipeline() -> Self {
    Self::new()
        // 1. Initial cleanup
        .add_pass(pre_opt::PreOptimizationPass::new())
        // 2. High-level aggregate optimizations
        .add_pass(const_fold::ConstFoldPass::new())
        // 3. (Conditional) Memory optimization only when needed
        .add_conditional_pass(mem2reg_ssa::Mem2RegSsaPass::new(), function_uses_memory)
        // 4. Validate SSA form
        .add_pass(Validation::new())
        // 5. Eliminate Phi nodes
        .add_pass(ssa_destruction::SsaDestructionPass::new())
        // 6. **LATE LOWERING**: Only lower aggregates if backend requires it
        // (This would be conditional based on backend support)
        // 7. Final optimizations
        .add_pass(FuseCmpBranch::new())
        .add_pass(DeadCodeElimination::new())
        // 8. Validate final MIR
        .add_pass(Validation::new_post_ssa())
}
```

## Implementation Plan

### Phase 1: Core Pipeline Infrastructure

#### 1.1. Extend PipelineConfig

**File**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/pipeline.rs`

```rust
impl PipelineConfig {
    /// Create CASM-optimized configuration
    pub const fn for_casm() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            backend_target: BackendTarget::Casm,
            debug: false,
        }
    }

    /// Create value-based configuration
    pub const fn for_vm() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            backend_target: BackendTarget::Vm,
            debug: false,
        }
    }

    /// Update from environment with backend override
    pub fn from_environment() -> Self {
        let mut config = Self::default();

        // Existing optimization level logic...

        // Backend target override
        if let Ok(val) = std::env::var("CAIRO_M_BACKEND") {
            config.backend_target = match val.as_str() {
                "casm" => BackendTarget::Casm,
                "vm" => BackendTarget::Vm,
                "generic" => BackendTarget::Generic,
                _ => BackendTarget::Casm, // Default to CASM
            };
        }

        config
    }
}
```

#### 1.2. Update PassManager

**File**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes.rs`

```rust
impl PassManager {
    /// Create backend-specific pipeline
    pub fn backend_pipeline(target: BackendTarget) -> Self {
        match target {
            BackendTarget::Casm => Self::casm_pipeline(),
            BackendTarget::Vm => Self::value_based_pipeline(),
            BackendTarget::Generic => Self::standard_pipeline(),
        }
    }

    /// CASM-optimized pipeline with early aggregate lowering
    pub fn casm_pipeline() -> Self {
        // Implementation as shown above
    }

    /// Value-based pipeline for future backends
    pub fn value_based_pipeline() -> Self {
        // Implementation as shown above
    }
}
```

#### 1.3. Update optimize_module Function

**File**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/pipeline.rs`

```rust
pub fn optimize_module(module: &mut MirModule, config: &PipelineConfig) {
    let mut pass_manager = match config.optimization_level {
        OptimizationLevel::None => return,
        OptimizationLevel::Basic => PassManager::basic_pipeline(),
        OptimizationLevel::Standard => PassManager::backend_pipeline(config.backend_target),
        OptimizationLevel::Aggressive => PassManager::backend_pipeline(config.backend_target),
    };

    // Rest of the function remains the same...
}
```

### Phase 2: Update Consumers

#### 2.1. Update Codegen Integration

**File**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/codegen/src/lib.rs`

Ensure the CASM pipeline is used by default when generating CASM code:

```rust
pub fn compile_to_casm(module: &mut MirModule) -> Result<Program, CodegenError> {
    // Use CASM-specific pipeline
    let config = PipelineConfig::for_casm();
    optimize_module(module, &config);

    // Generate CASM
    compile_module(module)
}
```

#### 2.2. Update CLI and Compiler Integration

Update the main compiler driver to use the correct backend configuration:

```rust
// In compiler driver
let config = PipelineConfig::from_environment()
    .with_backend(BackendTarget::Casm) // or based on output format
    .with_optimization_level(opt_level);
```

### Phase 3: Enhanced Validation

#### 3.1. Backend-Specific Validation

Add validation that ensures the correct lowering has occurred for each backend:

```rust
impl MirModule {
    /// Validate for specific backend target
    pub fn validate_for_backend(&self, target: BackendTarget) -> Result<(), ValidationError> {
        match target {
            BackendTarget::Casm => self.validate_for_casm(),
            BackendTarget::Vm => self.validate_for_vm(),
            BackendTarget::Generic => self.validate(),
        }
    }

    /// Validate that all aggregates are lowered for CASM
    fn validate_for_casm(&self) -> Result<(), ValidationError> {
        for (_, function) in self.functions() {
            for (_, block) in function.basic_blocks() {
                for instruction in &block.instructions {
                    if matches!(
                        instruction.kind,
                        InstructionKind::MakeTuple { .. }
                            | InstructionKind::ExtractTupleElement { .. }
                            | InstructionKind::MakeStruct { .. }
                            | InstructionKind::ExtractStructField { .. }
                            | InstructionKind::InsertField { .. }
                            | InstructionKind::InsertTuple { .. }
                    ) {
                        return Err(ValidationError::InvalidInstruction(format!(
                            "CASM backend requires aggregate lowering: {:?}",
                            instruction.kind
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}
```

## Migration Strategy

### Immediate Changes (Backward Compatible)

1. **Add new pipeline configurations** while keeping existing
   `standard_pipeline()` as default
2. **Update only CASM codegen path** to use the new CASM pipeline
3. **Keep existing optimization behavior** for other code paths

### Gradual Migration

1. **Phase 1**: Implement backend-aware pipelines with CASM as opt-in
2. **Phase 2**: Update tests to validate both pipeline behaviors
3. **Phase 3**: Make CASM pipeline the default for code generation
4. **Phase 4**: Remove or deprecate the old standard pipeline

### Testing Strategy

1. **Dual Testing**: Run existing tests with both old and new pipelines to
   ensure equivalence where expected
2. **CASM-Specific Tests**: Add tests that specifically validate aggregate
   lowering occurs before mem2reg
3. **Performance Benchmarks**: Measure optimization effectiveness of the new
   pipeline ordering
4. **Snapshot Testing**: Update MIR snapshots to reflect new pipeline output

## Configuration Approach

### Environment Variables

```bash
# Backend target
export CAIRO_M_BACKEND=casm  # or vm, generic

# Optimization level (existing)
export CAIRO_M_OPT_LEVEL=2

# Debug output (existing)
export CAIRO_M_DEBUG=1
```

### Programmatic Configuration

```rust
// For CASM compilation
let config = PipelineConfig::for_casm()
    .with_optimization_level(OptimizationLevel::Aggressive)
    .with_debug(true);

// For VM compilation (future)
let config = PipelineConfig::for_vm()
    .with_optimization_level(OptimizationLevel::Standard);
```

### CLI Integration

```bash
# Existing behavior (auto-detect from output)
cairo-m-compiler -i input.cm -o output.casm

# Explicit backend specification
cairo-m-compiler -i input.cm -o output.casm --backend casm
cairo-m-compiler -i input.cm -o output.vm --backend vm
```

## Benefits of This Approach

1. **Correctness**: Fixes the fundamental ordering issue for CASM backend
2. **Performance**: Allows mem2reg to optimize memory operations created by
   aggregate lowering
3. **Flexibility**: Supports different backend requirements without compromising
   optimization
4. **Future-Proof**: Enables value-based backends while maintaining CASM
   compatibility
5. **Backward Compatibility**: Existing code continues to work during migration
6. **Clear Separation**: Each backend gets its optimal pipeline configuration

## Implementation Checklist

- [ ] Define `BackendTarget` enum
- [ ] Extend `PipelineConfig` with backend awareness
- [ ] Implement `casm_pipeline()` with early aggregate lowering
- [ ] Implement `value_based_pipeline()` for future use
- [ ] Update `PassManager::backend_pipeline()` dispatch
- [ ] Update `optimize_module()` to use backend-specific pipelines
- [ ] Add backend-specific validation methods
- [ ] Update CASM codegen integration
- [ ] Add environment variable support for backend selection
- [ ] Update CLI to support backend specification
- [ ] Write comprehensive tests for both pipeline types
- [ ] Update documentation and examples
- [ ] Benchmark optimization effectiveness
- [ ] Plan migration timeline for existing codebase
