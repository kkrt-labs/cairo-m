# ConstantFolding Pass Implementation

## Overview

The `ConstantFolding` pass evaluates operations when all operands are
compile-time literals, replacing the instruction with a direct assignment to the
computed result.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/constant_folding.rs`
- **Integration**: Export in `passes/mod.rs`, add to pipelines after
  `ArithmeticSimplify`

## Foldable Operations

### Binary Operations

Based on `BinaryOp` variants from `instruction.rs:32-65`:

```rust
fn try_fold_binary_op(op: BinaryOp, left: Literal, right: Literal) -> Option<Literal> {
    match (op, left, right) {
        // Felt arithmetic
        (BinaryOp::Add, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(a.saturating_add(b))),
        (BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(a.saturating_sub(b))),
        (BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(a.saturating_mul(b))),
        (BinaryOp::Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 =>
            Some(Literal::Integer(a / b)),

        // Felt comparisons
        (BinaryOp::Eq, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a == b)),
        (BinaryOp::Neq, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a != b)),
        (BinaryOp::Less, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a < b)),
        (BinaryOp::Greater, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a > b)),
        (BinaryOp::LessEqual, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a <= b)),
        (BinaryOp::GreaterEqual, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean(a >= b)),

        // U32 arithmetic (with proper wrapping)
        (BinaryOp::U32Add, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(((a as u32).wrapping_add(b as u32)) as i32)),
        (BinaryOp::U32Sub, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(((a as u32).wrapping_sub(b as u32)) as i32)),
        (BinaryOp::U32Mul, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Integer(((a as u32).wrapping_mul(b as u32)) as i32)),
        (BinaryOp::U32Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 =>
            Some(Literal::Integer(((a as u32) / (b as u32)) as i32)),

        // U32 comparisons
        (BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) == (b as u32))),
        (BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) != (b as u32))),
        (BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) < (b as u32))),
        (BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) > (b as u32))),
        (BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) <= (b as u32))),
        (BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b)) =>
            Some(Literal::Boolean((a as u32) >= (b as u32))),

        // Boolean operations
        (BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b)) =>
            Some(Literal::Boolean(a && b)),
        (BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b)) =>
            Some(Literal::Boolean(a || b)),

        _ => None, // Cannot fold or unsafe to fold
    }
}
```

### Unary Operations

Based on `UnaryOp` from parser (imported in `instruction.rs:8`):

```rust
fn try_fold_unary_op(op: UnaryOp, operand: Literal) -> Option<Literal> {
    match (op, operand) {
        (UnaryOp::Not, Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
        (UnaryOp::Neg, Literal::Integer(i)) => Some(Literal::Integer(-i)),
        _ => None,
    }
}
```

## Implementation Structure

```rust
use crate::{InstructionKind, Literal, MirFunction, Value};
use super::MirPass;

#[derive(Debug, Default)]
pub struct ConstantFolding;

impl ConstantFolding {
    pub const fn new() -> Self {
        Self
    }

    fn try_fold_instruction(&self, instr: &mut Instruction) -> bool {
        match &instr.kind {
            InstructionKind::BinaryOp { op, dest, left, right } => {
                if let (Value::Literal(left_lit), Value::Literal(right_lit)) = (left, right) {
                    if let Some(result) = self.try_fold_binary_op(*op, *left_lit, *right_lit) {
                        // Replace with assignment to folded result
                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: Value::Literal(result),
                            ty: op.result_type(),
                        };
                        return true;
                    }
                }
            }

            InstructionKind::UnaryOp { op, dest, source } => {
                if let Value::Literal(source_lit) = source {
                    if let Some(result) = self.try_fold_unary_op(*op, *source_lit) {
                        // Determine result type based on operation
                        let result_ty = match op {
                            UnaryOp::Not => MirType::bool(),
                            UnaryOp::Neg => MirType::felt(), // Assuming negation on felt
                            _ => return false, // Unknown operation
                        };

                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: Value::Literal(result),
                            ty: result_ty,
                        };
                        return true;
                    }
                }
            }

            // Optional: Fold tuple operations on compile-time tuples
            InstructionKind::ExtractTupleElement { dest, tuple, index, element_ty } => {
                // Conservative: only fold if tuple was created in same block
                // and all elements are literals
                return self.try_fold_tuple_extract(*dest, tuple, *index, element_ty);
            }

            _ => {}
        }

        false
    }

    fn try_fold_tuple_extract(&self, dest: ValueId, tuple: &Value, index: usize, element_ty: &MirType) -> bool {
        // Implementation would need access to the defining instruction
        // Skip for initial implementation to keep it simple
        false
    }
}

impl MirPass for ConstantFolding {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            for instr in &mut block.instructions {
                if self.try_fold_instruction(instr) {
                    modified = true;
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "ConstantFolding"
    }
}
```

## Key API Usage

### Value Pattern Matching

```rust
// From value.rs:19-32 - extracting literals from Value enum
match (left, right) {
    (Value::Literal(left_lit), Value::Literal(right_lit)) => {
        // Both operands are compile-time constants
        // Can safely evaluate the operation
    }
    _ => {
        // At least one operand is computed at runtime
        // Cannot fold this operation
    }
}
```

### Type Management

```rust
// From instruction.rs:148-172 - getting result type for operations
let result_ty = op.result_type(); // BinaryOp has const fn result_type()

// For assignments, use proper MirType
// From mir_types.rs (referenced in instruction.rs)
InstructionKind::Assign {
    dest,
    source: Value::Literal(computed_result),
    ty: result_ty, // Preserve the operation's expected result type
}
```

### Safe Arithmetic

```rust
// Handle potential overflow/underflow safely
fn safe_add(a: i32, b: i32) -> i32 {
    a.saturating_add(b) // Saturate instead of panicking on overflow
}

fn safe_div(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        None // Never divide by zero
    } else {
        Some(a / b)
    }
}

// For U32 operations, use proper wrapping semantics
fn u32_add(a: i32, b: i32) -> i32 {
    ((a as u32).wrapping_add(b as u32)) as i32
}
```

## Integration Points

### Pipeline Position

```rust
// In pipeline.rs - add after ArithmeticSimplify
impl PassManager {
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())    // Add here
            .add_pass(CopyPropagation::new())
            .add_pass(SimplifyBranches::new())
            .add_pass(DeadCodeElimination::new())
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod constant_folding;
pub use constant_folding::ConstantFolding;

// In passes.rs
pub use passes::{
    arithmetic_simplify::ArithmeticSimplify,
    constant_folding::ConstantFolding,
    // ...
};
```

## Test Cases

### Basic Arithmetic Folding

```rust
// Test input MIR
%1 = 3 + 4       // Should become: %1 = 7
%2 = 10 - 6      // Should become: %2 = 4
%3 = 5 * 3       // Should become: %3 = 15
%4 = 12 / 4      // Should become: %4 = 3
```

### Comparison Folding

```rust
// Test input MIR
%1 = 5 == 5      // Should become: %1 = true
%2 = 3 != 3      // Should become: %2 = false
%3 = 7 > 4       // Should become: %3 = true
%4 = 2 <= 8      // Should become: %4 = true
```

### Boolean Folding

```rust
// Test input MIR
%1 = true && false   // Should become: %1 = false
%2 = true || false   // Should become: %2 = true
%3 = !true           // Should become: %3 = false
```

### U32 Operations

```rust
// Test input MIR with U32 arithmetic
%1 = U32Add 4294967295, 1  // Should become: %1 = 0 (wrapping)
%2 = U32Div 10, 3          // Should become: %2 = 3 (integer division)
```

### Mixed Operations (Should Not Fold)

```rust
// Test input MIR
%1 = %x + 5      // Should NOT fold (operand not literal)
%2 = 8 / 0       // Should NOT fold (division by zero)
%3 = %y && true  // Should NOT fold (wait for ArithmeticSimplify)
```

## Error Handling

### Division by Zero

```rust
// Never fold division by zero
(BinaryOp::Div | BinaryOp::U32Div, _, Literal::Integer(0)) => None,
```

### Overflow Handling

```rust
// Use saturating arithmetic for felt operations
let result = a.saturating_add(b);

// Use wrapping arithmetic for U32 operations
let result = (a as u32).wrapping_add(b as u32) as i32;
```

### Type Safety

```rust
// Only fold when operand types match expected operation types
// Conservative: skip folding if types are unclear
match (op, left_ty, right_ty) {
    (BinaryOp::Add, MirType::Felt, MirType::Felt) => { /* can fold */ }
    (BinaryOp::U32Add, MirType::U32, MirType::U32) => { /* can fold */ }
    _ => return None, // Type mismatch or unknown types
}
```

## Performance Considerations

- Single pass through all instructions
- Immediate folding with no deferred computation
- No expensive arithmetic (all operations are O(1))
- Conservative approach: skip when uncertain

## Integration with Other Passes

- **ArithmeticSimplify**: Runs first to expose more folding opportunities
- **CopyPropagation**: Runs after to eliminate assignments to folded constants
- **SimplifyBranches**: Benefits from folded boolean conditions
- **LocalCSE**: May find fewer redundant expressions due to constant folding
