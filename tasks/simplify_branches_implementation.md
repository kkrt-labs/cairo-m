# SimplifyBranches Pass Implementation

## Overview

The `SimplifyBranches` pass simplifies control flow by folding conditional
branches with constant conditions and reducing complex branch patterns exposed
by earlier optimization passes.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/simplify_branches.rs`
- **Integration**: Export in `passes/mod.rs`, add to pipelines after
  arithmetic/constant passes

## Branch Simplification Patterns

### Constant Condition Branches

```rust
// Pattern 1: If with literal true condition
if true then jump block_A else jump block_B
// Simplifies to:
jump block_A

// Pattern 2: If with literal false condition
if false then jump block_A else jump block_B
// Simplifies to:
jump block_B

// Pattern 3: BranchCmp with literal operands
if 5 == 3 then jump block_A else jump block_B
// Evaluates comparison and simplifies to:
jump block_B  // since 5 != 3
```

### Terminator Edge Management

Critical requirement: When changing terminators, we must update CFG edges using
the existing infrastructure from `builder/cfg_builder.rs:115-140`.

## Implementation Structure

```rust
use crate::{BasicBlockId, BinaryOp, Literal, MirFunction, Terminator, Value};
use super::MirPass;

#[derive(Debug, Default)]
pub struct SimplifyBranches;

impl SimplifyBranches {
    pub const fn new() -> Self {
        Self
    }

    /// Try to simplify a conditional branch with constant condition
    fn simplify_if_terminator(&self, terminator: &Terminator) -> Option<Terminator> {
        if let Terminator::If { condition, then_target, else_target } = terminator {
            match condition {
                Value::Literal(Literal::Boolean(true)) => {
                    Some(Terminator::jump(*then_target))
                }
                Value::Literal(Literal::Boolean(false)) => {
                    Some(Terminator::jump(*else_target))
                }
                Value::Literal(Literal::Integer(0)) => {
                    // In Cairo-M, 0 is false
                    Some(Terminator::jump(*else_target))
                }
                Value::Literal(Literal::Integer(_)) => {
                    // Non-zero integers are true
                    Some(Terminator::jump(*then_target))
                }
                _ => None, // Cannot simplify - condition is not constant
            }
        } else {
            None
        }
    }

    /// Try to simplify a comparison branch with constant operands
    fn simplify_branch_cmp(&self, terminator: &Terminator) -> Option<Terminator> {
        if let Terminator::BranchCmp { op, left, right, then_target, else_target } = terminator {
            // Only simplify if both operands are literals
            if let (Value::Literal(left_lit), Value::Literal(right_lit)) = (left, right) {
                let result = self.evaluate_comparison(*op, *left_lit, *right_lit)?;

                if result {
                    Some(Terminator::jump(*then_target))
                } else {
                    Some(Terminator::jump(*else_target))
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Evaluate a comparison operation on literal values
    fn evaluate_comparison(&self, op: BinaryOp, left: Literal, right: Literal) -> Option<bool> {
        match (op, left, right) {
            // Integer comparisons
            (BinaryOp::Eq, Literal::Integer(a), Literal::Integer(b)) => Some(a == b),
            (BinaryOp::Neq, Literal::Integer(a), Literal::Integer(b)) => Some(a != b),
            (BinaryOp::Less, Literal::Integer(a), Literal::Integer(b)) => Some(a < b),
            (BinaryOp::Greater, Literal::Integer(a), Literal::Integer(b)) => Some(a > b),
            (BinaryOp::LessEqual, Literal::Integer(a), Literal::Integer(b)) => Some(a <= b),
            (BinaryOp::GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => Some(a >= b),

            // U32 comparisons (treat as unsigned)
            (BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) == (b as u32)),
            (BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) != (b as u32)),
            (BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) < (b as u32)),
            (BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) > (b as u32)),
            (BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) <= (b as u32)),
            (BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b)) =>
                Some((a as u32) >= (b as u32)),

            // Boolean comparisons
            (BinaryOp::Eq, Literal::Boolean(a), Literal::Boolean(b)) => Some(a == b),
            (BinaryOp::Neq, Literal::Boolean(a), Literal::Boolean(b)) => Some(a != b),

            // Boolean logic (if used in branch conditions)
            (BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b)) => Some(a && b),
            (BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b)) => Some(a || b),

            _ => None, // Unsupported or invalid comparison
        }
    }
}

impl MirPass for SimplifyBranches {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Collect block IDs to avoid borrowing issues
        let block_ids: Vec<BasicBlockId> = function.basic_blocks.indices().collect();

        for block_id in block_ids {
            if let Some(block) = function.basic_blocks.get(block_id) {
                let current_terminator = block.terminator.clone();

                // Try to simplify the terminator
                let new_terminator = self.simplify_if_terminator(&current_terminator)
                    .or_else(|| self.simplify_branch_cmp(&current_terminator));

                if let Some(new_term) = new_terminator {
                    // Use the new utility function to update edges properly
                    function.set_terminator_with_edges(block_id, new_term);
                    modified = true;
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "SimplifyBranches"
    }
}
```

## Utility Function Implementation

### Adding to MirFunction

We need to add the utility function to `function.rs` as planned:

```rust
// Add to crates/compiler/mir/src/function.rs
impl MirFunction {
    /// Set terminator while properly maintaining CFG edges
    /// This is a helper for optimization passes that need to change control flow
    pub fn set_terminator_with_edges(&mut self, block_id: BasicBlockId, new_term: Terminator) {
        // Get old target blocks
        let old_targets = if let Some(block) = self.basic_blocks.get(block_id) {
            block.terminator.target_blocks()
        } else {
            return; // Block doesn't exist
        };

        // Disconnect from old targets
        for target in old_targets {
            self.disconnect(block_id, target);
        }

        // Set new terminator
        if let Some(block) = self.basic_blocks.get_mut(block_id) {
            block.set_terminator(new_term.clone());
        }

        // Connect to new targets
        for target in new_term.target_blocks() {
            self.connect(block_id, target);
        }
    }
}
```

## Key API Usage

### Terminator Pattern Matching

```rust
// From terminator.rs:22-53 - matching terminator variants
match &terminator {
    Terminator::If { condition, then_target, else_target } => {
        // Check if condition is a compile-time constant
    }
    Terminator::BranchCmp { op, left, right, then_target, else_target } => {
        // Check if both operands are compile-time constants
    }
    _ => {} // Other terminators don't need simplification
}
```

### Value Literal Extraction

```rust
// From value.rs:91-97 - extracting literals from Value enum
match condition {
    Value::Literal(Literal::Boolean(b)) => {
        // Use boolean value directly
    }
    Value::Literal(Literal::Integer(i)) => {
        // Convert integer to boolean (0 = false, non-zero = true)
    }
    _ => {} // Not a literal - cannot fold
}
```

### Edge Management

```rust
// From terminator.rs:116+ - getting target blocks
let old_targets = terminator.target_blocks(); // Vec<BasicBlockId>

// From function.rs:369-382 - connecting/disconnecting blocks
self.disconnect(pred_id, succ_id);  // Remove edge
self.connect(pred_id, succ_id);     // Add edge

// From basic_block.rs:91 - setting terminator
block.set_terminator(new_terminator);
```

## Integration Points

### Pipeline Position

```rust
// In pipeline.rs - add after arithmetic passes but before DeadCodeElimination
impl PassManager {
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(SimplifyBranches::new())    // Add here
            .add_pass(FuseCmpBranch::new())       // Existing pass
            .add_pass(DeadCodeElimination::new()) // Clean up unreachable blocks
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod simplify_branches;
pub use simplify_branches::SimplifyBranches;

// In passes.rs
pub use passes::{
    // ... other passes
    simplify_branches::SimplifyBranches,
    // ...
};
```

## Test Cases

### Constant Boolean Conditions

```rust
// Test input MIR
block1:
    %1 = 5 > 3          // Constant folding creates: %1 = true
    if %1 then jump block2 else jump block3

// After ConstantFolding + SimplifyBranches:
block1:
    // %1 = true (from constant folding)
    jump block2         // Simplified branch
```

### Constant Comparison Branches

```rust
// Test input MIR
block1:
    if 10 == 5 then jump block2 else jump block3

// Expected output MIR
block1:
    jump block3         // 10 != 5, so take else branch
```

### Integer as Boolean Condition

```rust
// Test input MIR
block1:
    if 0 then jump block2 else jump block3

// Expected output MIR
block1:
    jump block3         // 0 is false in Cairo-M

// Test input MIR
block1:
    if 42 then jump block2 else jump block3

// Expected output MIR
block1:
    jump block2         // Non-zero is true
```

### U32 Comparison Branches

```rust
// Test input MIR
block1:
    if U32Less 3, 7 then jump block2 else jump block3

// Expected output MIR
block1:
    jump block2         // 3 < 7 is true
```

### Complex Boolean Logic

```rust
// Test input MIR (after previous passes)
block1:
    if true && false then jump block2 else jump block3

// Expected output MIR (if ConstantFolding handles &&)
block1:
    jump block3         // true && false = false
```

### Branches That Should NOT Be Simplified

```rust
// Variable conditions (cannot simplify)
block1:
    if %x then jump block2 else jump block3  // Keep unchanged

// Mixed literal/variable comparisons (wait for other passes)
block1:
    if %x == 5 then jump block2 else jump block3  // Keep unchanged
```

## Error Handling

### Type Safety

```rust
// Only handle operations on matching types
match (op, left, right) {
    (BinaryOp::Eq, Literal::Integer(_), Literal::Integer(_)) => { /* safe */ }
    (BinaryOp::Eq, Literal::Boolean(_), Literal::Boolean(_)) => { /* safe */ }
    (BinaryOp::Eq, Literal::Integer(_), Literal::Boolean(_)) => return None, // Type mismatch
    // ...
}
```

### Division by Zero in Comparisons

```rust
// Even in constant comparison context, avoid division by zero
(BinaryOp::Div, _, Literal::Integer(0)) => return None, // Don't evaluate
```

### CFG Consistency

```rust
// Always ensure target blocks exist before creating jumps
let target_exists = function.basic_blocks.get(target_block).is_some();
if !target_exists {
    return None; // Don't create jump to non-existent block
}
```

## Performance Considerations

- Single pass through terminators only (not instructions)
- Immediate evaluation of constant expressions
- Uses existing CFG edge management infrastructure
- Minimal memory allocation (only for collecting block IDs)

## Integration with Other Passes

- **ArithmeticSimplify + ConstantFolding**: Create constant conditions for this
  pass to fold
- **CopyPropagation**: May expose more constant conditions
- **FuseCmpBranch**: Complementary - handles comparison+branch fusion while this
  handles constant folding
- **DeadCodeElimination**: Benefits from simplified control flow to remove
  unreachable blocks

## Interaction with Existing FuseCmpBranch

The existing `FuseCmpBranch` pass and this new `SimplifyBranches` pass are
complementary:

- **FuseCmpBranch**: Combines comparison instructions with branches
- **SimplifyBranches**: Evaluates constant conditions in any branch type

Running order: `SimplifyBranches` → `FuseCmpBranch` → `DeadCodeElimination`
