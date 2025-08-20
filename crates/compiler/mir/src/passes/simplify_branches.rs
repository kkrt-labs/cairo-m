//! # SimplifyBranches Pass
//!
//! This pass simplifies control flow by folding conditional branches with constant conditions
//! and reducing complex branch patterns exposed by earlier optimization passes.

use super::MirPass;
use crate::{BasicBlockId, BinaryOp, Literal, MirFunction, Terminator, Value};

/// SimplifyBranches Pass
///
/// This pass simplifies control flow by folding conditional branches with constant conditions.
/// It handles several patterns:
///
/// ### Examples:
/// - `if true then jump A else jump B` → `jump A`
/// - `if false then jump A else jump B` → `jump B`
/// - `if 0 then jump A else jump B` → `jump B` (0 is false)
/// - `if 42 then jump A else jump B` → `jump A` (non-zero is true)
/// - `if 5 == 3 then jump A else jump B` → `jump B` (constant comparison)
#[derive(Debug, Default)]
pub struct SimplifyBranches;

impl SimplifyBranches {
    /// Create a new SimplifyBranches pass
    pub const fn new() -> Self {
        Self
    }

    /// Try to simplify a conditional branch with constant condition
    const fn simplify_if_terminator(&self, terminator: &Terminator) -> Option<Terminator> {
        if let Terminator::If {
            condition,
            then_target,
            else_target,
        } = terminator
        {
            match condition {
                Value::Literal(Literal::Boolean(true)) => Some(Terminator::jump(*then_target)),
                Value::Literal(Literal::Boolean(false)) => Some(Terminator::jump(*else_target)),
                Value::Literal(Literal::Integer(0)) => {
                    // In Cairo-M, 0 is false
                    Some(Terminator::jump(*else_target))
                }
                Value::Literal(Literal::Integer(_)) => {
                    // Non-zero integers are true
                    Some(Terminator::jump(*then_target))
                }
                _ => None, // Cannot simplify - condition is not constant
            }
        } else {
            None
        }
    }

    /// Try to simplify a comparison branch with constant operands
    fn simplify_branch_cmp(&self, terminator: &Terminator) -> Option<Terminator> {
        if let Terminator::BranchCmp {
            op,
            left,
            right,
            then_target,
            else_target,
        } = terminator
        {
            // Only simplify if both operands are literals
            if let (Value::Literal(left_lit), Value::Literal(right_lit)) = (left, right) {
                let result = self.evaluate_comparison(*op, *left_lit, *right_lit)?;

                if result {
                    Some(Terminator::jump(*then_target))
                } else {
                    Some(Terminator::jump(*else_target))
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Evaluate a comparison operation on literal values
    const fn evaluate_comparison(
        &self,
        op: BinaryOp,
        left: Literal,
        right: Literal,
    ) -> Option<bool> {
        match (op, left, right) {
            // Integer comparisons
            (BinaryOp::Eq, Literal::Integer(a), Literal::Integer(b)) => Some(a == b),
            (BinaryOp::Neq, Literal::Integer(a), Literal::Integer(b)) => Some(a != b),
            (BinaryOp::Less, Literal::Integer(a), Literal::Integer(b)) => Some(a < b),
            (BinaryOp::Greater, Literal::Integer(a), Literal::Integer(b)) => Some(a > b),
            (BinaryOp::LessEqual, Literal::Integer(a), Literal::Integer(b)) => Some(a <= b),
            (BinaryOp::GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => Some(a >= b),

            // U32 comparisons (treat as unsigned)
            (BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) == (b as u32))
            }
            (BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) != (b as u32))
            }
            (BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) < (b as u32))
            }
            (BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) > (b as u32))
            }
            (BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) <= (b as u32))
            }
            (BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some((a as u32) >= (b as u32))
            }

            // Boolean comparisons
            (BinaryOp::Eq, Literal::Boolean(a), Literal::Boolean(b)) => Some(a == b),
            (BinaryOp::Neq, Literal::Boolean(a), Literal::Boolean(b)) => Some(a != b),

            // Boolean logic (if used in branch conditions)
            (BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b)) => Some(a && b),
            (BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b)) => Some(a || b),

            _ => None, // Unsupported or invalid comparison
        }
    }
}

impl MirPass for SimplifyBranches {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Collect block IDs to avoid borrowing issues
        let block_ids: Vec<BasicBlockId> = function.basic_blocks.indices().collect();

        for block_id in block_ids {
            if let Some(block) = function.basic_blocks.get(block_id) {
                let current_terminator = block.terminator.clone();

                // Try to simplify the terminator
                let new_terminator = self
                    .simplify_if_terminator(&current_terminator)
                    .or_else(|| self.simplify_branch_cmp(&current_terminator));

                if let Some(new_term) = new_terminator {
                    // Use the new utility function to update edges properly
                    function.set_terminator_with_edges(block_id, new_term);
                    modified = true;
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "SimplifyBranches"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MirType;

    #[test]
    fn test_constant_boolean_condition_true() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if true then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::Literal(Literal::Boolean(true)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump then_block
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, then_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_constant_boolean_condition_false() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if false then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::Literal(Literal::Boolean(false)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump else_block
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, else_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_integer_condition_zero_is_false() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 0 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::Literal(Literal::Integer(0)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump else_block (0 is false)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, else_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_integer_condition_nonzero_is_true() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 42 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::Literal(Literal::Integer(42)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump then_block (non-zero is true)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, then_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_constant_comparison_equal_true() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 5 == 5 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::Eq,
            left: Value::Literal(Literal::Integer(5)),
            right: Value::Literal(Literal::Integer(5)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump then_block (5 == 5 is true)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, then_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_constant_comparison_equal_false() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 5 == 3 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::Eq,
            left: Value::Literal(Literal::Integer(5)),
            right: Value::Literal(Literal::Integer(3)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump else_block (5 != 3)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, else_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_constant_comparison_less_than() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 3 < 7 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::Less,
            left: Value::Literal(Literal::Integer(3)),
            right: Value::Literal(Literal::Integer(7)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump then_block (3 < 7 is true)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, then_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_u32_comparison() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if U32Less 3, 7 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::U32Less,
            left: Value::Literal(Literal::Integer(3)),
            right: Value::Literal(Literal::Integer(7)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump then_block (3 < 7 as u32)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, then_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_boolean_logic_and_false() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if true && false then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::And,
            left: Value::Literal(Literal::Boolean(true)),
            right: Value::Literal(Literal::Boolean(false)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that it became: jump else_block (true && false = false)
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::Jump { target } => assert_eq!(*target, else_block),
            _ => panic!("Expected jump terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_variable_condition_not_simplified() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create a variable (not constant)
        let var = function.new_typed_value_id(MirType::felt());

        // Create: if %var then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::operand(var),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify variable conditions

        // Check that it remains unchanged
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::If {
                condition,
                then_target,
                else_target,
            } => {
                assert_eq!(*condition, Value::operand(var));
                assert_eq!(*then_target, then_block);
                assert_eq!(*else_target, else_block);
            }
            _ => panic!("Expected if terminator, got: {:?}", block.terminator),
        }
    }

    #[test]
    fn test_mixed_literal_variable_comparison_not_simplified() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create a variable (not constant)
        let var = function.new_typed_value_id(MirType::felt());

        // Create: if %var == 5 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::Eq,
            left: Value::operand(var),
            right: Value::Literal(Literal::Integer(5)),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(!modified); // Should not modify mixed literal/variable comparisons

        // Check that it remains unchanged
        let block = function.get_basic_block(entry).unwrap();
        match &block.terminator {
            Terminator::BranchCmp {
                op,
                left,
                right,
                then_target,
                else_target,
            } => {
                assert_eq!(*op, BinaryOp::Eq);
                assert_eq!(*left, Value::operand(var));
                assert_eq!(*right, Value::Literal(Literal::Integer(5)));
                assert_eq!(*then_target, then_block);
                assert_eq!(*else_target, else_block);
            }
            _ => panic!(
                "Expected branch cmp terminator, got: {:?}",
                block.terminator
            ),
        }
    }
}
