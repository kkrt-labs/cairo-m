# Phi-Node Elimination Pass Implementation Plan

## Executive Summary

This document outlines the implementation plan for converting Cairo-M's MIR from
SSA form with phi-nodes to a non-SSA form suitable for code generation. The
transformation uses the battle-tested **parallel copy insertion** algorithm from
Sreedhar et al. (1999), as described in modern compiler literature like Cooper &
Torczon's "Engineering a Compiler" and Appel's "Modern Compiler Implementation".

## Background

### Current State

- MIR is currently in SSA form with phi-nodes (`InstructionKind::Phi`)
- SSA form enables powerful optimizations (constant propagation, DCE, CSE)
- All optimization passes rely on SSA properties

### Target State

- Non-SSA form without phi-nodes
- Variables can be assigned multiple times
- Suitable for direct CASM code generation

### Why This Pass is Necessary

Cairo Assembly (CASM) operates on a traditional register/memory model where:

- Variables are mutable memory locations
- No phi-node instruction exists
- Control flow merges use explicit assignments

## Algorithm: Parallel Copy Insertion

We implement the standard algorithm from compiler literature with critical edge
splitting:

### Phase 1: Critical Edge Splitting

**Purpose:** Ensure unambiguous placement of copy instructions

```rust
// Critical edge: predecessor has multiple successors AND successor has multiple predecessors
// Must split these edges to create a safe insertion point for copies
```

### Phase 2: Phi Decomposition

**Purpose:** Replace phi-nodes with explicit copy instructions

For each phi-node:

```
%3 = phi [%1: bb0], [%2: bb1]
```

Becomes:

```
// At end of bb0:
%3 = %1

// At end of bb1:
%3 = %2
```

### Phase 3: Copy Sequencing (Lost Copy Problem)

**Purpose:** Handle circular dependencies in parallel copies

The "lost copy problem" occurs when phi-nodes create circular dependencies:

```
x = phi [y: pred]
y = phi [x: pred]
```

Standard solution using temporary variables:

```
t = x
x = y
y = t
```

## Implementation Details

### File Structure

```
crates/compiler/mir/src/passes/phi_elimination.rs
```

### Core Components

#### 1. `PhiElimination` Pass Structure

```rust
pub struct PhiElimination {
    /// Track whether we're in debug mode
    debug: bool,
    /// Statistics for reporting
    stats: EliminationStats,
}

impl MirPass for PhiElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // Implementation here
    }

    fn name(&self) -> &'static str {
        "PhiElimination"
    }
}
```

#### 2. Critical Edge Splitting

```rust
fn split_critical_edges(&mut self, function: &mut MirFunction) {
    // Use existing cfg::split_all_critical_edges
    let splits = cfg::split_all_critical_edges(function);
    self.stats.critical_edges_split = splits.len();
}
```

#### 3. Phi Collection and Mapping

```rust
struct PhiCopySet {
    /// Maps (predecessor_block, destination) -> source_value
    copies: FxHashMap<(BasicBlockId, ValueId), Value>,
}

fn collect_phi_copies(&self, function: &MirFunction) -> FxHashMap<BasicBlockId, Vec<Instruction>> {
    let mut predecessor_copies = FxHashMap::default();

    for (block_id, block) in function.basic_blocks.iter_enumerated() {
        for phi in block.phi_instructions() {
            // Extract copy instructions for each predecessor
            for (pred_id, source_value) in &phi.sources {
                let copy = Instruction::assign(phi.dest, source_value.clone(), phi.ty.clone());
                predecessor_copies.entry(*pred_id)
                    .or_insert_with(Vec::new)
                    .push(copy);
            }
        }
    }

    predecessor_copies
}
```

#### 4. Parallel Copy Sequencing

```rust
fn sequence_parallel_copies(&self, copies: Vec<Instruction>) -> Vec<Instruction> {
    // Detect cycles using DFS
    let graph = build_dependency_graph(&copies);
    let cycles = find_cycles(&graph);

    if cycles.is_empty() {
        return topological_sort(copies);
    }

    // Break cycles with temporaries
    let mut sequenced = Vec::new();
    for cycle in cycles {
        let temp = self.new_temp_value();
        // Save first value to temp
        sequenced.push(Instruction::assign(temp, cycle[0].source, cycle[0].ty));
        // Perform cycle rotations
        for i in 0..cycle.len()-1 {
            sequenced.push(Instruction::assign(cycle[i].dest, cycle[i+1].source, cycle[i].ty));
        }
        // Restore from temp
        sequenced.push(Instruction::assign(cycle.last().dest, temp, cycle.last().ty));
    }

    // Add non-cyclic copies
    sequenced.extend(non_cyclic_copies);
    sequenced
}
```

#### 5. Copy Insertion

```rust
fn insert_copies(&mut self, function: &mut MirFunction, copies: FxHashMap<BasicBlockId, Vec<Instruction>>) {
    for (block_id, copy_instructions) in copies {
        let block = function.basic_blocks.get_mut(block_id).unwrap();

        // Sequence the copies to handle dependencies
        let sequenced = self.sequence_parallel_copies(copy_instructions);

        // Insert before terminator
        let terminator = block.terminator.clone();
        block.terminator = Terminator::Unreachable; // Temporary

        for instr in sequenced {
            block.push_instruction(instr);
        }

        block.set_terminator(terminator);
    }
}
```

#### 6. Phi Removal

```rust
fn remove_phi_nodes(&mut self, function: &mut MirFunction) {
    for block in function.basic_blocks.iter_mut() {
        let original_count = block.instructions.len();
        block.instructions.retain(|instr| !matches!(instr.kind, InstructionKind::Phi { .. }));
        self.stats.phis_eliminated += original_count - block.instructions.len();
    }
}
```

### Integration Points

#### Pipeline Integration

In `crates/compiler/mir/src/passes.rs`:

```rust
pub mod phi_elimination;
use phi_elimination::PhiElimination;
```

In `PassManager::standard_pipeline()`:

```rust
pub fn standard_pipeline() -> Self {
    Self::new()
        // ... existing SSA optimizations ...
        .add_pass(DeadCodeElimination::new())
        .add_pass(PhiElimination::new())  // NEW: After all SSA optimizations
        .add_pass(Validation::new_post_ssa()) // Already exists, validates non-SSA
}
```

#### Validation Updates

The existing `Validation::new_post_ssa()` already skips SSA invariant checks,
perfect for post-phi-elimination validation.

## Testing Strategy

### Unit Tests

1. **Simple Diamond CFG**: Basic phi elimination
2. **Nested Loops**: Multiple phi nodes with back-edges
3. **Critical Edges**: Verify correct splitting
4. **Parallel Copies**: Test circular dependency handling
5. **Edge Cases**: Empty blocks, single predecessor, no phis

### Integration Tests

1. **End-to-end compilation**: Source → MIR → PhiElimination → CASM
2. **Optimization pipeline**: Verify all passes work correctly
3. **Performance benchmarks**: Measure compilation time impact

### Test File Structure

```
crates/compiler/mir/src/passes/phi_elimination.rs (includes unit tests)
crates/compiler/mir/tests/phi_elimination_tests.rs (integration tests)
```

## Implementation Phases

### Phase 1: Core Implementation (Week 1)

- [ ] Create `phi_elimination.rs` with basic structure
- [ ] Implement critical edge splitting integration
- [ ] Implement basic phi-to-copy transformation
- [ ] Add simple unit tests

### Phase 2: Parallel Copy Handling (Week 1-2)

- [ ] Implement dependency graph construction
- [ ] Add cycle detection (Tarjan's algorithm)
- [ ] Implement temporary variable insertion
- [ ] Test with complex CFGs

### Phase 3: Integration & Polish (Week 2)

- [ ] Integrate into optimization pipeline
- [ ] Add comprehensive integration tests
- [ ] Performance profiling and optimization
- [ ] Documentation and code review

## Performance Considerations

### Time Complexity

- Critical edge splitting: O(E) where E = edges
- Phi collection: O(P) where P = total phi operands
- Cycle detection: O(V + E) per predecessor block
- **Total**: O(N \* M) where N = blocks, M = avg phis per block

### Space Complexity

- Copy instruction storage: O(P)
- Temporary variables for cycles: O(C) where C = concurrent cycles
- **Total**: O(P) auxiliary space

### Optimization Opportunities

1. **Coalescing**: Merge copies of the form `x = x` (no-ops)
2. **Copy propagation**: Run after phi elimination to eliminate redundant copies
3. **Smart temporary reuse**: Reuse temporary variables across different cycles

## Error Handling

### Validation Checks

1. **Pre-conditions**:
   - Function is in valid SSA form
   - All phi nodes have correct structure
2. **Post-conditions**:
   - No phi instructions remain
   - All values still correctly defined
   - CFG structure preserved

### Debug Support

- Detailed logging of transformations
- Statistics reporting (edges split, phis eliminated, copies inserted)
- Visual CFG dumps before/after (when debug flag set)

## References

1. **Sreedhar et al. (1999)**: "Translating Out of Static Single Assignment
   Form"
2. **Cooper & Torczon**: "Engineering a Compiler" (2nd Ed), Section 9.4
3. **Appel**: "Modern Compiler Implementation in ML", Chapter 19
4. **Cytron et al. (1991)**: "Efficiently Computing Static Single Assignment
   Form" (for context)

## Appendix: Example Transformation

### Before Phi Elimination

```mir
bb0:
  %1 = const 10
  jump bb2

bb1:
  %2 = const 20
  jump bb2

bb2:
  %3 = phi [%1: bb0], [%2: bb1]
  return %3
```

### After Phi Elimination

```mir
bb0: %1 = const 10 %3 = %1          // Copy inserted jump bb2

bb1: %2 = const 20 %3 = %2          // Copy inserted jump bb2

bb2: // Phi removed return %3
```

Note: After phi elimination, %3 is assigned in multiple places (non-SSA).

## Next Steps

1. Review this plan with the team
2. Set up the file structure and boilerplate
3. Begin implementation following the phases outlined
4. Regular code reviews at phase boundaries

This implementation will provide a robust, well-tested phi elimination pass that
seamlessly integrates with the existing MIR infrastructure and prepares the code
for final CASM generation.
