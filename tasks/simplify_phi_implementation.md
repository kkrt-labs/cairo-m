# SimplifyPhi Pass Implementation

## Overview

The `SimplifyPhi` pass eliminates trivial phi nodes that may become redundant
after other optimization passes. A trivial phi is one where all operands are
identical (except for self-references) or where the phi is only used in
self-referencing cycles.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/simplify_phi.rs`
- **Integration**: Export in `passes/mod.rs`, add to pipelines as final cleanup
  (optional for basic pipeline)

## Trivial Phi Patterns

### Pattern 1: All Operands Identical

```mir
%phi = φ { [block1]: %x, [block2]: %x, [block3]: %x }
```

Can be replaced with `%x` since all incoming values are the same.

### Pattern 2: Only Self-References

```mir
%phi = φ { [block1]: %phi, [block2]: %phi }
```

This creates a cycle with no external input - can often be eliminated.

### Pattern 3: Single Non-Self Operand

```mir
%phi = φ { [block1]: %x, [block2]: %phi, [block3]: %phi }
```

Can be replaced with `%x` since the phi only adds self-references.

## Implementation Structure

```rust
use std::collections::HashSet;
use crate::{BasicBlockId, InstructionKind, MirFunction, Value, ValueId};
use super::MirPass;

#[derive(Debug, Default)]
pub struct SimplifyPhi;

impl SimplifyPhi {
    pub const fn new() -> Self {
        Self
    }

    /// Check if a phi node is trivial and return the replacement value
    fn analyze_phi(&self, phi_operands: &[(BasicBlockId, Value)], phi_dest: ValueId) -> Option<ValueId> {
        if phi_operands.is_empty() {
            return None; // Empty phi should not exist
        }

        let mut unique_operands = HashSet::new();

        // Collect all unique operands (excluding self-references)
        for (_, value) in phi_operands {
            match value {
                Value::Operand(id) if *id != phi_dest => {
                    unique_operands.insert(*id);
                }
                Value::Operand(_) => {
                    // This is a self-reference, ignore it
                }
                Value::Literal(_) => {
                    // Literal operands count as unique (but shouldn't appear in well-formed SSA)
                    return None; // Be conservative with literals
                }
                Value::Error => {
                    return None; // Don't simplify error values
                }
            }
        }

        // Check simplification patterns
        match unique_operands.len() {
            0 => {
                // All operands are self-references - this is a degenerate phi
                // We can't eliminate it safely without more analysis
                None
            }
            1 => {
                // All non-self operands are the same value
                let &replacement = unique_operands.iter().next().unwrap();
                Some(replacement)
            }
            _ => {
                // Multiple different operands - cannot simplify
                None
            }
        }
    }

    /// Find and collect all trivial phis in the function
    fn collect_trivial_phis(&self, function: &MirFunction) -> Vec<(BasicBlockId, usize, ValueId, ValueId)> {
        let mut trivial_phis = Vec::new();

        for (block_id, block) in function.basic_blocks() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                if let InstructionKind::Phi { dest, sources, .. } = &instr.kind {
                    if let Some(replacement) = self.analyze_phi(sources, *dest) {
                        trivial_phis.push((block_id, instr_idx, *dest, replacement));
                    }
                }
            }
        }

        trivial_phis
    }

    /// Remove trivial phi instructions after replacement
    fn remove_phi_instructions(&self, function: &mut MirFunction, trivial_phis: &[(BasicBlockId, usize, ValueId, ValueId)]) {
        // Group by block to handle multiple phis in the same block
        let mut blocks_to_update: std::collections::HashMap<BasicBlockId, Vec<usize>> = std::collections::HashMap::new();

        for &(block_id, instr_idx, _, _) in trivial_phis {
            blocks_to_update.entry(block_id).or_default().push(instr_idx);
        }

        // Remove phi instructions (in reverse order to maintain indices)
        for (block_id, mut indices) in blocks_to_update {
            indices.sort_by_key(|&i| std::cmp::Reverse(i)); // Sort in reverse order

            if let Some(block) = function.basic_blocks.get_mut(block_id) {
                for &idx in &indices {
                    if idx < block.instructions.len() {
                        block.instructions.remove(idx);
                    }
                }
            }
        }
    }
}

impl MirPass for SimplifyPhi {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // Collect trivial phis
        let trivial_phis = self.collect_trivial_phis(function);

        if trivial_phis.is_empty() {
            return false; // No trivial phis found
        }

        let mut modified = false;

        // Replace uses of trivial phi destinations with their replacements
        for &(_, _, dest, replacement) in &trivial_phis {
            function.replace_all_uses(dest, replacement);
            modified = true;
        }

        // Remove the now-unused phi instructions
        self.remove_phi_instructions(function, &trivial_phis);

        modified
    }

    fn name(&self) -> &'static str {
        "SimplifyPhi"
    }
}
```

## Key API Usage

### Phi Instruction Handling

```rust
// From instruction.rs:311-321 - phi instruction structure
match &instr.kind {
    InstructionKind::Phi { dest, ty, sources } => {
        // sources: Vec<(BasicBlockId, Value)>
        // Each entry represents: [block]: value
        for (block_id, value) in sources {
            // Analyze incoming values
        }
    }
    _ => {} // Not a phi
}
```

### Phi Instruction Detection

```rust
// From instruction.rs:1058-1061 - checking if instruction is phi
if instr.is_phi() {
    // This is a phi instruction
    if let Some(operands) = instr.phi_operands() {
        // operands: &[(BasicBlockId, Value)]
    }
}
```

### Value Replacement

```rust
// From function.rs:386 - replace all uses efficiently
function.replace_all_uses(phi_dest, replacement_value);

// This automatically handles:
// - Uses in other instructions
// - Uses in terminators
// - Uses in other phi nodes
// - Parameter/return value updates
// - Type information cleanup
```

## Integration Points

### Pipeline Position

```rust
// In pipeline.rs - add as optional cleanup at the end
impl PassManager {
    pub fn standard_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(LocalCSE::new())
            .add_pass(SimplifyBranches::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(SimplifyPhi::new())        // Add as final cleanup
            .add_pass(DeadCodeElimination::new())
    }

    pub fn aggressive_pipeline() -> Self {
        // Run phi simplification multiple times in aggressive mode
        let mut pm = Self::standard_pipeline();
        pm.add_pass(SimplifyPhi::new()); // Second pass to catch newly exposed trivial phis
        pm
    }

    // Skip in basic pipeline to keep it simple
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(SimplifyBranches::new())
            // SimplifyPhi skipped for basic pipeline
            .add_pass(DeadCodeElimination::new())
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod simplify_phi;
pub use simplify_phi::SimplifyPhi;

// In passes.rs
pub use passes::{
    // ... other passes
    simplify_phi::SimplifyPhi,
    // ...
};
```

## Test Cases

### Trivial Phi with Identical Operands

```rust
// Test input MIR
block1:
    jump block3

block2:
    jump block3

block3:
    %phi = φ { [block1]: %x, [block2]: %x }  // All operands are %x
    %result = %phi + 1
    return %result

// Expected output MIR
block1:
    jump block3

block2:
    jump block3

block3:
    // %phi removed
    %result = %x + 1    // Uses %x directly
    return %result
```

### Phi with Self-References

```rust
// Test input MIR
block1:
    %initial = 42
    jump block2

block2:
    %phi = φ { [block1]: %initial, [block2]: %phi }  // Self-reference from block2
    %cond = %phi < 100
    if %cond then jump block2 else jump block3

block3:
    return %phi

// Expected output MIR
block1:
    %initial = 42
    jump block2

block2:
    // %phi removed - replaced with %initial
    %cond = %initial < 100
    if %cond then jump block2 else jump block3

block3:
    return %initial    // Uses %initial directly
```

### Complex Phi with Mixed Self-References

```rust
// Test input MIR
block1:
    jump block3

block2:
    jump block3

block3:
    %phi = φ { [block1]: %y, [block2]: %phi }  // %y from block1, self from block2
    %result = %phi * 2

// Expected output MIR
block1:
    jump block3

block2:
    jump block3

block3:
    // %phi removed - all non-self operands are %y
    %result = %y * 2   // Uses %y directly
```

### Phi That Should NOT Be Simplified

```rust
// Test input MIR - different operands
block1:
    jump block3

block2:
    jump block3

block3:
    %phi = φ { [block1]: %x, [block2]: %y }  // Different operands
    return %phi

// Expected output MIR (unchanged)
block1:
    jump block3

block2:
    jump block3

block3:
    %phi = φ { [block1]: %x, [block2]: %y }  // Kept - not trivial
    return %phi
```

## Error Handling

### Conservative Approach

```rust
// Skip phis with literal operands (shouldn't exist in well-formed SSA)
if matches!(value, Value::Literal(_)) {
    return None; // Be conservative
}

// Skip phis with error values
if matches!(value, Value::Error) {
    return None; // Don't propagate errors
}
```

### Degenerate Phi Handling

```rust
// Handle phis with only self-references carefully
if unique_operands.is_empty() {
    // All operands are self-references
    // This creates a cycle with no external input
    // Conservative: don't eliminate without deeper analysis
    return None;
}
```

### Block Structure Validation

```rust
// Ensure phi operands reference valid blocks
for (block_id, _) in phi_operands {
    if !function.basic_blocks.get(*block_id).is_some() {
        return None; // Invalid block reference
    }
}
```

## Performance Considerations

- **Two-Pass Algorithm**: Collect first, then modify (avoids borrow conflicts)
- **Hash Set for Operand Tracking**: Efficient duplicate detection
- **Batch Removal**: Remove instructions in reverse order to avoid index shifts
- **Leverages Existing API**: Uses `replace_all_uses` for efficiency

## SSA Form Considerations

### When Phi Simplification is Safe

1. **SSA Invariants**: Each value is defined exactly once
2. **Dominance**: Replacement value must dominate all uses of the phi
3. **Type Consistency**: Replacement must have same type as phi

### Interaction with SSA Construction

This pass runs after SSA construction is complete, so it operates on
fully-formed SSA. The existing SSA construction in the function may have already
eliminated some trivial phis, but optimization passes can create new
opportunities.

## Integration with Other Passes

- **ArithmeticSimplify**: May create assignments that eliminate phi operands
- **ConstantFolding**: May create constant operands (handled conservatively)
- **CopyPropagation**: May unify phi operands by eliminating intermediate copies
- **LocalCSE**: May create common expressions that reduce phi operand diversity
- **SimplifyBranches**: May eliminate control flow that creates trivial merge
  points
- **DeadCodeElimination**: Benefits from phi elimination to remove unused values

## Iterative Application

In aggressive optimization mode, this pass may benefit from being run multiple
times, as eliminating one phi might make others trivial:

```rust
// Example of cascading phi elimination
// Before first pass:
%phi1 = φ { [block1]: %x, [block2]: %x }     // Trivial
%phi2 = φ { [block1]: %phi1, [block2]: %y }  // Not trivial yet

// After first pass:
// %phi1 eliminated, replaced with %x
%phi2 = φ { [block1]: %x, [block2]: %y }     // Still not trivial

// But if %y gets propagated by other passes:
%phi2 = φ { [block1]: %x, [block2]: %x }     // Now trivial for second pass
```
