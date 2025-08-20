# ArithmeticSimplify Pass Implementation

## Overview

The `ArithmeticSimplify` pass performs peephole rewriting of algebraic and
logical patterns to reduce instruction count and expose further optimization
opportunities.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/arithmetic_simplify.rs`
- **Integration**: Export in `passes/mod.rs`, add to pipelines in `pipeline.rs`

## Algebraic Simplification Rules

### Arithmetic Operations

```rust
// Based on BinaryOp variants from instruction.rs:32-65
match (op, left_val, right_val) {
    // Addition identity
    (BinaryOp::Add | BinaryOp::U32Add, Value::Literal(Literal::Integer(0)), right) =>
        rewrite_to_assign(dest, right),
    (BinaryOp::Add | BinaryOp::U32Add, left, Value::Literal(Literal::Integer(0))) =>
        rewrite_to_assign(dest, left),

    // Subtraction identity
    (BinaryOp::Sub | BinaryOp::U32Sub, left, Value::Literal(Literal::Integer(0))) =>
        rewrite_to_assign(dest, left),

    // Multiplication identities
    (BinaryOp::Mul | BinaryOp::U32Mul, Value::Literal(Literal::Integer(1)), right) =>
        rewrite_to_assign(dest, right),
    (BinaryOp::Mul | BinaryOp::U32Mul, left, Value::Literal(Literal::Integer(1))) =>
        rewrite_to_assign(dest, left),

    // Multiplication by zero
    (BinaryOp::Mul | BinaryOp::U32Mul, Value::Literal(Literal::Integer(0)), _) |
    (BinaryOp::Mul | BinaryOp::U32Mul, _, Value::Literal(Literal::Integer(0))) =>
        rewrite_to_literal(dest, Literal::Integer(0)),

    // Division identity
    (BinaryOp::Div | BinaryOp::U32Div, left, Value::Literal(Literal::Integer(1))) =>
        rewrite_to_assign(dest, left),
}
```

### Comparison Operations

```rust
// Self-comparison (for scalars with well-defined equality)
match (op, left_val, right_val) {
    (BinaryOp::Eq | BinaryOp::U32Eq, Value::Operand(a), Value::Operand(b)) if a == b =>
        rewrite_to_literal(dest, Literal::Boolean(true)),
    (BinaryOp::Neq | BinaryOp::U32Neq, Value::Operand(a), Value::Operand(b)) if a == b =>
        rewrite_to_literal(dest, Literal::Boolean(false)),
}
```

### Boolean Operations

```rust
// Based on existing UnaryOp and BinaryOp patterns
match op {
    // Boolean AND
    BinaryOp::And => match (left_val, right_val) {
        (Value::Literal(Literal::Boolean(true)), right) => rewrite_to_assign(dest, right),
        (left, Value::Literal(Literal::Boolean(true))) => rewrite_to_assign(dest, left),
        (Value::Literal(Literal::Boolean(false)), _) |
        (_, Value::Literal(Literal::Boolean(false))) =>
            rewrite_to_literal(dest, Literal::Boolean(false)),
    },

    // Boolean OR
    BinaryOp::Or => match (left_val, right_val) {
        (Value::Literal(Literal::Boolean(true)), _) |
        (_, Value::Literal(Literal::Boolean(true))) =>
            rewrite_to_literal(dest, Literal::Boolean(true)),
        (Value::Literal(Literal::Boolean(false)), right) => rewrite_to_assign(dest, right),
        (left, Value::Literal(Literal::Boolean(false))) => rewrite_to_assign(dest, left),
    },
}

// Double negation elimination for UnaryOp
if let InstructionKind::UnaryOp { op: UnaryOp::Not, source: Value::Operand(inner_id), .. } = &instr.kind {
    if let Some(defining_instr) = find_defining_instruction(function, *inner_id) {
        if let InstructionKind::UnaryOp { op: UnaryOp::Not, source: inner_source, .. } = &defining_instr.kind {
            // !(!x) → x
            rewrite_to_assign(dest, *inner_source);
        }
    }
}
```

## Implementation Structure

```rust
// Based on existing pass pattern from passes/fuse_cmp.rs:25-47
#[derive(Debug, Default)]
pub struct ArithmeticSimplify;

impl ArithmeticSimplify {
    pub const fn new() -> Self {
        Self
    }

    fn simplify_binary_op(&self, instr: &mut Instruction) -> bool {
        if let InstructionKind::BinaryOp { op, dest, left, right } = &instr.kind {
            // Apply simplification rules
            match self.try_simplify_binary(*op, *left, *right) {
                Some(SimplificationResult::Assign(source)) => {
                    instr.kind = InstructionKind::Assign {
                        dest: *dest,
                        source,
                        ty: op.result_type()
                    };
                    true
                }
                Some(SimplificationResult::Literal(lit)) => {
                    instr.kind = InstructionKind::Assign {
                        dest: *dest,
                        source: Value::Literal(lit),
                        ty: op.result_type()
                    };
                    true
                }
                None => false
            }
        } else {
            false
        }
    }

    fn simplify_unary_op(&self, instr: &mut Instruction, function: &MirFunction) -> bool {
        if let InstructionKind::UnaryOp { op, dest, source } = &instr.kind {
            // Handle double negation elimination
            if matches!(op, UnaryOp::Not) {
                if let Value::Operand(inner_id) = source {
                    // Look for the defining instruction in the same block
                    // (conservative: don't cross block boundaries)
                    return self.try_eliminate_double_negation(*dest, *inner_id, function);
                }
            }
        }
        false
    }
}

enum SimplificationResult {
    Assign(Value),    // Rewrite to assignment from another value
    Literal(Literal), // Rewrite to literal assignment
}

impl MirPass for ArithmeticSimplify {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Iterate through all blocks and instructions
        for block in function.basic_blocks.iter_mut() {
            for instr in &mut block.instructions {
                // Apply simplifications
                if self.simplify_binary_op(instr) || self.simplify_unary_op(instr, function) {
                    modified = true;
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "ArithmeticSimplify"
    }
}
```

## Key API Usage

### Working with Instructions

```rust
// From instruction.rs:225-366 - pattern matching on InstructionKind
match &mut instr.kind {
    InstructionKind::BinaryOp { op, dest, left, right } => {
        // Simplify based on op and operands
    }
    InstructionKind::UnaryOp { op, dest, source } => {
        // Handle unary simplifications
    }
    _ => {} // Skip non-arithmetic instructions
}

// Mutation pattern - change instruction in place
instr.kind = InstructionKind::Assign { dest, source, ty };
```

### Type Handling

```rust
// From instruction.rs:148-172 - using result_type()
let result_ty = op.result_type(); // Returns MirType for the operation result

// For rewritten instructions, preserve the expected result type
InstructionKind::Assign {
    dest,
    source,
    ty: result_ty  // Use the original operation's result type
}
```

## Integration Points

### Pipeline Integration

```rust
// In pipeline.rs - add to pipeline configurations
impl PassManager {
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())  // Add first
            .add_pass(ConstantFolding::new())     // Follow with constant folding
            .add_pass(DeadCodeElimination::new())
            .add_pass(Validation::new_post_ssa())
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod arithmetic_simplify;
pub use arithmetic_simplify::ArithmeticSimplify;

// In passes.rs - update imports
pub use passes::{
    arithmetic_simplify::ArithmeticSimplify,
    dead_code_elimination::DeadCodeElimination,
    fuse_cmp::FuseCmpBranch,
    // ...
};
```

## Test Cases

### Basic Arithmetic

```rust
// Test input MIR
%1 = 42 + 0     // Should become: %1 = 42
%2 = %x * 1     // Should become: %2 = %x
%3 = %y * 0     // Should become: %3 = 0
%4 = %z - 0     // Should become: %4 = %z
```

### Boolean Logic

```rust
// Test input MIR
%1 = %x && true   // Should become: %1 = %x
%2 = %y || false  // Should become: %2 = %y
%3 = %z && false  // Should become: %3 = false
%4 = %w || true   // Should become: %4 = true
```

### Self-Comparison

```rust
// Test input MIR
%1 = %x == %x     // Should become: %1 = true
%2 = %y != %y     // Should become: %2 = false
```

### Double Negation

```rust
// Test input MIR (across instructions in same block)
%1 = !%x
%2 = !%1          // Should become: %2 = %x (eliminate both instructions)
```

## Error Handling

- Conservative approach: skip optimizations when types are unclear
- Preserve instruction semantics exactly
- Handle `MirType::Unknown` gracefully
- No division by zero optimizations (unsafe)
- No `x/x → 1` optimization (unsafe with zero)

## Performance Considerations

- Single pass through all instructions
- No expensive analysis - only local pattern matching
- Immediate instruction mutation (no deferred rewriting)
- Compatible with existing `DeadCodeElimination` to clean up unused instructions

## Integration with Subsequent Passes

- **ConstantFolding**: Benefits from simplified expressions with literals
- **CopyPropagation**: Can eliminate assignments created by this pass
- **SimplifyBranches**: Can fold branches on simplified boolean conditions
- **DeadCodeElimination**: Removes instructions with unused destinations
