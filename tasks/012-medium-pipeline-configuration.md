# Task 012: Pipeline Configuration for MIR Feature Flags

**Priority:** MEDIUM  
**Dependencies:** Multiple tasks (needs overall infrastructure)

## Why

During the migration from memory-centric to aggregate-first MIR, we need robust
configuration infrastructure to enable safe rollout and testing of new features.
The current MIR refactoring involves significant changes to core compiler
behavior, including new instruction types, modified lowering logic, and
simplified optimization passes. Without proper feature flags and configuration
management, this migration poses risks to stability and makes it difficult to:

1. **Perform A/B testing** between old and new MIR generation approaches
2. **Gradual rollout** of new aggregate instructions while maintaining backward
   compatibility
3. **Debug and isolate issues** by selectively enabling/disabling features
4. **Support different backend requirements** that may need different MIR
   representations
5. **Maintain CI/CD stability** during the transition period

The lack of configuration infrastructure forces an all-or-nothing approach to
MIR changes, making it harder to identify regressions and reducing confidence in
incremental updates.

## What

This task implements a comprehensive pipeline configuration system that
provides:

### Core Configuration Infrastructure

- **PipelineConfig struct** with feature flag management capabilities
- **Environment variable integration** for runtime configuration (e.g.,
  `CAIROM_AGG_MIR=1`)
- **Compile-time feature toggles** for conditional compilation paths
- **Backend-specific options** for target-dependent behavior

### MIR Feature Flags

- `aggregate_mir`: Enable/disable aggregate-first MIR generation (default: on)
- `value_based_lowering`: Toggle between memory-based and value-based aggregate
  lowering
- `optimize_aggregates`: Enable/disable aggregate-specific optimization passes
- `late_aggregate_lowering`: Backend compatibility mode for aggregate→memory
  conversion

### Configuration Scopes

- **Global settings** affecting the entire compilation pipeline
- **Per-function toggles** for fine-grained control during testing
- **Backend configuration** for target-specific behavior
- **Debug/development flags** for enhanced logging and validation

### A/B Testing Infrastructure

- **Dual-mode execution** capability for comparing old vs new MIR generation
- **Golden file testing** with multiple configuration variants
- **Performance benchmarking** across different pipeline configurations
- **Regression detection** through automated comparison testing

## How

### 1. Adding PipelineConfig Options

**Create core configuration infrastructure:**

```rust
// In crates/compiler/mir/src/pipeline/config.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineConfig {
    pub aggregate_features: AggregateConfig,
    pub optimization_features: OptimizationConfig,
    pub backend_config: BackendConfig,
    pub debug_config: DebugConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateConfig {
    /// Enable aggregate-first MIR generation
    pub enable_aggregate_mir: bool,
    /// Use value-based lowering for tuples/structs
    pub value_based_lowering: bool,
    /// Enable new aggregate instruction types
    pub enable_aggregate_instructions: bool,
    /// Enable variable-SSA pass for aggregate values
    pub enable_var_ssa: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationConfig {
    /// Enable aggregate-specific constant folding
    pub enable_aggregate_folding: bool,
    /// Skip SROA/Mem2Reg for value-based functions
    pub conditional_memory_passes: bool,
    /// Enable pre-optimization aggregate simplifications
    pub enable_pre_opt_aggregates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendConfig {
    /// Force late aggregate lowering for backend compatibility
    pub force_late_aggregate_lowering: bool,
    /// Backend-specific option overrides
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugConfig {
    /// Enable verbose MIR dumping for aggregate operations
    pub dump_aggregate_mir: bool,
    /// Enable pipeline timing information
    pub enable_timing: bool,
    /// Enable validation warnings for new instruction types
    pub validate_aggregate_ops: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
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
            backend_config: BackendConfig {
                force_late_aggregate_lowering: false,
                options: HashMap::new(),
            },
            debug_config: DebugConfig {
                dump_aggregate_mir: false,
                enable_timing: false,
                validate_aggregate_ops: true,
            },
        }
    }
}
```

### 2. Environment Variable Support

**Implement environment-based configuration:**

```rust
// In crates/compiler/mir/src/pipeline/config.rs
impl PipelineConfig {
    pub fn from_environment() -> Self {
        let mut config = Self::default();

        // Primary aggregate MIR toggle
        if let Ok(val) = env::var("CAIROM_AGG_MIR") {
            let enable = val == "1" || val.to_lowercase() == "true";
            config.aggregate_features.enable_aggregate_mir = enable;

            // If aggregate MIR is disabled, disable dependent features
            if !enable {
                config.aggregate_features.value_based_lowering = false;
                config.aggregate_features.enable_aggregate_instructions = false;
                config.optimization_features.enable_aggregate_folding = false;
            }
        }

        // Fine-grained feature toggles
        if let Ok(val) = env::var("CAIROM_VALUE_LOWERING") {
            config.aggregate_features.value_based_lowering = val == "1";
        }

        if let Ok(val) = env::var("CAIROM_AGGREGATE_FOLDING") {
            config.optimization_features.enable_aggregate_folding = val == "1";
        }

        if let Ok(val) = env::var("CAIROM_LATE_AGG_LOWERING") {
            config.backend_config.force_late_aggregate_lowering = val == "1";
        }

        // Debug flags
        if let Ok(val) = env::var("CAIROM_DUMP_AGG_MIR") {
            config.debug_config.dump_aggregate_mir = val == "1";
        }

        if let Ok(val) = env::var("CAIROM_PIPELINE_TIMING") {
            config.debug_config.enable_timing = val == "1";
        }

        config
    }

    pub fn with_override(mut self, key: &str, value: &str) -> Self {
        match key {
            "aggregate_mir" => self.aggregate_features.enable_aggregate_mir = value == "true",
            "value_lowering" => self.aggregate_features.value_based_lowering = value == "true",
            "aggregate_folding" => self.optimization_features.enable_aggregate_folding = value == "true",
            "late_lowering" => self.backend_config.force_late_aggregate_lowering = value == "true",
            _ => {
                self.backend_config.options.insert(key.to_string(), value.to_string());
            }
        }
        self
    }
}
```

### 3. Feature Flag Plumbing

**Integrate configuration throughout the pipeline:**

```rust
// In crates/compiler/mir/src/passes.rs
impl PassManager {
    pub fn standard_pipeline_with_config(config: &PipelineConfig) -> Self {
        let mut manager = PassManager::new();

        // Always run pre-optimization
        manager.add_pass(Box::new(PreOptimizationPass::new()));

        // Conditional aggregate optimizations
        if config.optimization_features.enable_pre_opt_aggregates {
            manager.add_pass(Box::new(AggregateSimplificationPass::new()));
        }

        // Variable-SSA for aggregate values
        if config.aggregate_features.enable_var_ssa {
            manager.add_pass(Box::new(VarSsaPass::new()));
        }

        // Conditional memory optimization passes
        if config.optimization_features.conditional_memory_passes {
            manager.add_pass(Box::new(ConditionalMemoryOptPass::new(config.clone())));
        } else {
            // Always run traditional memory passes if conditional mode disabled
            manager.add_pass(Box::new(SroaPass::new()));
            manager.add_pass(Box::new(Mem2RegSsaPass::new()));
        }

        // Standard optimization passes
        manager.add_pass(Box::new(FuseCmpBranchPass::new()));
        manager.add_pass(Box::new(DeadCodeEliminationPass::new()));

        // Backend compatibility pass
        if config.backend_config.force_late_aggregate_lowering {
            manager.add_pass(Box::new(LateAggregateLoweringPass::new()));
        }

        // SSA destruction for backends that need it
        manager.add_pass(Box::new(SsaDestructionPass::new()));

        // Final validation
        manager.add_pass(Box::new(ValidationPass::new_post_ssa(config.clone())));

        manager
    }
}

// In crates/compiler/mir/src/lowering/function.rs
pub fn generate_mir_with_config(
    db: &dyn MirDatabase,
    function_id: FunctionId,
    config: &PipelineConfig,
) -> MirFunction {
    let mut builder = MirBuilder::new_with_config(db, function_id, config);
    // ... existing implementation, but now config-aware
}
```

**Update lowering to respect configuration:**

```rust
// In crates/compiler/mir/src/lowering/expr.rs
impl MirBuilder {
    fn lower_tuple_literal(&mut self, tuple: &TupleLiteral) -> Value {
        if self.config.aggregate_features.value_based_lowering {
            // New aggregate-first approach
            let elements: Vec<Value> = tuple.elements
                .iter()
                .map(|expr| self.lower_expression(expr))
                .collect();

            let dest = self.allocate_value(self.get_expr_type(&tuple.into()));
            self.emit_instruction(Instruction::make_tuple(dest, elements));
            Value::Operand(dest)
        } else {
            // Legacy memory-based approach
            self.lower_tuple_literal_legacy(tuple)
        }
    }

    fn lower_struct_literal(&mut self, struct_lit: &StructLiteral) -> Value {
        if self.config.aggregate_features.value_based_lowering {
            // New aggregate-first approach
            let fields: Vec<(String, Value)> = struct_lit.fields
                .iter()
                .map(|field| {
                    let value = self.lower_expression(&field.value);
                    (field.name.clone(), value)
                })
                .collect();

            let dest = self.allocate_value(self.get_expr_type(&struct_lit.into()));
            let struct_ty = self.get_expr_type(&struct_lit.into());
            self.emit_instruction(Instruction::make_struct(dest, fields, struct_ty));
            Value::Operand(dest)
        } else {
            // Legacy memory-based approach
            self.lower_struct_literal_legacy(struct_lit)
        }
    }
}
```

### 4. A/B Testing Infrastructure

**Implement comparison testing framework:**

```rust
// In crates/compiler/mir/src/testing/ab_test.rs
pub struct ABTestRunner {
    baseline_config: PipelineConfig,
    experimental_config: PipelineConfig,
}

impl ABTestRunner {
    pub fn new() -> Self {
        Self {
            baseline_config: PipelineConfig {
                aggregate_features: AggregateConfig {
                    enable_aggregate_mir: false,
                    value_based_lowering: false,
                    enable_aggregate_instructions: false,
                    enable_var_ssa: false,
                },
                ..Default::default()
            },
            experimental_config: PipelineConfig::default(),
        }
    }

    pub fn run_comparison_test(&self, source: &str) -> ABTestResult {
        let baseline_result = self.compile_with_config(source, &self.baseline_config);
        let experimental_result = self.compile_with_config(source, &self.experimental_config);

        ABTestResult {
            baseline: baseline_result,
            experimental: experimental_result,
            comparison: self.compare_results(&baseline_result, &experimental_result),
        }
    }

    fn compare_results(&self, baseline: &CompilationResult, experimental: &CompilationResult) -> Comparison {
        Comparison {
            instruction_count_delta: experimental.instruction_count as i32 - baseline.instruction_count as i32,
            compile_time_delta: experimental.compile_time - baseline.compile_time,
            memory_usage_delta: experimental.memory_usage as i32 - baseline.memory_usage as i32,
            correctness_match: baseline.execution_trace == experimental.execution_trace,
        }
    }
}

// In tests/integration_tests.rs
#[test]
fn test_aggregate_mir_ab_comparison() {
    let test_cases = [
        "let p = Point { x: 1, y: 2 }; p.x + p.y",
        "let t = (1, 2, 3); t.0 + t.1 + t.2",
        "fn f() -> (i32, i32) { (1, 2) } let (a, b) = f();",
    ];

    let runner = ABTestRunner::new();

    for test_case in &test_cases {
        let result = runner.run_comparison_test(test_case);
        assert!(result.comparison.correctness_match,
               "Execution traces don't match for: {}", test_case);

        // New MIR should generally have fewer instructions for aggregate operations
        assert!(result.comparison.instruction_count_delta <= 0,
               "New MIR generated more instructions than baseline for: {}", test_case);
    }
}
```

**CI Integration:**

```yaml
# In .github/workflows/mir_ab_testing.yml
name: MIR A/B Testing

on: [push, pull_request]

jobs:
  ab_test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run baseline tests
        run: |
          CAIROM_AGG_MIR=0 cargo test --test integration_tests

      - name: Run experimental tests
        run: |
          CAIROM_AGG_MIR=1 cargo test --test integration_tests

      - name: Run A/B comparison
        run: |
          cargo test --test ab_comparison_tests

      - name: Performance comparison
        run: |
          CAIROM_AGG_MIR=0 cargo bench --bench compile_benchmark -- --save-baseline baseline
          CAIROM_AGG_MIR=1 cargo bench --bench compile_benchmark -- --save-baseline experimental
          cargo bench --bench compile_benchmark -- --load-baseline baseline experimental
```

This implementation provides a robust foundation for safely rolling out the MIR
aggregate-first refactoring while maintaining compatibility, enabling thorough
testing, and providing the flexibility needed for different deployment
scenarios.
