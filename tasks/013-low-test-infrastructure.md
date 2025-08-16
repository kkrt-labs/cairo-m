# 013 - Test Infrastructure for Aggregate-First MIR

**Priority:** LOW  
**Dependencies:** Most other tasks (comprehensive testing)

## Why

Comprehensive testing is critical for the successful migration to an
aggregate-first MIR design. The introduction of new instructions, optimization
passes, and architectural changes requires a robust testing strategy to:

1. **Validate correctness** of the new aggregate-first lowering compared to the
   old memory-based approach
2. **Prevent regressions** as we incrementally migrate from memory-based to
   value-based operations
3. **Ensure compatibility** between old and new MIR generation modes during the
   transition period
4. **Verify performance improvements** from eliminating unnecessary SROA/Mem2Reg
   passes
5. **Test edge cases** in the new Variable-SSA pass and aggregate optimizations
6. **Maintain confidence** in the compiler's output throughout the refactoring
   process

Without proper test coverage, the migration risks introducing subtle bugs,
performance regressions, or breaking existing functionality that depends on the
current MIR structure.

## What

The test infrastructure needs to cover multiple dimensions of the
aggregate-first MIR implementation:

### Unit Tests

- **New instruction validation**: Tests for `MakeTuple`, `ExtractTupleElement`,
  `MakeStruct`, `ExtractStructField`, and `InsertField` instructions
- **Builder API**: Tests for new aggregate construction and extraction methods
- **Pass correctness**: Individual tests for the Variable-SSA pass,
  pre-optimization folding, and aggregate-aware validation
- **Type system integration**: Tests ensuring aggregate operations respect type
  constraints

### Integration Tests

- **Common patterns**: Real-world usage scenarios involving tuple/struct
  creation, access, and mutation
- **Control flow interaction**: Testing aggregate handling across branches,
  loops, and function calls
- **Multi-value returns**: Testing function calls that return multiple values
  and their integration with tuple contexts
- **Assignment patterns**: Testing various forms of aggregate assignment and
  destructuring

### Regression Tests

- **A/B comparison**: Tests that can run in both "aggregate MIR on/off" modes to
  ensure behavioral equivalence
- **Performance benchmarks**: Measuring compilation time and generated code
  quality improvements
- **Memory usage**: Ensuring the new approach doesn't introduce memory leaks or
  excessive allocations

### Snapshot Tests

- **MIR generation**: Golden files showing the expected MIR output for various
  aggregate operations
- **Optimization results**: Before/after snapshots demonstrating the
  effectiveness of aggregate folding
- **Pretty-printing**: Ensuring readable output for debugging and development

## How

### 1. Unit Tests for New Instructions and Passes

**File:** `crates/compiler/mir/src/instruction.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_tuple_instruction() {
        // Test creating tuple instructions with various element types
        // Verify destinations(), used_values(), and validation
    }

    #[test]
    fn test_extract_tuple_element() {
        // Test tuple element extraction with bounds checking
        // Test type consistency between tuple and extracted element
    }

    #[test]
    fn test_make_struct_instruction() {
        // Test struct creation with field ordering and types
        // Test handling of unknown or invalid field names
    }

    #[test]
    fn test_extract_struct_field() {
        // Test field extraction with type validation
        // Test error handling for non-existent fields
    }
}
```

**File:** `crates/compiler/mir/src/passes/var_ssa.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_phi_insertion() {
        // Test: let x = 0; if c { x = 1 } else { x = 2 }; return x
        // Should generate one Phi in merge block
    }

    #[test]
    fn test_struct_variable_ssa() {
        // Test: x = MakeStruct(...); x = InsertField(x,"f",v); return x
        // Should handle struct reassignment correctly
    }
}
```

**File:** `crates/compiler/mir/src/passes/pre_opt.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_make_tuple_folding() {
        // ExtractTuple(MakeTuple(vs), i) → vs[i]
        let before_count = function.instruction_count();
        pre_opt_pass.run(&mut function);
        assert!(function.instruction_count() < before_count);
    }

    #[test]
    fn test_extract_make_struct_folding() {
        // ExtractField(MakeStruct{… f: v …}, "f") → v
    }

    #[test]
    fn test_insert_field_folding() {
        // InsertField(MakeStruct{… f: old …}, "f", v) → MakeStruct{… f: v …}
    }
}
```

### 2. Integration Tests for Common Patterns

**File:** `crates/compiler/mir/tests/aggregate_patterns.rs`

```rust
use cairo_m_compiler_mir::*;

#[test]
fn test_tuple_literal_and_access() {
    let source = r#"
        func main() -> felt {
            let t = (1, 2);
            return t.0 + t.1;
        }
    "#;

    // Test both aggregate-first and memory-based lowering
    test_both_modes(source, |mir| {
        // Aggregate mode: should contain MakeTuple and ExtractTuple
        // Memory mode: should contain frame_alloc and loads
        // Both should produce same semantic result
    });
}

#[test]
fn test_struct_creation_and_modification() {
    let source = r#"
        struct Point { x: felt, y: felt }

        func main() -> felt {
            let mut p = Point { x: 10, y: 20 };
            p.x = 15;
            return p.x + p.y;
        }
    "#;

    test_both_modes(source, |mir| {
        // Aggregate mode: MakeStruct, InsertField, ExtractField
        // Memory mode: frame_alloc, store, load
    });
}

#[test]
fn test_control_flow_with_aggregates() {
    let source = r#"
        func conditional_tuple(flag: felt) -> felt {
            let t = if flag {
                (1, 2)
            } else {
                (3, 4)
            };
            return t.0 + t.1;
        }
    "#;

    test_both_modes(source, |mir| {
        // Should handle Phi nodes for tuple values correctly
        // Variable-SSA should insert appropriate Phi instructions
    });
}

#[test]
fn test_multi_value_function_returns() {
    let source = r#"
        func get_pair() -> (felt, felt) {
            return (42, 84);
        }

        func main() -> felt {
            let pair = get_pair();
            return pair.0 + pair.1;
        }
    "#;

    test_both_modes(source, |mir| {
        // Test MakeTuple synthesis for multi-value returns
        // Test direct indexing optimization: get_pair().0
    });
}

fn test_both_modes<F>(source: &str, validator: F)
where F: Fn(&MirModule) {
    // Test with aggregate MIR enabled
    let mir_agg = compile_with_config(source, AggregateConfig::Enabled);
    validator(&mir_agg);

    // Test with aggregate MIR disabled (fallback to memory)
    let mir_mem = compile_with_config(source, AggregateConfig::Disabled);
    validator(&mir_mem);

    // Ensure both produce semantically equivalent results
    assert_equivalent_execution(&mir_agg, &mir_mem);
}
```

### 3. Snapshot Test Updates

**File:** `crates/compiler/mir/tests/snapshots/`

Create comprehensive snapshot tests for MIR generation:

```rust
#[test]
fn test_aggregate_mir_snapshots() {
    insta::glob!("fixtures/*.cm", |path| {
        let source = std::fs::read_to_string(path).unwrap();
        let mir = compile_to_mir(&source);

        // Snapshot the pretty-printed MIR
        insta::assert_snapshot!(mir.pretty_print());
    });
}

// Test fixtures to create:
// - fixtures/tuple_basic.cm: Simple tuple creation and access
// - fixtures/struct_basic.cm: Struct creation, field access, modification
// - fixtures/nested_aggregates.cm: Tuples of structs, structs with tuple fields
// - fixtures/control_flow.cm: Aggregates across if/while/match
// - fixtures/function_calls.cm: Multi-value returns, tuple arguments
```

### 4. Performance Regression Testing

**File:** `crates/compiler/mir/benches/aggregate_performance.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_compilation_time(c: &mut Criterion) {
    let large_program = generate_aggregate_heavy_program();

    c.bench_function("compile_aggregate_mir", |b| {
        b.iter(|| {
            compile_with_config(black_box(&large_program), AggregateConfig::Enabled)
        })
    });

    c.bench_function("compile_memory_mir", |b| {
        b.iter(|| {
            compile_with_config(black_box(&large_program), AggregateConfig::Disabled)
        })
    });
}

fn benchmark_optimization_passes(c: &mut Criterion) {
    let mir_module = load_test_mir();

    c.bench_function("pre_optimization_pass", |b| {
        b.iter(|| {
            let mut module = mir_module.clone();
            PreOptimizationPass::new().run_on_module(black_box(&mut module));
        })
    });

    // Compare against old SROA/Mem2Reg pipeline
    c.bench_function("old_optimization_pipeline", |b| {
        b.iter(|| {
            let mut module = mir_module.clone();
            run_legacy_optimization_pipeline(black_box(&mut module));
        })
    });
}

criterion_group!(benches, benchmark_compilation_time, benchmark_optimization_passes);
criterion_main!(benches);
```

### 5. Configuration and Environment Testing

**File:** `crates/compiler/mir/tests/config_tests.rs`

```rust
#[test]
fn test_aggregate_mir_toggle() {
    let source = sample_aggregate_program();

    // Test environment variable
    std::env::set_var("CAIROM_AGG_MIR", "1");
    let mir_env_on = compile_to_mir(source);

    std::env::set_var("CAIROM_AGG_MIR", "0");
    let mir_env_off = compile_to_mir(source);

    // Test explicit config
    let config_on = PipelineConfig::default()
        .with_option("agg_mir", "on");
    let mir_config_on = compile_with_config(source, config_on);

    assert_mir_mode_matches(&mir_env_on, AggregateMode::Enabled);
    assert_mir_mode_matches(&mir_env_off, AggregateMode::Disabled);
    assert_equivalent_mir(&mir_env_on, &mir_config_on);
}

#[test]
fn test_pipeline_conditional_passes() {
    let memory_heavy_source = generate_array_heavy_program();
    let aggregate_heavy_source = generate_struct_heavy_program();

    // Memory-heavy programs should still use Mem2Reg
    let mir_mem = compile_to_mir(memory_heavy_source);
    assert!(pipeline_used_mem2reg(&mir_mem));

    // Aggregate-only programs should skip Mem2Reg
    let mir_agg = compile_to_mir(aggregate_heavy_source);
    assert!(!pipeline_used_mem2reg(&mir_agg));
}
```

### 6. Test Organization and Execution

**CI Integration:**

```yaml
# In .github/workflows/test.yml
- name: Run MIR tests in both modes
  run: |
    # Test aggregate-first mode (default)
    cargo test -p cairo-m-compiler-mir

    # Test legacy memory mode
    CAIROM_AGG_MIR=0 cargo test -p cairo-m-compiler-mir

    # Run performance benchmarks
    cargo bench --bench aggregate_performance

- name: Snapshot review check
  run: |
    cargo insta test
    # Fail if snapshots need review
    if cargo insta pending-snapshots | grep -q "pending"; then
      echo "Snapshot tests have pending changes"
      exit 1
    fi
```

**Test Utilities:**

```rust
// In mir/tests/utils.rs
pub fn assert_equivalent_execution(mir_a: &MirModule, mir_b: &MirModule) {
    // Run both through the interpreter/codegen and compare results
    let result_a = execute_mir(mir_a);
    let result_b = execute_mir(mir_b);
    assert_eq!(result_a, result_b, "MIR implementations produce different results");
}

pub fn assert_mir_mode_matches(mir: &MirModule, expected_mode: AggregateMode) {
    match expected_mode {
        AggregateMode::Enabled => {
            assert!(mir.uses_aggregate_instructions());
            assert!(!mir.uses_memory_for_aggregates());
        }
        AggregateMode::Disabled => {
            assert!(!mir.uses_aggregate_instructions());
            assert!(mir.uses_memory_for_aggregates());
        }
    }
}

pub fn generate_aggregate_heavy_program() -> String {
    // Generate program with many tuple/struct operations
    // to stress-test the new instruction set
}
```

This comprehensive testing strategy ensures that the aggregate-first MIR
migration is thoroughly validated, maintains backward compatibility during
transition, and delivers the expected performance improvements while preventing
regressions.
