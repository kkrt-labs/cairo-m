# Scalar Replacement of Aggregates (SROA) Implementation Task

## Executive Summary

Implement a Scalar Replacement of Aggregates (SROA) optimization pass for the
Cairo-M MIR that decomposes tuples and structs into per-field SSA values,
eliminating unnecessary aggregate construction and copying. The pass will
materialize aggregates on-demand only when required for ABI boundaries (calls,
stores), leading to improved register allocation and enabling better downstream
optimizations.

## Background & Motivation

### Why SROA?

The current MIR uses value-based aggregate operations (`MakeTuple`,
`MakeStruct`, `ExtractField`, etc.) which often result in:

- Unnecessary aggregate construction followed immediately by extraction
- Redundant copying of entire aggregates when only individual fields are used
- Missed optimization opportunities due to opaque aggregate values
- Increased register pressure from aggregate temporaries

### Expected Benefits

1. **Eliminate redundant operations**: Remove MakeTuple/MakeStruct when all uses
   are field extractions
2. **Enable better optimizations**: Scalar values expose more opportunities for
   CSE, constant folding, and copy propagation
3. **Reduce memory traffic**: Avoid materializing aggregates that never escape
   to memory
4. **Prepare for efficient codegen**: Lower-level code prefers scalars over
   aggregates

## Design Overview

### Core Concept

SROA tracks aggregates as collections of per-field SSA values rather than
monolithic values. Aggregates are only materialized (rebuilt) at
"materialization sites" where the full aggregate is required:

- Function calls expecting aggregate parameters
- Store instructions with aggregate types
- AddressOf operations on aggregates
- PHI nodes with aggregate types (Phase 1: skip these)

### Architectural Fit

The pass integrates seamlessly with the existing MIR infrastructure:

- Implements the `MirPass` trait
- Works with existing instruction types
- Preserves SSA form and CFG structure
- Maintains PHI-first basic block invariant

## Implementation Plan

### Phase 1: Core SROA (This Task)

#### 1. Pass Structure (`passes/sroa.rs`)

```rust
pub struct ScalarReplacementOfAggregates {
}
```

#### 2. Aggregate State Tracking

```rust
struct AggState {
    /// Per-field values in declaration order
    elems: Vec<Value>,
    /// Original aggregate type for materialization
    ty: MirType,
}

/// Map from aggregate ValueId to its decomposed state
type AggregateMap = FxHashMap<ValueId, AggState>;
```

#### 3. Core Algorithm

**Per-block processing** (preserve PHI prefix):

1. Copy PHI instructions unchanged
2. Track aggregate states through the block
3. Transform non-PHI instructions:
   - `MakeTuple/MakeStruct` → Track state, drop instruction
   - `ExtractField/ExtractTuple` → Replace with scalar `Assign`
   - `InsertField/InsertTuple` → Update state, drop instruction
   - `Call/Store` → Materialize aggregates as needed

#### 4. Materialization Logic

```rust
fn materialize(
    func: &mut MirFunction,
    state: &AggState,
    ty: &MirType,
) -> ValueId {
    let dest = func.new_typed_value_id(ty.clone());
    match ty {
        MirType::Tuple(_) => {
            Instruction::make_tuple(dest, state.elems.clone())
        }
        MirType::Struct { fields, .. } => {
            let field_pairs = reconstruct_fields(state, fields);
            Instruction::make_struct(dest, field_pairs, ty.clone())
        }
        _ => unreachable!()
    }
}
```

#### 5. Instruction Classification

**Scalarizable uses** (no materialization):

- `ExtractTupleElement`, `ExtractStructField`
- `InsertTuple`, `InsertField`
- `Assign` (aggregate copy becomes state copy)
- `Debug` instructions
- Arithmetic/comparison after extraction

**Materialization-forcing uses**:

- `Call` when `signature.param_types[i]` is aggregate
- `Store` when `ty` is aggregate
- `AddressOf` when operand type is aggregate
- `Phi` instructions (Phase 1: mark non-scalarizable)

### Phase 2: Advanced Features (Future)

- **Aggregate PHI splitting**: Convert aggregate PHIs to per-field PHIs
- **Partial escape analysis**: Track which fields escape
- **Array support**: Limited scalarization for small fixed-size arrays
- **Inter-procedural analysis**: Propagate across function boundaries

## Integration Points

### 1. Pass Manager Integration

```rust
// In passes.rs
pub mod sroa;
pub use sroa::ScalarReplacementOfAggregates;

// In PassManager::standard_pipeline()
Self::new()
    .add_pass(ScalarReplacementOfAggregates::new())  // Early, before other opts
    .add_pass(ArithmeticSimplify::new())
    .add_pass(ConstantFolding::new())
    .add_pass(CopyPropagation::new())
    .add_pass(LocalCSE::new())
    // ...
```

### 2. Module Exports

```rust
// In lib.rs
pub use passes::sroa::ScalarReplacementOfAggregates;
```

## Testing Strategy

### Unit Tests

1. **Simple tuple scalarization**:

   ```rust
   let t = (x, y);
   return t.0 + t.1;
   // → No MakeTuple, direct use of x + y
   ```

2. **Partial updates**:

   ```rust
   let t = (x, y);
   t = insert(t, 1, z);
   return t.1;
   // → Returns z directly
   ```

3. **Call materialization**:

   ```rust
   let t = (x, y);
   foo(t);  // foo expects tuple
   // → MakeTuple appears just before call
   ```

4. **Store materialization**:

   ```rust
   let s = MyStruct { a: x, b: y };
   *ptr = s;
   // → MakeStruct appears just before store
   ```

5. **PHI preservation** (Phase 1):
   ```rust
   // Verify PHIs remain unchanged
   // Aggregates flowing through PHIs stay as-is
   ```

### Validation

- Run `MirModule::validate()` after pass
- Verify PHI-first invariant maintained
- Check type consistency at materialization sites
- Ensure no use-before-def violations

### Benchmarks

Track metrics:

- Instruction count reduction
- Aggregate operations eliminated
- Materializations inserted
- Compile time impact

## Implementation Checklist

- [ ] Create `passes/sroa.rs` with pass skeleton
- [ ] Implement `AggState` and tracking structures
- [ ] Add instruction classification logic
- [ ] Implement core rewrite loop with PHI preservation
- [ ] Add materialization helpers
- [ ] Handle `Assign` forwarding for aggregate copies
- [ ] Integrate with `PassManager`
- [ ] Add module exports
- [ ] Write unit tests
- [ ] Add snapshot tests
- [ ] Run validation suite
- [ ] Document pass behavior
- [ ] Measure performance impact

## Success Criteria

1. **Correctness**: All existing tests pass with SROA enabled
2. **Effectiveness**: 50%+ reduction in aggregate operations for typical code
3. **Performance**: <5% compile time overhead
4. **Robustness**: Handles all aggregate patterns correctly
5. **Maintainability**: Clean, documented implementation

## Code Quality Requirements

- Follow existing pass structure (see `copy_propagation.rs`)
- Use `FxHashMap` for performance
- Add debug logging with `log::debug!`
- Include inline documentation
- Handle edge cases gracefully
- Preserve source spans for debugging

## Non-Goals (Phase 1)

- Arrays (require memory operations)
- Aggregate PHI splitting (complex CFG updates)
- Partial field tracking (full escape analysis)
- Cross-function optimization
- Nested aggregate flattening

## References

- LLVM SROA: Aggressive splitting with on-demand materialization
- GCC SRA: Clear goals with tunable heuristics
- Escape Analysis (Choi et al.): Materialization concepts
- Partial Escape Analysis (Stadler): Java's approach to scalar replacement

## Risk Mitigation

1. **Conservative defaults**: Start with small aggregates only
2. **Validation pass**: Run after SROA to catch invariant violations
3. **Incremental rollout**: Add behind feature flag initially
4. **Extensive testing**: Cover all instruction combinations
5. **Fallback path**: Skip complex patterns in Phase 1

## Next Steps

After Phase 1 completion:

1. Analyze real-world impact on Cairo-M programs
2. Identify patterns that need Phase 2 features
3. Design PHI splitting algorithm
4. Consider integration with mem2reg for unified aggregate handling
