# LocalCSE (Local Value Numbering) Pass Implementation

## Overview

The `LocalCSE` pass implements per-basic-block common subexpression elimination
for pure expressions. It builds on the value numbering concept from
`tasks/06_pure_expression_cse.md` but operates as a post-SSA optimization pass
rather than during SSA construction.

## Implementation Location

- **File**: `crates/compiler/mir/src/passes/local_cse.rs`
- **Integration**: Export in `passes/mod.rs`, add to standard/aggressive
  pipelines before `SimplifyBranches`

## Value Numbering Key Design

### PureExpressionKey

Based on the existing pattern from Task 6, but adapted for post-SSA
optimization:

```rust
use rustc_hash::FxHashMap;
use crate::{BasicBlockId, BinaryOp, UnaryOp, MirType, Value, ValueId, Instruction, InstructionKind};

/// A key representing a pure expression for memoization within a basic block
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PureExpressionKey {
    /// Binary operation: (op, left, right, result_type)
    Binary {
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
        result_type: MirType,
    },

    /// Unary operation: (op, operand, result_type)
    Unary {
        op: UnaryOp,
        operand: ValueId,
        result_type: MirType,
    },

    /// Tuple extraction: (tuple, index, element_type)
    ExtractTuple {
        tuple: ValueId,
        index: usize,
        element_type: MirType,
    },

    /// Struct field extraction: (struct, field_name, field_type)
    ExtractField {
        struct_val: ValueId,
        field_name: String,
        field_type: MirType,
    },

    /// Tuple creation: (elements, tuple_type)
    MakeTuple {
        elements: Vec<ValueId>,
        tuple_type: MirType,
    },

    /// Struct creation: (fields, struct_type)
    MakeStruct {
        fields: Vec<(String, ValueId)>,
        struct_type: MirType,
    },
}

impl PureExpressionKey {
    /// Try to create a PureExpressionKey from an instruction
    /// Returns None if the instruction has side effects or uses literals
    pub fn from_instruction(instr: &Instruction) -> Option<Self> {
        // Only consider pure instructions
        if !instr.is_pure() {
            return None;
        }

        match &instr.kind {
            InstructionKind::BinaryOp { op, left, right, .. } => {
                // Only handle operand-operand operations (not mixed literal/operand)
                if let (Value::Operand(left_id), Value::Operand(right_id)) = (left, right) {
                    Some(PureExpressionKey::Binary {
                        op: *op,
                        left: *left_id,
                        right: *right_id,
                        result_type: op.result_type(),
                    })
                } else {
                    None // Skip mixed literal/operand for simplicity
                }
            }

            InstructionKind::UnaryOp { op, source, .. } => {
                if let Value::Operand(operand_id) = source {
                    // Determine result type based on operation
                    let result_type = match op {
                        UnaryOp::Not => MirType::bool(),
                        UnaryOp::Neg => MirType::felt(),
                        _ => return None, // Unknown operation
                    };

                    Some(PureExpressionKey::Unary {
                        op: *op,
                        operand: *operand_id,
                        result_type,
                    })
                } else {
                    None // Literal operand handled by constant folding
                }
            }

            InstructionKind::ExtractTupleElement { tuple, index, element_ty, .. } => {
                if let Value::Operand(tuple_id) = tuple {
                    Some(PureExpressionKey::ExtractTuple {
                        tuple: *tuple_id,
                        index: *index,
                        element_type: element_ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::ExtractStructField { struct_val, field_name, field_ty, .. } => {
                if let Value::Operand(struct_id) = struct_val {
                    Some(PureExpressionKey::ExtractField {
                        struct_val: *struct_id,
                        field_name: field_name.clone(),
                        field_type: field_ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::MakeTuple { elements, .. } => {
                // Only handle all-operand tuples
                let element_ids: Option<Vec<ValueId>> = elements.iter()
                    .map(|v| match v {
                        Value::Operand(id) => Some(*id),
                        _ => None,
                    })
                    .collect();

                if let Some(ids) = element_ids {
                    // Reconstruct tuple type from elements
                    // This is simplified - in practice might need function context
                    Some(PureExpressionKey::MakeTuple {
                        elements: ids,
                        tuple_type: MirType::Unknown, // Simplified for now
                    })
                } else {
                    None
                }
            }

            InstructionKind::MakeStruct { fields, struct_ty, .. } => {
                // Only handle all-operand structs
                let field_ids: Option<Vec<(String, ValueId)>> = fields.iter()
                    .map(|(name, v)| match v {
                        Value::Operand(id) => Some((name.clone(), *id)),
                        _ => None,
                    })
                    .collect();

                if let Some(ids) = field_ids {
                    Some(PureExpressionKey::MakeStruct {
                        fields: ids,
                        struct_type: struct_ty.clone(),
                    })
                } else {
                    None
                }
            }

            // Skip instructions with side effects or not supported
            InstructionKind::Call { .. } |
            InstructionKind::Store { .. } |
            InstructionKind::Load { .. } |
            InstructionKind::FrameAlloc { .. } |
            InstructionKind::GetElementPtr { .. } |
            InstructionKind::Assign { .. } |
            InstructionKind::Debug { .. } |
            InstructionKind::Phi { .. } |
            InstructionKind::Nop => None,

            // Aggregate modification operations - skip for conservatism
            InstructionKind::InsertField { .. } |
            InstructionKind::InsertTuple { .. } => None,

            // Cast operations - skip for now
            InstructionKind::Cast { .. } |
            InstructionKind::AddressOf { .. } => None,
        }
    }
}
```

## Implementation Structure

```rust
use std::collections::HashMap;
use crate::{MirFunction, ValueId};
use super::MirPass;

#[derive(Debug, Default)]
pub struct LocalCSE;

impl LocalCSE {
    pub const fn new() -> Self {
        Self
    }

    /// Perform local value numbering within a single basic block
    fn process_block(&self, function: &mut MirFunction, block_id: BasicBlockId) -> bool {
        let mut modified = false;
        let mut value_table: FxHashMap<PureExpressionKey, ValueId> = FxHashMap::default();

        // We need to collect replacements first, then apply them
        // to avoid borrowing issues during iteration
        let mut replacements = Vec::new();

        if let Some(block) = function.basic_blocks.get(block_id) {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                if let Some(key) = PureExpressionKey::from_instruction(instr) {
                    if let Some(&existing_value) = value_table.get(&key) {
                        // Found a common subexpression!
                        if let Some(dest) = instr.destination() {
                            replacements.push((dest, existing_value, instr_idx));
                        }
                    } else {
                        // First occurrence - record it
                        if let Some(dest) = instr.destination() {
                            value_table.insert(key, dest);
                        }
                    }
                }
            }
        }

        // Apply replacements
        for (dest, existing_value, instr_idx) in replacements {
            // Replace all uses of dest with existing_value
            function.replace_all_uses(dest, existing_value);

            // Mark instruction for removal (we'll do this in a separate pass)
            // For now, just mark that we modified something
            modified = true;
        }

        // Remove redundant instructions (need to do this carefully)
        if modified {
            self.remove_redundant_instructions(function, block_id);
        }

        modified
    }

    /// Remove instructions that compute values we've already replaced
    fn remove_redundant_instructions(&self, function: &mut MirFunction, block_id: BasicBlockId) {
        if let Some(block) = function.basic_blocks.get_mut(block_id) {
            let use_counts = function.get_value_use_counts();

            // Remove instructions whose destinations are no longer used
            block.instructions.retain(|instr| {
                if let Some(dest) = instr.destination() {
                    // Keep instruction if its result is still used
                    use_counts.get(&dest).copied().unwrap_or(0) > 0
                } else {
                    // Keep instructions without destinations (side effects)
                    true
                }
            });
        }
    }
}

impl MirPass for LocalCSE {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Process each basic block independently
        let block_ids: Vec<_> = function.basic_blocks.indices().collect();
        for block_id in block_ids {
            if self.process_block(function, block_id) {
                modified = true;
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "LocalCSE"
    }
}
```

## Key API Usage

### Pure Instruction Detection

```rust
// From instruction.rs:1039-1041 - checking if instruction is pure
if !instr.is_pure() {
    return None; // Skip instructions with side effects
}

// is_pure() returns true for:
// - BinaryOp, UnaryOp (arithmetic/logical operations)
// - ExtractTupleElement, ExtractStructField (read-only access)
// - MakeTuple, MakeStruct (value construction)
//
// is_pure() returns false for:
// - Call, Store, FrameAlloc, Debug (side effects)
// - Load (conservative - memory could be modified)
```

### Use Count Analysis

```rust
// From function.rs - getting use counts for dead code elimination
let use_counts = function.get_value_use_counts();

// Remove instructions whose results are no longer used
block.instructions.retain(|instr| {
    if let Some(dest) = instr.destination() {
        use_counts.get(&dest).copied().unwrap_or(0) > 0
    } else {
        true // Keep side-effect instructions
    }
});
```

### Value Replacement

```rust
// From function.rs:386 - replace all uses efficiently
function.replace_all_uses(redundant_value, canonical_value);

// This updates:
// - All instruction operands
// - Terminator operands
// - Parameter/return value lists
// - Type information
```

## Integration Points

### Pipeline Position

```rust
// In pipeline.rs - add to standard/aggressive pipelines
impl PassManager {
    pub fn standard_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            .add_pass(LocalCSE::new())           // Add here
            .add_pass(SimplifyBranches::new())
            .add_pass(FuseCmpBranch::new())
            .add_pass(DeadCodeElimination::new())
    }

    // Skip in basic pipeline to keep it fast
    pub fn basic_pipeline() -> Self {
        Self::new()
            .add_pass(ArithmeticSimplify::new())
            .add_pass(ConstantFolding::new())
            .add_pass(CopyPropagation::new())
            // LocalCSE skipped for basic pipeline
            .add_pass(SimplifyBranches::new())
            .add_pass(DeadCodeElimination::new())
    }
}
```

### Module Export

```rust
// In passes/mod.rs
pub mod local_cse;
pub use local_cse::LocalCSE;

// In passes.rs
pub use passes::{
    arithmetic_simplify::ArithmeticSimplify,
    constant_folding::ConstantFolding,
    copy_propagation::CopyPropagation,
    local_cse::LocalCSE,
    // ...
};
```

## Test Cases

### Basic Common Subexpression

```rust
// Test input MIR
block1:
    %1 = %x + %y
    %2 = %z * 2
    %3 = %x + %y     // Same as %1
    %4 = %1 + %3     // Uses both original and duplicate

// Expected output MIR
block1:
    %1 = %x + %y
    %2 = %z * 2
    // %3 = %x + %y removed
    %4 = %1 + %1     // Uses %1 twice
```

### Complex Expression CSE

```rust
// Test input MIR
block1:
    %1 = extracttuple %t, 0
    %2 = %1 * 5
    %3 = extracttuple %t, 0   // Same as %1
    %4 = %3 * 5               // Same as %2
    return %2 + %4

// Expected output MIR
block1:
    %1 = extracttuple %t, 0
    %2 = %1 * 5
    // %3 removed
    // %4 removed
    return %2 + %2            // Uses %2 twice
```

### Struct Field CSE

```rust
// Test input MIR
block1:
    %1 = extractfield %s, "x"
    %2 = extractfield %s, "y"
    %3 = extractfield %s, "x"   // Same as %1
    %4 = %1 + %3                // Uses duplicate

// Expected output MIR
block1:
    %1 = extractfield %s, "x"
    %2 = extractfield %s, "y"
    // %3 removed
    %4 = %1 + %1               // Uses %1 twice
```

### Cross-Block Boundary (Should NOT Eliminate)

```rust
// Test input MIR
block1:
    %1 = %x + %y
    jump block2

block2:
    %2 = %x + %y    // NOT eliminated - different block

// Expected output MIR (unchanged)
block1:
    %1 = %x + %y
    jump block2

block2:
    %2 = %x + %y    // Kept - local CSE is block-local only
```

## Conservative Limitations

### What We Skip

1. **Load Instructions**: Not CSE'd due to potential aliasing
2. **Cross-Block CSE**: Only within basic blocks for simplicity
3. **Mixed Literal/Operand**: Let other passes handle these
4. **Side-Effect Instructions**: Never CSE'd for correctness

### Type Safety

```rust
// Always include type information in keys for safety
PureExpressionKey::Binary {
    op,
    left,
    right,
    result_type: op.result_type(), // Ensure type consistency
}
```

## Performance Considerations

- **Hash Map Lookup**: O(1) average case for expression lookup
- **Block-Local Only**: Avoids expensive dominance analysis
- **Single Pass Per Block**: Linear in instruction count
- **Immediate Removal**: Uses existing use-count analysis

## Integration with Other Passes

- **ArithmeticSimplify**: Creates simpler expressions that CSE can handle better
- **ConstantFolding**: Reduces expressions that CSE might otherwise handle
- **CopyPropagation**: Reduces noise for CSE by eliminating copy chains
- **SimplifyBranches**: Benefits from CSE'd boolean expressions
- **DeadCodeElimination**: Cleans up instructions removed by CSE

## Memory Analysis Avoidance

By being conservative about `Load` instructions and staying within basic blocks,
we avoid the need for expensive alias analysis while still catching the most
common redundant expressions.
