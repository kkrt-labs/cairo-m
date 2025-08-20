use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, Literal, MirFunction, Value};

use super::MirPass;

/// Arithmetic Simplification Pass
///
/// This pass performs peephole rewriting of algebraic and logical patterns
/// to reduce instruction count and expose further optimization opportunities.
///
/// ### Examples:
/// - `x + 0 → x`
/// - `x * 1 → x`
/// - `x * 0 → 0`
/// - `x == x → true`
/// - `!(!x) → x`
#[derive(Debug, Default)]
pub struct ArithmeticSimplify;

impl ArithmeticSimplify {
    /// Create a new arithmetic simplification pass
    pub const fn new() -> Self {
        Self
    }

    /// Try to simplify a binary operation
    fn try_simplify_binary(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
    ) -> Option<SimplificationResult> {
        match (op, left, right) {
            // Addition identity: x + 0 → x, 0 + x → x
            (BinaryOp::Add | BinaryOp::U32Add, Value::Literal(Literal::Integer(0)), right) => {
                Some(SimplificationResult::Assign(right))
            }
            (BinaryOp::Add | BinaryOp::U32Add, left, Value::Literal(Literal::Integer(0))) => {
                Some(SimplificationResult::Assign(left))
            }

            // Subtraction identity: x - 0 → x
            (BinaryOp::Sub | BinaryOp::U32Sub, left, Value::Literal(Literal::Integer(0))) => {
                Some(SimplificationResult::Assign(left))
            }

            // Multiplication identities: x * 1 → x, 1 * x → x
            (BinaryOp::Mul | BinaryOp::U32Mul, Value::Literal(Literal::Integer(1)), right) => {
                Some(SimplificationResult::Assign(right))
            }
            (BinaryOp::Mul | BinaryOp::U32Mul, left, Value::Literal(Literal::Integer(1))) => {
                Some(SimplificationResult::Assign(left))
            }

            // Multiplication by zero: x * 0 → 0, 0 * x → 0
            (BinaryOp::Mul | BinaryOp::U32Mul, Value::Literal(Literal::Integer(0)), _)
            | (BinaryOp::Mul | BinaryOp::U32Mul, _, Value::Literal(Literal::Integer(0))) => {
                Some(SimplificationResult::Literal(Literal::Integer(0)))
            }

            // Division identity: x / 1 → x
            (BinaryOp::Div | BinaryOp::U32Div, left, Value::Literal(Literal::Integer(1))) => {
                Some(SimplificationResult::Assign(left))
            }

            // Self-comparison for operands: x == x → true, x != x → false
            (BinaryOp::Eq | BinaryOp::U32Eq, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Boolean(true)))
            }
            (BinaryOp::Neq | BinaryOp::U32Neq, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Boolean(false)))
            }

            // Boolean AND: x && true → x, x && false → false, true && x → x, false && x → false
            (BinaryOp::And, Value::Literal(Literal::Boolean(true)), right) => {
                Some(SimplificationResult::Assign(right))
            }
            (BinaryOp::And, left, Value::Literal(Literal::Boolean(true))) => {
                Some(SimplificationResult::Assign(left))
            }
            (BinaryOp::And, Value::Literal(Literal::Boolean(false)), _)
            | (BinaryOp::And, _, Value::Literal(Literal::Boolean(false))) => {
                Some(SimplificationResult::Literal(Literal::Boolean(false)))
            }

            // Boolean OR: x || true → true, x || false → x, true || x → true, false || x → x
            (BinaryOp::Or, Value::Literal(Literal::Boolean(true)), _)
            | (BinaryOp::Or, _, Value::Literal(Literal::Boolean(true))) => {
                Some(SimplificationResult::Literal(Literal::Boolean(true)))
            }
            (BinaryOp::Or, Value::Literal(Literal::Boolean(false)), right) => {
                Some(SimplificationResult::Assign(right))
            }
            (BinaryOp::Or, left, Value::Literal(Literal::Boolean(false))) => {
                Some(SimplificationResult::Assign(left))
            }

            // Self-subtraction: x - x → 0 (for operands)
            (BinaryOp::Sub | BinaryOp::U32Sub, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Integer(0)))
            }

            _ => None, // No simplification available
        }
    }

    /// Try to eliminate double negation: !(!x) → x
    fn try_eliminate_double_negation(
        &self,
        dest: crate::ValueId,
        inner_id: crate::ValueId,
        function: &MirFunction,
    ) -> bool {
        // Look for the defining instruction of inner_id in the same function
        // This is a conservative approach - we only look for obvious double negation
        for (_block_id, block) in function.basic_blocks() {
            for instr in &block.instructions {
                if let Some(instr_dest) = instr.destination() {
                    if instr_dest == inner_id {
                        if let InstructionKind::UnaryOp {
                            op: UnaryOp::Not,
                            source,
                            ..
                        } = &instr.kind
                        {
                            // Found !x where we have !!x, so we can simplify to x
                            // Note: This is a simplified implementation that doesn't actually
                            // perform the rewrite here, just detects the pattern
                            // The actual rewrite would need to happen in the main loop
                            return true;
                        }
                        break; // Found the defining instruction, but it's not a not
                    }
                }
            }
        }
        false
    }

    /// Apply simplification to a binary operation instruction
    fn simplify_binary_op(&self, instr: &mut crate::Instruction) -> bool {
        if let InstructionKind::BinaryOp {
            op,
            dest,
            left,
            right,
        } = &instr.kind
        {
            if let Some(result) = self.try_simplify_binary(*op, *left, *right) {
                match result {
                    SimplificationResult::Assign(source) => {
                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source,
                            ty: op.result_type(),
                        };
                    }
                    SimplificationResult::Literal(lit) => {
                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: Value::Literal(lit),
                            ty: op.result_type(),
                        };
                    }
                }
                return true;
            }
        }
        false
    }

    /// Apply simplification to a unary operation instruction
    fn simplify_unary_op(&self, instr: &mut crate::Instruction, function: &MirFunction) -> bool {
        if let InstructionKind::UnaryOp { op, dest, source } = &instr.kind {
            if matches!(op, UnaryOp::Not) {
                if let Value::Operand(inner_id) = source {
                    // Check if this is a double negation pattern
                    if self.try_eliminate_double_negation(*dest, *inner_id, function) {
                        // For now, we'll just detect the pattern but not rewrite it
                        // A more complete implementation would need to track and rewrite
                        // the double negation across instructions
                        return false; // TODO: Implement actual double negation elimination
                    }
                }
            }
        }
        false
    }
}

/// Result of attempting to simplify an operation
enum SimplificationResult {
    /// Rewrite to assignment from another value
    Assign(Value),
    /// Rewrite to literal assignment
    Literal(Literal),
}

impl MirPass for ArithmeticSimplify {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // We need to avoid borrowing issues by separating the read-only analysis
        // from the mutable instruction modification
        let block_count = function.basic_blocks.len();

        for block_idx in 0..block_count {
            let block_id = crate::BasicBlockId::from_raw(block_idx);
            if let Some(block) = function.basic_blocks.get_mut(block_id) {
                for instr in &mut block.instructions {
                    // Apply binary operation simplifications
                    if self.simplify_binary_op(instr) {
                        modified = true;
                    }
                    // For now, skip unary simplifications to avoid borrowing issues
                    // TODO: Implement double negation elimination properly
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "ArithmeticSimplify"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_addition_identity() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 42 + 0
        let val_42 = function.new_typed_value_id(MirType::felt());
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_42,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val_result,
            Value::operand(val_42),
            Value::integer(0),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the addition was simplified to an assignment
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.instructions.len(), 2);

        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[1].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::operand(val_42));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_multiplication_by_zero() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x * 0
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Mul,
            val_result,
            Value::operand(val_x),
            Value::integer(0),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that multiplication by zero was simplified to 0
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[1].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::integer(0));
        } else {
            panic!("Expected assignment instruction with literal 0");
        }
    }

    #[test]
    fn test_self_comparison() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x == %x
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Eq,
            val_result,
            Value::operand(val_x),
            Value::operand(val_x),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that self-comparison was simplified to true
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[1].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::boolean(true));
        } else {
            panic!("Expected assignment instruction with literal true");
        }
    }

    #[test]
    fn test_boolean_and_simplification() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x && true
        let val_x = function.new_typed_value_id(MirType::bool());
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::boolean(false),
            MirType::bool(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::And,
            val_result,
            Value::operand(val_x),
            Value::boolean(true),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that x && true was simplified to x
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[1].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::operand(val_x));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_no_simplification_for_complex_expressions() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x + %y (should not be simplified)
        let val_x = function.new_typed_value_id(MirType::felt());
        let val_y = function.new_typed_value_id(MirType::felt());
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::assign(
            val_x,
            Value::integer(42),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::assign(
            val_y,
            Value::integer(37),
            MirType::felt(),
        ));
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val_result,
            Value::operand(val_x),
            Value::operand(val_y),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify complex expressions

        // Check that the instruction is unchanged
        let block = function.get_basic_block(entry).unwrap();
        assert!(matches!(
            block.instructions[2].kind,
            InstructionKind::BinaryOp { .. }
        ));
    }
}
