# Pipeline Configuration Simplification

## Summary

The MIR optimization pipeline configuration has been dramatically simplified
from a complex, multi-layered system to a straightforward configuration with
sensible defaults.

## What Changed

### Before (Overly Complex)

- Multiple configuration structs: `AggregateConfig`, `OptimizationConfig`,
  `DebugConfig`, `BackendConfig`
- Pluggable backend system despite only targeting CASM
- 10+ environment variables for fine-grained control
- A/B testing framework for comparing configurations
- Complex feature flags for every minor detail

### After (Simple and Clean)

```rust
pub struct PipelineConfig {
    pub optimization_level: OptimizationLevel,  // None, Basic, Standard, Aggressive
    pub debug: bool,                           // Enable debug output
    pub lower_aggregates_to_memory: bool,      // Only if needed for compatibility
}
```

## Key Improvements

1. **Single Optimization Level**: Instead of dozens of flags, use a simple
   optimization level (0-3) like most compilers
2. **Sensible Defaults**: `OptimizationLevel::Standard` by default - the best
   configuration for most cases
3. **No Backend Abstraction**: Removed unnecessary backend trait system since we
   only target CASM
4. **Minimal Environment Variables**: Only 2 environment variables instead of
   10+
5. **No A/B Testing Complexity**: Removed the entire A/B testing framework

## Environment Variables

### Before (10+ variables)

- `CAIROM_AGG_MIR`
- `CAIROM_VALUE_LOWERING`
- `CAIROM_AGGREGATE_FOLDING`
- `CAIROM_LOWER_AGGREGATES`
- `CAIROM_DUMP_AGG_MIR`
- `CAIROM_PIPELINE_TIMING`
- `CAIROM_CONDITIONAL_PASSES`
- etc...

### After (2 variables)

- `CAIRO_M_OPT_LEVEL` - Set optimization level (0-3)
- `CAIRO_M_DEBUG` - Enable debug output (true/false)

## Usage

### Default Configuration

```rust
let config = PipelineConfig::default();
// optimization_level: Standard
// debug: false
// lower_aggregates_to_memory: false
```

### Debug Configuration

```rust
let config = PipelineConfig::debug();
// Same as default but with debug: true
```

### No Optimization (for debugging)

```rust
let config = PipelineConfig::no_opt();
// optimization_level: None
```

### From Environment

```bash
export CAIRO_M_OPT_LEVEL=2    # Standard optimizations
export CAIRO_M_DEBUG=1         # Enable debug output
```

```rust
let config = PipelineConfig::from_environment();
```

## Optimization Levels

- **None (0)**: No optimizations, useful for debugging
- **Basic (1)**: Only dead code elimination
- **Standard (2)**: Default - includes constant folding, conditional memory
  passes, SSA destruction
- **Aggressive (3)**: Currently same as Standard, room for future aggressive
  optimizations

## Files Changed

### Removed

- `mir/src/backend.rs` - Unnecessary backend abstraction
- `mir/src/testing/ab_test.rs` - Overly complex A/B testing

### Simplified

- `mir/src/pipeline.rs` - From 300+ lines to ~160 lines
- `mir/src/passes.rs` - Removed complex configuration logic
- `codegen/src/backend.rs` - Simplified to direct CASM generation

### Updated

- Documentation to reflect simplified configuration
- Tests to use new simple configuration

## Philosophy

The simplification follows these principles:

1. **Defaults Should Be Best**: The default configuration should be optimal for
   99% of use cases
2. **Avoid Premature Abstraction**: Don't build pluggable systems until you need
   them
3. **Simple > Configurable**: A simple system that works well is better than a
   complex configurable one
4. **Standard Conventions**: Use familiar patterns (optimization levels 0-3)
   that developers expect

## Migration

For code using the old configuration:

### Old

```rust
let config = PipelineConfig {
    backend_config: BackendConfig::default(),
    run_mir_optimizations: true,
    run_backend_optimizations: true,
    aggregate_features: AggregateConfig {
        enable_aggregate_mir: true,
        value_based_lowering: true,
        enable_aggregate_instructions: true,
        enable_var_ssa: true,
    },
    optimization_features: OptimizationConfig {
        enable_aggregate_folding: true,
        conditional_memory_passes: true,
        enable_pre_opt_aggregates: true,
    },
    debug_config: DebugConfig {
        dump_aggregate_mir: false,
        enable_timing: false,
        validate_aggregate_ops: true,
    },
};
```

### New

```rust
let config = PipelineConfig::default();  // That's it!
```

## Conclusion

The pipeline configuration is now simple, intuitive, and follows standard
compiler conventions. The aggregate-first MIR design is always enabled by
default (as it should be - it's strictly better), and the only real choices are:

1. How much optimization do you want? (0-3)
2. Do you want debug output?
3. Do you need memory compatibility for a specific backend?

This represents a significant improvement in maintainability and usability while
preserving all the functionality that matters.
