use super::MirPass;
use crate::{BasicBlockId, BinaryOp, InstructionKind, MirFunction, MirType, Value, ValueId};
use cairo_m_compiler_parser::parser::UnaryOp;
use rustc_hash::FxHashMap;

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
    pub(crate) fn from_instruction(instr: &crate::Instruction) -> Option<Self> {
        // Only consider pure instructions
        if !instr.is_pure() {
            return None;
        }

        match &instr.kind {
            InstructionKind::BinaryOp {
                op, left, right, ..
            } => {
                // Only handle operand-operand operations (not mixed literal/operand)
                if let (Value::Operand(left_id), Value::Operand(right_id)) = (left, right) {
                    Some(Self::Binary {
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
                    };

                    Some(Self::Unary {
                        op: *op,
                        operand: *operand_id,
                        result_type,
                    })
                } else {
                    None // Literal operand handled by constant folding
                }
            }

            InstructionKind::ExtractTupleElement {
                tuple,
                index,
                element_ty,
                ..
            } => {
                if let Value::Operand(tuple_id) = tuple {
                    Some(Self::ExtractTuple {
                        tuple: *tuple_id,
                        index: *index,
                        element_type: element_ty.clone(),
                    })
                } else {
                    None
                }
            }

            InstructionKind::ExtractStructField {
                struct_val,
                field_name,
                field_ty,
                ..
            } => {
                if let Value::Operand(struct_id) = struct_val {
                    Some(Self::ExtractField {
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
                let element_ids: Option<Vec<ValueId>> = elements
                    .iter()
                    .map(|v| match v {
                        Value::Operand(id) => Some(*id),
                        _ => None,
                    })
                    .collect();

                element_ids.map(|ids| Self::MakeTuple {
                    elements: ids,
                    tuple_type: MirType::Unknown, // Simplified for now
                })
            }

            InstructionKind::MakeStruct {
                fields, struct_ty, ..
            } => {
                // Only handle all-operand structs
                let field_ids: Option<Vec<(String, ValueId)>> = fields
                    .iter()
                    .map(|(name, v)| match v {
                        Value::Operand(id) => Some((name.clone(), *id)),
                        _ => None,
                    })
                    .collect();

                field_ids.map(|ids| Self::MakeStruct {
                    fields: ids,
                    struct_type: struct_ty.clone(),
                })
            }

            // Skip instructions with side effects or not supported
            InstructionKind::Call { .. }
            | InstructionKind::Assign { .. }
            | InstructionKind::Debug { .. }
            | InstructionKind::Phi { .. }
            | InstructionKind::Nop => None,

            // Aggregate modification operations - skip for conservatism
            InstructionKind::InsertField { .. }
            | InstructionKind::InsertTuple { .. }
            | InstructionKind::ArrayInsert { .. } => None,

            // Cast operations - skip for now
            InstructionKind::Cast { .. } => None,

            // Array operations - can be CSE'd similar to tuples
            InstructionKind::MakeFixedArray { elements, .. } => {
                // Only handle all-operand arrays
                let element_ids: Option<Vec<ValueId>> = elements
                    .iter()
                    .map(|v| match v {
                        Value::Operand(id) => Some(*id),
                        _ => None,
                    })
                    .collect();

                element_ids.map(|ids| Self::MakeTuple {
                    elements: ids,
                    tuple_type: MirType::Unknown, // Use MakeTuple variant for simplicity
                })
            }

            InstructionKind::ArrayIndex {
                array,
                index,
                element_ty,
                ..
            } => {
                // Only CSE when array is operand and index is a constant literal
                if let (
                    Value::Operand(array_id),
                    Value::Literal(crate::value::Literal::Integer(i)),
                ) = (array, index)
                {
                    Some(Self::ExtractTuple {
                        tuple: *array_id, // Reuse ExtractTuple variant
                        index: *i as usize,
                        element_type: element_ty.clone(),
                    })
                } else {
                    None
                }
            }
        }
    }
}

/// Local CSE (Local Value Numbering) Pass
///
/// This pass implements per-basic-block common subexpression elimination
/// for pure expressions. It identifies expressions that compute the same
/// value and eliminates redundant calculations.
///
/// ### Examples:
/// - `%2 = %x + %y; %4 = %x + %y` → `%4 = %2` (second computation eliminated)
/// - `%1 = extracttuple %t, 0; %3 = extracttuple %t, 0` → `%3 = %1`
/// - Cross-instruction patterns within basic blocks
#[derive(Debug, Default)]
pub struct LocalCSE;

impl LocalCSE {
    /// Create a new local CSE pass
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
        for (dest, existing_value, _instr_idx) in replacements {
            // Replace all uses of dest with existing_value
            function.replace_all_uses(dest, existing_value);
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
        // Get use counts first to avoid borrowing conflicts
        let use_counts = function.get_value_use_counts();

        if let Some(block) = function.basic_blocks.get_mut(block_id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_basic_common_subexpression() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x + %y; %2 = %z * 2; %3 = %x + %y; %4 = %1 + %3
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_y = function.new_typed_value_id(MirType::felt());
        let val_z = function.new_typed_value_id(MirType::felt());
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());
        let val3 = function.new_typed_value_id(MirType::felt());
        let val4 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        // Initialize values
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(10),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val_y,
            Value::integer(20),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val_z,
            Value::integer(30),
            MirType::felt(),
        ));

        // %1 = %x + %y
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val1,
            Value::operand(val_x),
            Value::operand(val_y),
        ));

        // %2 = %z * 2
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Mul,
            val2,
            Value::operand(val_z),
            Value::integer(2),
        ));

        // %3 = %x + %y (same as %1)
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val3,
            Value::operand(val_x),
            Value::operand(val_y),
        ));

        // %4 = %1 + %3
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val4,
            Value::operand(val1),
            Value::operand(val3),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val4)));

        let mut pass = LocalCSE::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the redundant computation was eliminated

        // The redundant instruction should have been removed by dead code elimination
        // Let's check that val3 (the duplicate) is no longer used
        let use_counts = function.get_value_use_counts();

        // val3 should no longer be used (replaced with val1)
        assert_eq!(use_counts.get(&val3).copied().unwrap_or(0), 0);

        // val1 should be used at least twice now (original + replacement of val3)
        assert!(use_counts.get(&val1).copied().unwrap_or(0) >= 1);
    }

    #[test]
    fn test_tuple_extraction_cse() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = extracttuple %t, 0; %2 = %1 * 5; %3 = extracttuple %t, 0; %4 = %3 * 5
        let val_t =
            function.new_typed_value_id(MirType::tuple(vec![MirType::felt(), MirType::felt()]));
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());
        let val3 = function.new_typed_value_id(MirType::felt());
        let val4 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        // Initialize tuple
        block.push_instruction(crate::Instruction::make_tuple(
            val_t,
            vec![Value::integer(42), Value::integer(84)],
        ));

        // %1 = extracttuple %t, 0
        block.push_instruction(crate::Instruction::extract_tuple_element(
            val1,
            Value::operand(val_t),
            0,
            MirType::felt(),
        ));

        // %2 = %1 * 5
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Mul,
            val2,
            Value::operand(val1),
            Value::integer(5),
        ));

        // %3 = extracttuple %t, 0 (same as %1)
        block.push_instruction(crate::Instruction::extract_tuple_element(
            val3,
            Value::operand(val_t),
            0,
            MirType::felt(),
        ));

        // %4 = %3 * 5 (should become %4 = %1 * 5 after CSE)
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Mul,
            val4,
            Value::operand(val3),
            Value::integer(5),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val4)));

        let mut pass = LocalCSE::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the redundant tuple extraction was eliminated
        let use_counts = function.get_value_use_counts();

        // val3 should no longer be used (replaced with val1)
        assert_eq!(use_counts.get(&val3).copied().unwrap_or(0), 0);

        // val1 should be used at least once
        assert!(use_counts.get(&val1).copied().unwrap_or(0) >= 1);
    }

    #[test]
    fn test_mixed_literal_operand_not_csed() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create operations with mixed literal/operand (should not be CSE'd)
        let val_x = function.new_typed_value_id(MirType::felt());
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(42),
            MirType::felt(),
        ));

        // %1 = %x + 5 (literal operand)
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val1,
            Value::operand(val_x),
            Value::integer(5),
        ));

        // %2 = %x + 5 (same literal operand - but not CSE'd due to literal)
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val2,
            Value::operand(val_x),
            Value::integer(5),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val2)));

        let original_instr_count = block.instructions.len();

        let mut pass = LocalCSE::new();
        let modified = pass.run(&mut function);

        // Should not modify because mixed literal/operand expressions are skipped
        assert!(!modified);

        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), original_instr_count);
    }

    #[test]
    fn test_cross_block_boundary_no_cse() {
        let mut function = MirFunction::new("test".to_string());
        let block1 = function.add_basic_block();
        let block2 = function.add_basic_block();
        function.entry_block = block1;

        // Block 1: %1 = %x + %y; jump block2
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_y = function.new_typed_value_id(MirType::felt());
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(block1).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(10),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val_y,
            Value::integer(20),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val1,
            Value::operand(val_x),
            Value::operand(val_y),
        ));
        block.set_terminator(Terminator::jump(block2));

        // Block 2: %2 = %x + %y (NOT eliminated - different block)
        let block = function.get_basic_block_mut(block2).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val2,
            Value::operand(val_x),
            Value::operand(val_y),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val2)));

        let mut pass = LocalCSE::new();
        let modified = pass.run(&mut function);

        // Should not modify because LocalCSE is block-local only
        assert!(!modified);

        // Both blocks should retain their instructions
        let block1_ref = function.get_basic_block(block1).unwrap();
        let block2_ref = function.get_basic_block(block2).unwrap();

        // Each block should still have its computation
        assert!(block1_ref
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::BinaryOp { .. })));
        assert!(block2_ref
            .instructions
            .iter()
            .any(|i| matches!(i.kind, InstructionKind::BinaryOp { .. })));
    }

    #[test]
    fn test_no_common_subexpressions() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create function with no common subexpressions
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_y = function.new_typed_value_id(MirType::felt());
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(10),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val_y,
            Value::integer(20),
            MirType::felt(),
        ));

        // %1 = %x + %y
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val1,
            Value::operand(val_x),
            Value::operand(val_y),
        ));

        // %2 = %x * %y (different operation)
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Mul,
            val2,
            Value::operand(val_x),
            Value::operand(val_y),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val2)));

        let mut pass = LocalCSE::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify anything when no common subexpressions exist
    }
}
