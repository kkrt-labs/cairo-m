# Implement Pass Scheduling with Fixed-Point Iteration

**Priority**: HIGH  
**Component**: MIR Pass Infrastructure  
**Impact**: Correctness, Performance

## Problem

Current pass management is overly simplistic and prevents effective
optimization:

### Current Limitations

1. **No fixed-point iteration**: Each pass runs exactly once per pipeline
2. **Manual pass ordering**: Risk of passes undoing each other's work
3. **No convergence detection**: Can't tell when optimization reaches steady
   state
4. **Pipeline rigidity**: Same ordering for all functions regardless of
   characteristics

### Impact on Optimization Quality

- **SROA vs Copy Propagation**: SROA can expose new copy propagation
  opportunities
- **Constant Folding vs Simplification**: Each can enable the other
- **Dead Code vs Other Passes**: DCE should run after other passes create dead
  code
- **Missed optimization opportunities**: Passes that could benefit from multiple
  rounds

## Solution

### Enhanced PassManager with Fixed-Point Support

**Update**: `crates/compiler/mir/src/passes.rs`

```rust
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// Maximum iterations for fixed-point convergence
    pub max_iterations: usize,
    /// Enable detailed logging of pass execution
    pub log_passes: bool,
    /// Passes that should run to fixed-point
    pub fixedpoint_passes: Vec<String>,
    /// Passes that should only run once
    pub single_shot_passes: Vec<String>,
}

impl PassManager {
    /// Run passes until convergence or max iterations
    pub fn run_to_fixedpoint(&mut self, function: &mut MirFunction) -> PassStats {
        let mut stats = PassStats::new();
        let mut iteration = 0;

        while iteration < self.config.max_iterations {
            let mut any_changed = false;

            for pass in &mut self.passes {
                let changed = pass.run(function);
                stats.record_pass_run(pass.name(), changed);
                any_changed |= changed;

                if self.config.log_passes {
                    println!("Pass {} iteration {}: changed={}",
                             pass.name(), iteration, changed);
                }
            }

            if !any_changed {
                break; // Converged
            }

            iteration += 1;
        }

        stats.iterations = iteration;
        stats
    }

    /// Create pipeline with intelligent pass ordering
    pub fn intelligent_pipeline() -> Self {
        Self {
            passes: vec![
                // Phase 1: Structure optimization
                Box::new(SROA::new()),
                Box::new(PhiElimination::new()),

                // Phase 2: Value optimization (to fixed-point)
                Box::new(ConstantFolding::new()),
                Box::new(CopyPropagation::new()),
                Box::new(ArithmeticSimplify::new()),

                // Phase 3: Control flow optimization
                Box::new(SimplifyBranches::new()),
                Box::new(DeadCodeElimination::new()),

                // Phase 4: Final cleanup
                Box::new(NopElimination::new()),
            ],
            config: PassConfig {
                max_iterations: 10,
                fixedpoint_passes: vec![
                    "ConstantFolding".to_string(),
                    "CopyPropagation".to_string(),
                    "ArithmeticSimplify".to_string(),
                ],
                single_shot_passes: vec![
                    "SROA".to_string(),
                    "PhiElimination".to_string(),
                ],
                log_passes: false,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct PassStats {
    pub iterations: usize,
    pub passes_run: HashMap<String, usize>,
    pub passes_changed: HashMap<String, usize>,
    pub total_time: Duration,
}
```

### Pass Scheduling Strategies

1. **Fixed-Point Groups**: Run value optimization passes to convergence
2. **Dependency-Aware**: Ensure prerequisites run before dependent passes
3. **Adaptive Ordering**: Different strategies for different function
   characteristics

### Integration with Pipeline

```rust
impl PipelineConfig {
    pub fn intelligent() -> Self {
        Self {
            optimization_level: OptimizationLevel::Standard,
            pass_strategy: PassStrategy::FixedPoint,
            debug: false,
        }
    }
}

pub fn optimize_module(module: &mut MirModule, config: &PipelineConfig) {
    let mut pass_manager = match config.pass_strategy {
        PassStrategy::SinglePass => PassManager::standard_pipeline(),
        PassStrategy::FixedPoint => PassManager::intelligent_pipeline(),
        PassStrategy::Aggressive => PassManager::aggressive_fixedpoint_pipeline(),
    };

    for function in module.functions_mut() {
        let stats = pass_manager.run_to_fixedpoint(function);

        if config.debug {
            println!("Optimization stats for {}: {:?}", function.name, stats);
        }
    }
}
```

## Files to Modify

- **Update**: `crates/compiler/mir/src/passes.rs` - Add PassConfig and
  fixed-point logic
- **Update**: `crates/compiler/mir/src/pipeline.rs` - Add PassStrategy enum
- **New**: `crates/compiler/mir/src/passes/pass_stats.rs` - Statistics tracking
- **Tests**: Add fixed-point convergence tests

## Implementation Plan

### Phase 1: Basic Fixed-Point Infrastructure

1. Add PassConfig with iteration limits
2. Implement `run_to_fixedpoint` method
3. Add convergence detection

### Phase 2: Intelligent Pass Ordering

1. Define pass dependency relationships
2. Implement adaptive scheduling
3. Add per-pass iteration controls

### Phase 3: Statistics and Diagnostics

1. Track pass execution statistics
2. Add timing information
3. Implement convergence diagnostics

### Phase 4: Pipeline Integration

1. Update pipeline.rs to use new PassManager
2. Add configuration options
3. Update CLI to expose new options

## Test Strategy

```rust
#[test]
fn test_fixed_point_convergence() {
    // Function that requires multiple rounds of optimization
    let mut function = create_test_function_with_nested_opportunities();

    let mut pass_manager = PassManager::intelligent_pipeline();
    let stats = pass_manager.run_to_fixedpoint(&mut function);

    assert!(stats.iterations > 1);
    assert!(stats.iterations < 10);
    assert_eq!(function.count_instructions_of_type::<ConstantFolding>(), 0);
}

#[test]
fn test_convergence_detection() {
    let mut function = create_already_optimized_function();

    let mut pass_manager = PassManager::intelligent_pipeline();
    let stats = pass_manager.run_to_fixedpoint(&mut function);

    assert_eq!(stats.iterations, 1); // Should converge immediately
}
```

## Benefits

1. **Better optimization quality**: Multiple rounds allow passes to build on
   each other
2. **Automatic convergence**: No manual tuning of pass ordering
3. **Performance insights**: Statistics help understand optimization behavior
4. **Robust termination**: Iteration limits prevent infinite loops
5. **Configurable behavior**: Different strategies for different use cases

## Dependencies

- Should implement after fixing constant folding semantics (Task #1)
- Coordinate with centralized const eval module (Task #2)

## Acceptance Criteria

- [ ] PassManager supports fixed-point iteration with configurable limits
- [ ] Intelligent pass ordering based on dependencies
- [ ] Convergence detection prevents unnecessary iterations
- [ ] Statistics tracking for performance analysis
- [ ] Integration with existing pipeline infrastructure
- [ ] All existing tests pass with improved optimization
- [ ] New tests verify convergence behavior
