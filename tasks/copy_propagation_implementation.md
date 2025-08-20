# CopyPropagation Pass Implementation

## Overview

The `CopyPropagation` pass removes redundant assignments in SSA form by
replacing uses of copied values with their original sources. This is
particularly effective after `ArithmeticSimplify` and `ConstantFolding` which
may create many assignment instructions.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/copy_propagation.rs`
- **Integration**: Export in `passes/mod.rs`, add to pipelines after
  `ConstantFolding`

## Copy Elimination Pattern

### SSA Copy Pattern

In SSA form, a copy instruction has the pattern:

```mir
%dest = assign %source (type)
```

This can be eliminated by:

1. Replacing all uses of `%dest` with `%source`
2. Removing the assignment instruction

### Safety in SSA Form

In SSA, copy elimination is safe when:

- The source value dominates all uses of the destination
- The types match exactly
- The source is not a literal (handled by constant propagation instead)

## Implementation Structure

```rust
use std::collections::HashMap;
use crate::{InstructionKind, MirFunction, Value, ValueId};
use super::MirPass;

#[derive(Debug, Default)]
pub struct CopyPropagation;

impl CopyPropagation {
    pub const fn new() -> Self {
        Self
    }

    /// Collect all copy instructions that can be eliminated
    fn collect_copies(&self, function: &MirFunction) -> HashMap<ValueId, ValueId> {
        let mut copies = HashMap::new();

        for (_block_id, block) in function.basic_blocks() {
            for instr in &block.instructions {
                if let InstructionKind::Assign { dest, source, ty: _ } = &instr.kind {
                    // Only eliminate copies from operands, not literals
                    if let Value::Operand(source_id) = source {
                        // Verify types match (defensive programming)
                        if let (Some(dest_ty), Some(source_ty)) = (
                            function.get_value_type(*dest),
                            function.get_value_type(*source_id)
                        ) {
                            if dest_ty == source_ty {
                                copies.insert(*dest, *source_id);
                            }
                        } else {
                            // If we can't verify types, be conservative and skip
                            continue;
                        }
                    }
                }
            }
        }

        copies
    }

    /// Remove copy instructions that have been propagated
    fn remove_copy_instructions(&self, function: &mut MirFunction, copies: &HashMap<ValueId, ValueId>) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            // Collect indices to remove (in reverse order to avoid index shifts)
            let mut to_remove = Vec::new();

            for (idx, instr) in block.instructions.iter().enumerate() {
                if let InstructionKind::Assign { dest, source, .. } = &instr.kind {
                    if let Value::Operand(_) = source {
                        if copies.contains_key(dest) {
                            to_remove.push(idx);
                        }
                    }
                }
            }

            // Remove instructions in reverse order
            for &idx in to_remove.iter().rev() {
                block.instructions.remove(idx);
                modified = true;
            }
        }

        modified
    }
}

impl MirPass for CopyPropagation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // Collect all copy instructions
        let copies = self.collect_copies(function);

        if copies.is_empty() {
            return false; // No copies to propagate
        }

        let mut modified = false;

        // Replace all uses of copied values
        for (&dest_id, &source_id) in &copies {
            // Use the existing replace_all_uses method from function.rs:386
            function.replace_all_uses(dest_id, source_id);
            modified = true;
        }

        // Remove the now-unused copy instructions
        if self.remove_copy_instructions(function, &copies) {
            modified = true;
        }

        modified
    }

    fn name(&self) -> &'static str {
        "CopyPropagation"
    }
}
```

## Key API Usage

### Using Existing Function API

```rust
// From function.rs:386 - replace all uses of a value
function.replace_all_uses(from_value, to_value);

// This handles:
// - All instruction operands (via instruction.replace_value_uses())
// - Terminator operands (via terminator.replace_value_uses())
// - Function parameters and return values
// - Type information cleanup
```

### Instruction Pattern Matching

```rust
// From instruction.rs:225-234 - matching assignment pattern
match &instr.kind {
    InstructionKind::Assign { dest, source, ty } => {
        // Check if this is a copy (not literal assignment)
        if let Value::Operand(source_id) = source {
            // This is a copy instruction: %dest = %source
            // Can be eliminated by replacing uses of %dest with %source
        }
    }
    _ => {} // Not a copy instruction
}
```

### Type Verification

```rust
// From function.rs - getting value types for safety checks
if let (Some(dest_ty), Some(source_ty)) = (
    function.get_value_type(dest),
    function.get_value_type(source_id)
) {
    if dest_ty == source_ty {
        // Types match - safe to propagate
        copies.insert(dest, source_id);
    }
}
```

## Integration Points

### Pipeline Position

```rust
// In pipeline.rs - add after ConstantFolding
impl PassManager {
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())     // Add here
            .add_pass(SimplifyBranches::new())
            .add_pass(DeadCodeElimination::new())
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod copy_propagation;
pub use copy_propagation::CopyPropagation;

// In passes.rs
pub use passes::{
    arithmetic_simplify::ArithmeticSimplify,
    constant_folding::ConstantFolding,
    copy_propagation::CopyPropagation,
    // ...
};
```

## Test Cases

### Basic Copy Elimination

```rust
// Test input MIR
%1 = 42
%2 = %1         // Copy instruction
%3 = %2 + 1     // Use of copied value

// Expected output MIR
%1 = 42
// %2 = %1 removed
%3 = %1 + 1     // Uses original %1 instead of %2
```

### Multiple Copies

```rust
// Test input MIR
%1 = call some_function()
%2 = %1         // Copy 1
%3 = %2         // Copy 2 (transitively copies %1)
%4 = %3 + %1    // Uses both copied and original

// Expected output MIR
%1 = call some_function()
// %2 = %1 removed
// %3 = %2 removed
%4 = %1 + %1    // Both uses point to original %1
```

### Copy in Control Flow

```rust
// Test input MIR
block1:
    %1 = 42
    %2 = %1     // Copy in one block
    jump block2

block2:
    %3 = %2 + 1 // Use in different block
    return %3

// Expected output MIR (SSA dominance ensures safety)
block1:
    %1 = 42
    // %2 = %1 removed
    jump block2

block2:
    %3 = %1 + 1 // Uses original %1
    return %3
```

### Copies That Should NOT Be Eliminated

```rust
// Literal assignment (let ConstantFolding handle this)
%1 = 42         // NOT eliminated - this is constant propagation

// Type mismatch (hypothetical)
%1 = %x (felt)
%2 = %1 (u32)   // NOT eliminated - different types

// Complex expression assignment
%1 = %x + %y    // NOT eliminated - not a simple copy
```

## Error Handling

### Type Safety

```rust
// Always verify types match before propagating
if dest_ty != source_ty {
    continue; // Skip this copy - type mismatch
}

// Handle unknown types conservatively
if function.get_value_type(dest).is_none() || function.get_value_type(source).is_none() {
    continue; // Skip if types are unknown
}
```

### SSA Invariant Preservation

```rust
// In SSA form, dominance is automatically satisfied for copies
// within the same function, so we don't need explicit dominance checking.
// The copy instruction itself proves that the source dominates the destination.
```

### Collection and Removal Safety

```rust
// Remove instructions in reverse order to avoid index invalidation
let mut to_remove = Vec::new();
for (idx, instr) in block.instructions.iter().enumerate() {
    if should_remove(instr) {
        to_remove.push(idx);
    }
}

// Remove in reverse order
for &idx in to_remove.iter().rev() {
    block.instructions.remove(idx);
}
```

## Performance Considerations

- Two-pass algorithm: collect copies, then apply changes
- Uses efficient `HashMap` for copy tracking
- Leverages existing `replace_all_uses` implementation
- Single traversal for instruction removal

## SSA Form Compatibility

### Why This Works in SSA

1. **Single Definition**: Each value is defined exactly once
2. **Dominance**: In SSA, the copy instruction dominates all uses of its
   destination
3. **Type Consistency**: SSA maintains type information per value
4. **No Aliasing**: Values cannot be modified after definition

### Interaction with Phi Nodes

```rust
// Phi nodes are not affected by copy propagation since they represent
// different values from different control flow paths, not copies
match &instr.kind {
    InstructionKind::Phi { .. } => {
        // Skip phi nodes - these are not copies
    }
    _ => {}
}
```

## Integration with Other Passes

- **ArithmeticSimplify**: Creates assignment instructions that this pass can
  eliminate
- **ConstantFolding**: Creates assignment instructions that this pass can
  eliminate
- **SimplifyBranches**: Benefits from reduced instruction count
- **DeadCodeElimination**: Runs after to clean up any remaining unused values
- **LocalCSE**: May find fewer common expressions due to copy elimination
