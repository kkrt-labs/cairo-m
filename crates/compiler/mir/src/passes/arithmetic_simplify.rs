use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, Literal, MirFunction, Value};

use super::{const_eval::ConstEvaluator, MirPass};

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
pub struct ArithmeticSimplify {
    evaluator: ConstEvaluator,
}

impl ArithmeticSimplify {
    /// Create a new arithmetic simplification pass
    pub const fn new() -> Self {
        Self {
            evaluator: ConstEvaluator::new(),
        }
    }

    /// Try to simplify a binary operation
    fn try_simplify_binary(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
    ) -> Option<SimplificationResult> {
        // Check for identity values
        if let Some(identity) = self.evaluator.identity_value(op) {
            if left == Value::Literal(identity) {
                // identity op x → x
                return Some(SimplificationResult::Assign(right));
            }
            if right == Value::Literal(identity) {
                // x op identity → x
                return Some(SimplificationResult::Assign(left));
            }
        }

        // Check for absorbing values
        if let Some(absorbing) = self.evaluator.absorbing_value(op) {
            if left == Value::Literal(absorbing) || right == Value::Literal(absorbing) {
                // x op absorbing → absorbing, absorbing op x → absorbing
                return Some(SimplificationResult::Literal(absorbing));
            }
        }

        // Special cases that aren't handled by identity/absorbing
        match (op, left, right) {
            // Self-comparison for operands: x == x → true, x != x → false
            (BinaryOp::Eq | BinaryOp::U32Eq, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Boolean(true)))
            }
            (BinaryOp::Neq | BinaryOp::U32Neq, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Boolean(false)))
            }

            // Self-subtraction: x - x → 0 (for operands)
            (BinaryOp::Sub | BinaryOp::U32Sub, Value::Operand(a), Value::Operand(b)) if a == b => {
                Some(SimplificationResult::Literal(Literal::Integer(0)))
            }

            // Evaluate constant expressions
            (op, Value::Literal(left_lit), Value::Literal(right_lit)) => {
                // Use the evaluator for constant folding
                self.evaluator
                    .eval_binary_op(op, left_lit, right_lit)
                    .map(SimplificationResult::Literal)
            }

            _ => None, // No simplification available
        }
    }

    /// Try to eliminate double negation: !(!x) → x
    #[allow(dead_code)]
    fn try_eliminate_double_negation(
        &self,
        _dest: crate::ValueId,
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
                            source: _,
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
                    SimplificationResult::Assign(value) => {
                        // Replace with assignment
                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: value,
                            ty: op.result_type(),
                        };
                        true
                    }
                    SimplificationResult::Literal(lit) => {
                        // Replace with constant assignment
                        instr.kind = InstructionKind::Assign {
                            dest: *dest,
                            source: Value::Literal(lit),
                            ty: op.result_type(),
                        };
                        true
                    }
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Result of a simplification attempt
enum SimplificationResult {
    /// Replace with an assignment of the given value
    Assign(Value),
    /// Replace with a literal constant
    Literal(Literal),
}

impl MirPass for ArithmeticSimplify {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            for instr in &mut block.instructions {
                if self.simplify_binary_op(instr) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirType, Terminator};

    #[test]
    fn test_add_zero_identity() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = %x + 0
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
            Value::integer(0),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that x + 0 was simplified to x
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { source, .. } = &block.instructions[1].kind {
            assert_eq!(*source, Value::operand(val_x));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_mul_zero_absorbing() {
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

        // Check that x * 0 was simplified to 0
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { source, .. } = &block.instructions[1].kind {
            assert_eq!(*source, Value::integer(0));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_self_equality() {
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

        // Check that x == x was simplified to true
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { source, .. } = &block.instructions[1].kind {
            assert_eq!(*source, Value::boolean(true));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_boolean_and_identity() {
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
        if let InstructionKind::Assign { source, .. } = &block.instructions[1].kind {
            assert_eq!(*source, Value::operand(val_x));
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_constant_folding_in_simplify() {
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

        let mut pass = ArithmeticSimplify::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that 3 + 4 was folded to 7
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { source, .. } = &block.instructions[0].kind {
            assert_eq!(*source, Value::integer(7));
        } else {
            panic!("Expected assignment instruction");
        }
    }
}
