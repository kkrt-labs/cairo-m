# Task 6: Add Pure Expression Value Numbering

## Goal

Add local value numbering (CSE) for pure expressions during SSA construction to
eliminate redundant computations.

## Files to Create

- `mir/src/value_numbering.rs` - New module for pure expression tracking

## Files to Modify

- `mir/src/lib.rs` - Add `pub mod value_numbering;`
- `mir/src/ssa.rs` - Integrate value numbering into SSA builder

## Current State

No local value numbering or common subexpression elimination during MIR
construction.

## Required Changes

### 1. Create Pure Expression Key Module (`mir/src/value_numbering.rs`)

```rust
//! # Local Value Numbering for Pure Expressions
//!
//! This module implements local value numbering within basic blocks
//! to eliminate redundant pure expressions during SSA construction.

use rustc_hash::FxHashMap;
use crate::{BasicBlockId, BinaryOp, UnaryOp, MirType, Value, ValueId, Instruction, InstructionKind};

/// A key representing a pure expression for memoization
///
/// Pure expressions have no side effects and their result depends
/// only on their operands. This allows safe memoization within a block.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PureKey {
    /// Unary operation: (op, operand, result_type)
    Unary {
        op: UnaryOp,
        operand: ValueId,
        result_type: MirType,
    },

    /// Binary operation: (op, left, right, result_type)
    Binary {
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
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

    /// Tuple element insertion: (tuple, index, new_value, tuple_type)
    InsertTuple {
        tuple: ValueId,
        index: usize,
        new_value: ValueId,
        tuple_type: MirType,
    },

    /// Struct field insertion: (struct, field_name, new_value, struct_type)
    InsertField {
        struct_val: ValueId,
        field_name: String,
        new_value: ValueId,
        struct_type: MirType,
    },
}

impl PureKey {
    /// Try to create a PureKey from an instruction
    /// Returns None if the instruction has side effects
    pub fn from_instruction(instr: &Instruction) -> Option<Self> {
        match &instr.kind {
            InstructionKind::Unary { op, src, ty, .. } => {
                if let Value::Operand(operand) = src {
                    Some(PureKey::Unary {
                        op: *op,
                        operand: *operand,
                        result_type: ty.clone(),
                    })
                } else {
                    None // Literal operands are already optimized
                }
            }

            InstructionKind::Binary { op, left, right, ty, .. } => {
                if let (Value::Operand(left_id), Value::Operand(right_id)) = (left, right) {
                    Some(PureKey::Binary {
                        op: *op,
                        left: *left_id,
                        right: *right_id,
                        result_type: ty.clone(),
                    })
                } else {
                    None // Mixed literal/operand not memoized for simplicity
                }
            }

            InstructionKind::ExtractTupleElement { src, index, ty, .. } => {
                if let Value::Operand(tuple_id) = src {
                    Some(PureKey::ExtractTuple {
                        tuple: *tuple_id,
                        index: *index,
                        element_type: ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::ExtractStructField { src, field_name, ty, .. } => {
                if let Value::Operand(struct_id) = src {
                    Some(PureKey::ExtractField {
                        struct_val: *struct_id,
                        field_name: field_name.clone(),
                        field_type: ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::MakeTuple { elements, ty, .. } => {
                let element_ids: Option<Vec<ValueId>> = elements.iter()
                    .map(|v| if let Value::Operand(id) = v { Some(*id) } else { None })
                    .collect();

                element_ids.map(|ids| PureKey::MakeTuple {
                    elements: ids,
                    tuple_type: ty.clone(),
                })
            }

            InstructionKind::MakeStruct { fields, ty, .. } => {
                let field_ids: Option<Vec<(String, ValueId)>> = fields.iter()
                    .map(|(name, v)| {
                        if let Value::Operand(id) = v {
                            Some((name.clone(), *id))
                        } else {
                            None
                        }
                    })
                    .collect();

                field_ids.map(|ids| PureKey::MakeStruct {
                    fields: ids,
                    struct_type: ty.clone(),
                })
            }

            InstructionKind::InsertTuple { src, index, value, ty, .. } => {
                if let (Value::Operand(tuple_id), Value::Operand(value_id)) = (src, value) {
                    Some(PureKey::InsertTuple {
                        tuple: *tuple_id,
                        index: *index,
                        new_value: *value_id,
                        tuple_type: ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::InsertField { src, field_name, value, ty, .. } => {
                if let (Value::Operand(struct_id), Value::Operand(value_id)) = (src, value) {
                    Some(PureKey::InsertField {
                        struct_val: *struct_id,
                        field_name: field_name.clone(),
                        new_value: *value_id,
                        struct_type: ty.clone(),
                    })
                } else {
                    None
                }
            }

            // These instructions have side effects or are not pure
            InstructionKind::Call { .. } |
            InstructionKind::Store { .. } |
            InstructionKind::Load { .. } |
            InstructionKind::FrameAlloc { .. } |
            InstructionKind::GetElementPtr { .. } |
            InstructionKind::Assign { .. } |
            InstructionKind::Debug { .. } |
            InstructionKind::Phi { .. } => None,
        }
    }
}

/// Local value numbering table for a single basic block
#[derive(Debug, Default)]
pub struct LocalValueNumbering {
    /// Map from pure expression to its result ValueId
    expr_table: FxHashMap<PureKey, ValueId>,
}

impl LocalValueNumbering {
    /// Create a new empty value numbering table
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a pure expression, returning existing ValueId if found
    pub fn lookup(&self, key: &PureKey) -> Option<ValueId> {
        self.expr_table.get(key).copied()
    }

    /// Record a pure expression with its result ValueId
    pub fn record(&mut self, key: PureKey, result: ValueId) {
        self.expr_table.insert(key, result);
    }

    /// Clear all recorded expressions (for starting a new block)
    pub fn clear(&mut self) {
        self.expr_table.clear();
    }

    /// Get the number of recorded expressions
    pub fn len(&self) -> usize {
        self.expr_table.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.expr_table.is_empty()
    }
}

/// Per-block value numbering for the entire function
#[derive(Debug, Default)]
pub struct FunctionValueNumbering {
    /// Value numbering table for each block
    block_tables: FxHashMap<BasicBlockId, LocalValueNumbering>,
}

impl FunctionValueNumbering {
    /// Create a new function-wide value numbering
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create the value numbering table for a block
    pub fn get_block_table(&mut self, block: BasicBlockId) -> &mut LocalValueNumbering {
        self.block_tables.entry(block).or_default()
    }

    /// Look up a pure expression in a specific block
    pub fn lookup_in_block(&self, block: BasicBlockId, key: &PureKey) -> Option<ValueId> {
        self.block_tables.get(&block)?.lookup(key)
    }

    /// Record a pure expression in a specific block
    pub fn record_in_block(&mut self, block: BasicBlockId, key: PureKey, result: ValueId) {
        self.get_block_table(block).record(key, result);
    }

    /// Clear the table for a specific block
    pub fn clear_block(&mut self, block: BasicBlockId) {
        if let Some(table) = self.block_tables.get_mut(&block) {
            table.clear();
        }
    }
}
```

### 2. Integrate Value Numbering into SSA Builder

Add to `mir/src/ssa.rs`:

```rust
use crate::value_numbering::{PureKey, FunctionValueNumbering};

// Add to SSABuilder struct:
pub struct SSABuilder<'f> {
    func: &'f mut MirFunction,
    current_def: FxHashMap<(MirDefinitionId, BasicBlockId), ValueId>,
    pending_phi_list: FxHashMap<BasicBlockId, Vec<(MirDefinitionId, ValueId)>>,
    phi_cache: FxHashMap<(BasicBlockId, MirDefinitionId), ValueId>,

    // NEW: Local value numbering
    value_numbering: FunctionValueNumbering,
}

// Add these methods to impl SSABuilder:
impl<'f> SSABuilder<'f> {
    /// Try to find an existing value for a pure expression in the current block
    /// If found, return the existing ValueId. Otherwise, create new instruction.
    pub fn pure_unary(
        &mut self,
        block: BasicBlockId,
        op: UnaryOp,
        operand: ValueId,
        result_type: MirType
    ) -> ValueId {
        let key = PureKey::Unary {
            op,
            operand,
            result_type: result_type.clone(),
        };

        // Check if we already computed this expression
        if let Some(existing) = self.value_numbering.lookup_in_block(block, &key) {
            return existing;
        }

        // Create new instruction
        let dest = self.func.new_typed_value_id(result_type.clone());
        let instr = Instruction::unary(dest, op, Value::Operand(operand), result_type);

        if let Some(block_ref) = self.func.basic_blocks.get_mut(block) {
            block_ref.push_instruction(instr);
        }

        // Record for future lookups
        self.value_numbering.record_in_block(block, key, dest);
        dest
    }

    /// Pure binary operation with CSE
    pub fn pure_binary(
        &mut self,
        block: BasicBlockId,
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
        result_type: MirType,
    ) -> ValueId {
        let key = PureKey::Binary {
            op,
            left,
            right,
            result_type: result_type.clone(),
        };

        if let Some(existing) = self.value_numbering.lookup_in_block(block, &key) {
            return existing;
        }

        let dest = self.func.new_typed_value_id(result_type.clone());
        let instr = Instruction::binary(
            dest,
            op,
            Value::Operand(left),
            Value::Operand(right),
            result_type
        );

        if let Some(block_ref) = self.func.basic_blocks.get_mut(block) {
            block_ref.push_instruction(instr);
        }

        self.value_numbering.record_in_block(block, key, dest);
        dest
    }

    /// Pure tuple extraction with CSE
    pub fn pure_extract_tuple(
        &mut self,
        block: BasicBlockId,
        tuple: ValueId,
        index: usize,
        element_type: MirType,
    ) -> ValueId {
        let key = PureKey::ExtractTuple {
            tuple,
            index,
            element_type: element_type.clone(),
        };

        if let Some(existing) = self.value_numbering.lookup_in_block(block, &key) {
            return existing;
        }

        let dest = self.func.new_typed_value_id(element_type.clone());
        let instr = Instruction::extract_tuple_element(
            dest,
            Value::Operand(tuple),
            index,
            element_type,
        );

        if let Some(block_ref) = self.func.basic_blocks.get_mut(block) {
            block_ref.push_instruction(instr);
        }

        self.value_numbering.record_in_block(block, key, dest);
        dest
    }

    /// Clear value numbering when starting a new block
    pub fn start_block(&mut self, block: BasicBlockId) {
        self.value_numbering.clear_block(block);
    }
}
```

### 3. Add Module to lib.rs

```rust
// In mir/src/lib.rs:
pub mod value_numbering;
pub use value_numbering::{PureKey, LocalValueNumbering, FunctionValueNumbering};
```

## Legacy Code to Remove

AFTER this task completes:

- None (this is purely additive optimization)

## Dependencies

- Task 4 (SSA builder core)
- Requires `InstructionKind` variants and helper methods

## Testing

- Test that identical pure expressions return same ValueId
- Test that different expressions get different ValueIds
- Test that value numbering is block-local
- Test interaction with phi elimination
- Performance test on code with redundant expressions

## Success Criteria

- ✅ Pure expressions are correctly identified and memoized
- ✅ Local value numbering eliminates redundant computations within blocks
- ✅ Non-pure instructions are not memoized
- ✅ Block-local CSE works correctly
- ✅ Integration with SSA builder is seamless
- ✅ Tests demonstrate redundancy elimination
