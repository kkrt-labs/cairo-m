use cairo_m_compiler_parser::parser::UnaryOp;

use super::MirPass;
use super::const_eval::ConstEvaluator;
use crate::{InstructionKind, MirFunction, MirType, Value};

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
pub struct ConstantFolding {
    evaluator: ConstEvaluator,
}

impl ConstantFolding {
    /// Create a new constant folding pass
    pub const fn new() -> Self {
        Self {
            evaluator: ConstEvaluator::new(),
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
                    if let Some(result) = self.evaluator.eval_binary_op(*op, *left_lit, *right_lit)
                    {
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

            InstructionKind::UnaryOp {
                op,
                dest,
                source: Value::Literal(source_lit),
            } => {
                if let Some(result) = self.evaluator.eval_unary_op(*op, *source_lit) {
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
    use stwo_prover::core::fields::m31::{M31, P};

    use super::*;
    use crate::{BinaryOp, MirType, Terminator};

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

        // Create: %1 = U32Add u32::MAX, 1 (should wrap to 0)
        let val_result = function.new_typed_value_id(MirType::u32());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::U32Add,
            val_result,
            Value::integer(u32::MAX),
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
    fn test_felt_modular_arithmetic() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = (P - 1) + 2 (should wrap to 1)
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Add,
            val_result,
            Value::integer(P - 1),
            Value::integer(2),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the addition wrapped correctly in M31
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::integer(1)); // (P-1) + 2 = 1 mod P
        } else {
            panic!("Expected assignment instruction");
        }
    }

    #[test]
    fn test_u32_comparison_unsigned() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 0x80000000 >u32 0x7FFFFFFF (should be true for unsigned)
        let val_result = function.new_typed_value_id(MirType::bool());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::U32Greater,
            val_result,
            Value::integer(0x80000000),
            Value::integer(0x7FFFFFFF),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the comparison was evaluated correctly as unsigned
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { dest, source, .. } = &block.instructions[0].kind {
            assert_eq!(*dest, val_result);
            assert_eq!(*source, Value::boolean(true)); // 0x80000000 > 0x7FFFFFFF in unsigned
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

    #[test]
    fn test_felt_division_inverse() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        function.entry_block = entry;

        // Create: %1 = 1 / 2 (should use modular inverse)
        let val_result = function.new_typed_value_id(MirType::felt());

        let block = function.get_basic_block_mut(entry).unwrap();
        block.push_instruction(crate::Instruction::binary_op(
            BinaryOp::Div,
            val_result,
            Value::integer(1),
            Value::integer(2),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(val_result)));

        let mut pass = ConstantFolding::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that division was folded using modular inverse
        let block = function.get_basic_block(entry).unwrap();
        if let InstructionKind::Assign { source, .. } = &block.instructions[0].kind {
            if let Value::Literal(crate::Literal::Integer(inv2)) = source {
                // Verify that inv2 * 2 = 1 (mod P)
                let m31_inv2 = M31::from(*inv2);
                let m31_2 = M31::from(2u32);
                let product = m31_inv2 * m31_2;
                assert_eq!(product.0, 1, "2 * (1/2) should equal 1 in M31");
            } else {
                panic!("Expected integer literal");
            }
        } else {
            panic!("Expected assignment instruction");
        }
    }
}
