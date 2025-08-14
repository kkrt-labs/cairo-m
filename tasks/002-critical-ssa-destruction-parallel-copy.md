# Task: Fix SSA Destruction Parallel Copy Semantics

## Priority

CRITICAL

## Status

✅ COMPLETED

## Why

The current SSA destruction pass in
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/ssa_destruction.rs`
has a fundamental semantic correctness issue that can lead to wrong program
execution.

When eliminating phi nodes, the current implementation inserts sequential
assignments without proper parallel copy semantics. This creates a critical bug
when multiple phi nodes have overlapping sources and destinations, potentially
creating copy cycles where values get overwritten before they can be read.

**Example problematic scenario:**

```
%a = phi [%x, block1], [%y, block2]
%b = phi [%y, block1], [%x, block2]
```

Current implementation generates:

```
// In predecessor block - WRONG!
%a = %x  // or %y depending on predecessor
%b = %y  // or %x - but %x might already be overwritten!
```

This violates the parallel copy semantics required by phi elimination and can
produce incorrect results.

## What

The core issue is in the `eliminate_phi_nodes` function (lines 51-124). The
current approach:

1. **Sequential Assignment Problem**: Inserts assignments one by one without
   considering dependencies between multiple phi nodes that share
   source/destination variables
2. **Missing Parallel Copy Algorithm**: Lacks the standard parallel copy
   algorithm needed to handle copy cycles and overlapping assignments
3. **No Temporary Generation**: Doesn't generate temporaries to break cycles
   when needed

**Current problematic code structure (lines 84-103):**

```rust
for (pred_block_id, succ_block_id, dest, value, ty) in assignments {
    // Insert assignment directly - no consideration of other phi nodes!
    let assign_inst = Instruction::assign(dest, value, ty);
    insert_block.instructions.push(assign_inst);
}
```

The fix requires implementing a proper parallel copy algorithm that:

1. Detects copy cycles between phi assignments in the same block/edge
2. Introduces temporaries to break cycles
3. Ensures all assignments execute with proper parallel semantics

## How

### Algorithm Implementation

Replace the current linear assignment approach with a proper parallel copy
algorithm. The implementation should be located in the `eliminate_phi_nodes`
function starting around line 84.

### 1. Data Structure Changes

Modify the data collection phase (lines 58-81) to group phi assignments by
insertion location:

```rust
// Group assignments by where they need to be inserted
let mut insertion_groups: HashMap<BasicBlockId, Vec<(ValueId, Value)>> = HashMap::new();

for (phi_block_id, phi_inst_idx, assignments) in phi_replacements {
    for (pred_block_id, succ_block_id, dest, value, ty) in assignments {
        let insert_block_id = /* critical edge logic as before */;
        insertion_groups
            .entry(insert_block_id)
            .or_default()
            .push((dest, value));
    }
}
```

### 2. Parallel Copy Algorithm

For each insertion location, implement the parallel copy algorithm:

```rust
fn generate_parallel_copy(
    function: &mut MirFunction,
    insert_block_id: BasicBlockId,
    copies: Vec<(ValueId, Value)>,
) {
    // 1. Build dependency graph
    let mut graph = ParallelCopyGraph::new();
    for (dest, src) in &copies {
        graph.add_copy(*dest, *src);
    }

    // 2. Detect cycles using DFS
    let cycles = graph.find_cycles();

    // 3. Break cycles with temporaries
    let mut temp_assignments = Vec::new();
    for cycle in cycles {
        let temp = function.new_typed_value_id(/* get type from cycle */);
        // temp = first_element_of_cycle
        temp_assignments.push((temp, cycle[0].source));
        // Modify cycle to use temporary
        cycle[0].source = Value::operand(temp);
    }

    // 4. Topological sort for remaining dependencies
    let sorted_copies = graph.topological_sort();

    // 5. Generate assignments in correct order
    let insert_block = function.basic_blocks.get_mut(insert_block_id).unwrap();

    // Insert temporary assignments first
    for (temp_dest, temp_src) in temp_assignments {
        insert_block.instructions.push(
            Instruction::assign(temp_dest, temp_src, /* appropriate type */)
        );
    }

    // Insert main assignments in dependency order
    for (dest, src) in sorted_copies {
        insert_block.instructions.push(
            Instruction::assign(dest, src, /* appropriate type */)
        );
    }
}
```

### 3. Supporting Data Structures

Add to `ssa_destruction.rs`:

```rust
struct ParallelCopyGraph {
    copies: Vec<(ValueId, Value)>,
    dependencies: HashMap<ValueId, Vec<ValueId>>,
}

impl ParallelCopyGraph {
    fn add_copy(&mut self, dest: ValueId, src: Value) { /* ... */ }
    fn find_cycles(&self) -> Vec<Vec<CopyEdge>> { /* DFS cycle detection */ }
    fn topological_sort(&self) -> Vec<(ValueId, Value)> { /* Kahn's algorithm */ }
}

struct CopyEdge {
    dest: ValueId,
    source: Value,
}
```

### 4. Integration Points

- **Line 84**: Replace the simple assignment loop with calls to
  `generate_parallel_copy`
- **Lines 52-57**: Import CFG utilities and add dependency graph structures
- **Lines 98-103**: Replace direct instruction insertion with parallel copy
  generation

### 5. Type Handling

The current code properly preserves types through the `ty` field in phi
instructions. The parallel copy implementation must:

- Pass type information through the dependency graph
- Use appropriate types when creating temporary variables
- Maintain type correctness in generated assignments

## Testing

### Test Cases to Add to `ssa_destruction_tests.rs`:

1. **Copy Cycle Test**:

```rust
#[test]
fn test_phi_copy_cycle() {
    // %a = phi [%x, pred], [%y, pred2]
    // %b = phi [%y, pred], [%x, pred2]
    // Should generate temporaries to break cycle
}
```

2. **Complex Dependency Chain**:

```rust
#[test]
fn test_phi_dependency_chain() {
    // %a = phi [%x, pred]
    // %b = phi [%a, pred]  // depends on %a
    // %c = phi [%b, pred]  // depends on %b
    // Should respect dependency order
}
```

3. **Self-Loop Test**:

```rust
#[test]
fn test_phi_self_reference() {
    // %a = phi [%a, loop_back], [%x, entry]
    // Should handle self-references correctly
}
```

4. **Mixed Cycle and Chain**:

```rust
#[test]
fn test_phi_mixed_dependencies() {
    // Complex mix of cycles and chains in same block
}
```

### Verification Strategy:

- Add property-based tests using `proptest` to generate random phi
  configurations
- Verify that generated code preserves original semantics by execution
  comparison
- Add integration tests that compile Cairo-M programs with complex control flow
- Test critical edge cases where cycles span across edge-split blocks

## Impact

### Correctness Guarantees

- **Eliminates wrong-answer bugs**: Fixes potential silent correctness issues in
  programs with complex control flow
- **Preserves program semantics**: Ensures phi elimination maintains the
  parallel copy semantics required by SSA form
- **Maintains type safety**: Proper temporary generation preserves MIR type
  invariants

### Performance Implications

- **Minimal runtime overhead**: Only affects compilation time, not generated
  code performance
- **Potential temporary generation**: May create additional temporary variables,
  but only when necessary to break cycles
- **Improved optimization opportunities**: Correct SSA destruction enables
  better subsequent optimizations

### Code Quality

- **Algorithmic correctness**: Implements the standard, well-studied parallel
  copy algorithm from compiler literature
- **Maintainability**: Clear separation of concerns with dependency graph
  abstraction
- **Extensibility**: Framework can be extended for other parallel assignment
  scenarios

The fix is critical for ensuring Cairo-M's compiler correctness and should be
implemented before any production use.

## Implementation Summary

### Changes Made

- Implemented proper parallel copy algorithm in `ssa_destruction.rs`
- Added `ParallelCopyGraph` struct to manage copy dependencies
- Implemented cycle detection using DFS to identify copy cycles
- Added temporary generation to break cycles when needed
- Implemented topological sort (Kahn's algorithm) for correct assignment
  ordering
- Modified phi elimination to group assignments by insertion block for parallel
  processing

### Algorithm Components

1. **CopyOperation struct**: Represents individual copy operations with type
   information
2. **ParallelCopyGraph**: Manages the dependency graph between copy operations
3. **Cycle detection**: DFS-based algorithm to find cycles in the dependency
   graph
4. **Temporary generation**: Breaks cycles by saving values to temporaries first
5. **Topological sort**: Ensures assignments execute in correct dependency order

### Testing Results

- ✅ All existing SSA destruction tests pass
- ✅ New test `test_phi_copy_cycle` verifies correct handling of overlapping phi
  assignments
- ✅ New test `test_phi_dependency_chain` verifies correct handling of dependent
  phi nodes
- ✅ Full MIR test suite passes (52 tests)

### Impact

The implementation ensures:

- **Correctness**: Phi elimination now preserves parallel copy semantics
- **Robustness**: Handles complex control flow patterns including loops and
  critical edges
- **Type safety**: Maintains type information through temporary generation
- **Performance**: Only introduces temporaries when necessary to break cycles
