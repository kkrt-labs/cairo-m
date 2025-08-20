use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, Literal, MirFunction, MirType, Value};

use super::MirPass;

/// Constant Folding Pass
///
/// This pass evaluates operations when all operands are compile-time literals,
/// replacing the instruction with a direct assignment to the computed result.
///
/// ### Examples:
/// - `3 + 4 → 7`
/// - `5 * 0 → 0`
/// - `10 == 5 → false`
/// - `true && false → false`
#[derive(Debug, Default)]
pub struct ConstantFolding;

impl ConstantFolding {
    /// Create a new constant folding pass
    pub const fn new() -> Self {
        Self
    }

    /// Try to fold a binary operation with literal operands
    const fn try_fold_binary_op(
        &self,
        op: BinaryOp,
        left: Literal,
        right: Literal,
    ) -> Option<Literal> {
        match (op, left, right) {
            // Felt arithmetic
            (BinaryOp::Add, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.saturating_add(b)))
            }
            (BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.saturating_sub(b)))
            }
            (BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.saturating_mul(b)))
            }
            (BinaryOp::Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 => {
                Some(Literal::Integer(a / b))
            }

            // Felt comparisons
            (BinaryOp::Eq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a == b))
            }
            (BinaryOp::Neq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a != b))
            }
            (BinaryOp::Less, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a < b))
            }
            (BinaryOp::Greater, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a > b))
            }
            (BinaryOp::LessEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a <= b))
            }
            (BinaryOp::GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a >= b))
            }

            // U32 arithmetic (with proper wrapping)
            (BinaryOp::U32Add, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(((a as u32).wrapping_add(b as u32)) as i32))
            }
            (BinaryOp::U32Sub, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(((a as u32).wrapping_sub(b as u32)) as i32))
            }
            (BinaryOp::U32Mul, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(((a as u32).wrapping_mul(b as u32)) as i32))
            }
            (BinaryOp::U32Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 => {
                Some(Literal::Integer(((a as u32) / (b as u32)) as i32))
            }

            // U32 comparisons
            (BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) == (b as u32)))
            }
            (BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) != (b as u32)))
            }
            (BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) < (b as u32)))
            }
            (BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) > (b as u32)))
            }
            (BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) <= (b as u32)))
            }
            (BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean((a as u32) >= (b as u32)))
            }

            // Boolean operations
            (BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b)) => {
                Some(Literal::Boolean(a && b))
            }
            (BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b)) => {
                Some(Literal::Boolean(a || b))
            }

            _ => None, // Cannot fold or unsafe to fold
        }
    }

    /// Try to fold a unary operation with literal operand
    const fn try_fold_unary_op(&self, op: UnaryOp, operand: Literal) -> Option<Literal> {
        match (op, operand) {
            (UnaryOp::Not, Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
            (UnaryOp::Neg, Literal::Integer(i)) => Some(Literal::Integer(-i)),
            _ => None,
        }
    }

    /// Try to fold an instruction if all operands are literals
    fn try_fold_instruction(&self, instr: &mut crate::Instruction) -> bool {
        match &instr.kind {
            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
            } => {
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

            _ => {}
        }

        false
    }
}

impl MirPass for ConstantFolding {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Iterate through all blocks and instructions
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_arithmetic_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 3 + 4
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val_result,
            Value::integer(3),
            Value::integer(4),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the addition was folded to 7
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::integer(7));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_comparison_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 5 == 3
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Eq,
            val_result,
            Value::integer(5),
            Value::integer(3),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the comparison was folded to false
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::boolean(false));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_boolean_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = true && false
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::And,
            val_result,
            Value::boolean(true),
            Value::boolean(false),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the boolean operation was folded to false
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::boolean(false));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_unary_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = !true
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::unary_op(
            UnaryOp::Not,
            val_result,
            Value::boolean(true),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the unary operation was folded to false
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::boolean(false));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_u32_arithmetic_folding() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = U32Add -1, 1 (should wrap to 0, since -1 as u32 is u32::MAX)
        let val_result = function.new_typed_value_id(MirType::u32());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::U32Add,
            val_result,
            Value::integer(-1), // -1 as u32 is u32::MAX
            Value::integer(1),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the U32 addition was folded with proper wrapping
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::integer(0)); // Wrapped result
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_division_by_zero_not_folded() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 5 / 0 (should NOT be folded)
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Div,
            val_result,
            Value::integer(5),
            Value::integer(0),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should NOT modify division by zero

        // Check that the instruction is unchanged
        let block = function.get_basic_block(entry).unwrap();
        assert!(matches!(
            block.instructions[0].kind,
            InstructionKind::BinaryOp { .. }
        ));
    }

    #[test]
    fn test_mixed_operands_not_folded() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x + 5 (should NOT be folded - mixed literal/operand)
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val_result,
            Value::operand(val_x),
            Value::integer(5),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should NOT modify mixed operands

        // Check that the instruction is unchanged
        let block = function.get_basic_block(entry).unwrap();
        assert!(matches!(
            block.instructions[1].kind,
            InstructionKind::BinaryOp { .. }
        ));
    }
}
