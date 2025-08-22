use super::MirPass;
use crate::{InstructionKind, MirFunction, Value, ValueId};
use std::collections::HashMap;

/// Copy Propagation Pass
///
/// This pass removes redundant assignments in SSA form by replacing uses of
/// copied values with their original sources. It eliminates instructions of the form:
/// `%dest = assign %source (type)` by replacing all uses of `%dest` with `%source`.
///
/// ### Examples:
/// - `%2 = %1; %3 = %2 + 1` → `%3 = %1 + 1`
/// - Multiple copies: `%2 = %1; %3 = %2` → all uses of `%3` become uses of `%1`
/// - Cross-block: Works safely due to SSA dominance properties
#[derive(Debug, Default)]
pub struct CopyPropagation;

impl CopyPropagation {
    /// Create a new copy propagation pass
    pub const fn new() -> Self {
        Self
    }

    /// Collect all copy instructions that can be eliminated
    /// Returns a map from destination value to ultimate source value (transitively resolved)
    fn collect_copies(&self, function: &MirFunction) -> HashMap<ValueId, ValueId> {
        let mut direct_copies = HashMap::new();

        // First, collect all direct copies
        for (_block_id, block) in function.basic_blocks() {
            for instr in &block.instructions {
                if let InstructionKind::Assign {
                    dest,
                    source: Value::Operand(source_id),
                    ty: _,
                } = &instr.kind
                {
                    // Only eliminate copies from operands, not literals
                    // (literals are handled by constant propagation/folding)
                    // Verify types match for safety
                    if let (Some(dest_ty), Some(source_ty)) = (
                        function.get_value_type(*dest),
                        function.get_value_type(*source_id),
                    ) {
                        if dest_ty == source_ty {
                            direct_copies.insert(*dest, *source_id);
                        }
                    }
                    // If we can't verify types, be conservative and skip
                }
            }
        }

        // Now resolve copy chains transitively to get ultimate sources
        let mut resolved_copies = HashMap::new();
        for (&dest, &source) in &direct_copies {
            let ultimate_source = self.resolve_copy_chain(&direct_copies, source);
            resolved_copies.insert(dest, ultimate_source);
        }

        resolved_copies
    }

    /// Resolve a copy chain to find the ultimate source value
    /// Handles chains like %2 -> %1 -> %0 to return %0 for %2
    fn resolve_copy_chain(
        &self,
        copies: &HashMap<ValueId, ValueId>,
        mut current: ValueId,
    ) -> ValueId {
        let mut visited = std::collections::HashSet::new();

        // Follow the chain until we find a value that's not a copy
        while let Some(&next) = copies.get(&current) {
            // Cycle detection to prevent infinite loops
            if !visited.insert(current) {
                // We've seen this value before - there's a cycle
                // Return the current value to avoid infinite recursion
                break;
            }
            current = next;
        }

        current
    }

    /// Remove copy instructions that have been propagated
    /// Returns true if any instructions were removed
    fn remove_copy_instructions(
        &self,
        function: &mut MirFunction,
        copies: &HashMap<ValueId, ValueId>,
    ) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            // Collect indices to remove (in reverse order to avoid index shifts)
            let mut to_remove = Vec::new();

            for (idx, instr) in block.instructions.iter().enumerate() {
                if let InstructionKind::Assign {
                    dest,
                    source: Value::Operand(_),
                    ..
                } = &instr.kind
                {
                    if copies.contains_key(dest) {
                        to_remove.push(idx);
                    }
                }
            }

            // Remove instructions in reverse order to maintain indices
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
        // Collect all copy instructions that can be eliminated
        let copies = self.collect_copies(function);

        if copies.is_empty() {
            return false; // No copies to propagate
        }

        let mut modified = false;

        // Replace all uses of copied values with their sources
        for (&dest_id, &source_id) in &copies {
            // Use the existing replace_all_uses method which handles:
            // - All instruction operands
            // - Terminator operands
            // - Function parameters and return values
            // - Type information cleanup
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_basic_copy_elimination() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 42; %2 = %1; %3 = %2 + 1
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());
        let val3 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        // %1 = 42
        block.push_instruction(crate::Instruction::assign(
            val1,
            Value::integer(42),
            MirType::felt(),
        ));

        // %2 = %1 (copy instruction)
        block.push_instruction(crate::Instruction::assign(
            val2,
            Value::operand(val1),
            MirType::felt(),
        ));

        // %3 = %2 + 1
        block.push_instruction(crate::Instruction::binary_op(
            crate::BinaryOp::Add,
            val3,
            Value::operand(val2),
            Value::integer(1),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val3)));

        let mut pass = CopyPropagation::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the copy instruction was removed
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 2); // Should have removed the copy

        // Check that %3 = %2 + 1 became %3 = %1 + 1
        if let crate::InstructionKind::BinaryOp { left, .. } = &block.instructions[1].kind {
            assert_eq!(*left, Value::operand(val1)); // Should use val1, not val2
        } else {
            panic!("Expected binary operation");
        }
    }

    #[test]
    fn test_multiple_copies() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %0 = 42; %1 = %0; %2 = %1; %3 = %2 + %0
        let val0 = function.new_typed_value_id(MirType::felt());
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());
        let val3 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();

        block.push_instruction(crate::Instruction::assign(
            val0,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val1,
            Value::operand(val0),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val2,
            Value::operand(val1),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            crate::BinaryOp::Add,
            val3,
            Value::operand(val2),
            Value::operand(val0),
        ));

        block.set_terminator(Terminator::return_value(Value::operand(val3)));

        let mut pass = CopyPropagation::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that both copy instructions were removed
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 2); // Original assign + binary op

        // Check that %3 = %2 + %0 became %3 = %0 + %0 (transitive resolution: %2 -> %1 -> %0)
        if let crate::InstructionKind::BinaryOp { left, right, .. } = &block.instructions[1].kind {
            assert_eq!(*left, Value::operand(val0)); // val2 -> val0 (transitively)
            assert_eq!(*right, Value::operand(val0)); // unchanged
        } else {
            panic!("Expected binary operation");
        }
    }

    #[test]
    fn test_literal_assignment_not_eliminated() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 42 (literal assignment, not a copy)
        let val1 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val1,
            Value::integer(42),
            MirType::felt(),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val1)));

        let original_len = block.instructions.len();

        let mut pass = CopyPropagation::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify literal assignments

        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), original_len); // No instructions removed
    }

    #[test]
    fn test_no_copies_no_modification() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create function with no copy instructions
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val1,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            crate::BinaryOp::Add,
            val2,
            Value::operand(val1),
            Value::integer(1),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val2)));

        let mut pass = CopyPropagation::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify anything when no copies exist
    }

    #[test]
    fn test_cross_block_copy_elimination() {
        let mut function = MirFunction::new("test".to_string());
        let block1 = function.add_basic_block();
        let block2 = function.add_basic_block();
        function.entry_block = block1;

        // Block 1: %1 = 42; %2 = %1; jump block2
        let val1 = function.new_typed_value_id(MirType::felt());
        let val2 = function.new_typed_value_id(MirType::felt());
        let val3 = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(block1).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val1,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val2,
            Value::operand(val1),
            MirType::felt(),
        ));
        block.set_terminator(Terminator::jump(block2));

        // Block 2: %3 = %2 + 1; return %3
        let block = function.get_basic_block_mut(block2).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            crate::BinaryOp::Add,
            val3,
            Value::operand(val2),
            Value::integer(1),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val3)));

        let mut pass = CopyPropagation::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the copy was eliminated in block1
        let block1_ref = function.get_basic_block(block1).unwrap();
        assert_eq!(block1_ref.instructions.len(), 1); // Only the original assignment

        // Check that block2 now uses %1 instead of %2
        let block2_ref = function.get_basic_block(block2).unwrap();
        if let crate::InstructionKind::BinaryOp { left, .. } = &block2_ref.instructions[0].kind {
            assert_eq!(*left, Value::operand(val1)); // Should use val1, not val2
        } else {
            panic!("Expected binary operation");
        }
    }
}
