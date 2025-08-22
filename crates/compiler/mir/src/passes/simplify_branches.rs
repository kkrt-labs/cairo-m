//! # SimplifyBranches Pass
//!
//! This pass simplifies control flow by folding conditional branches with constant conditions
//! and reducing complex branch patterns exposed by earlier optimization passes.

use super::{const_eval::ConstEvaluator, MirPass};
use crate::{BasicBlockId, Literal, MirFunction, Terminator, Value};

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
pub struct SimplifyBranches {
    evaluator: ConstEvaluator,
}

impl SimplifyBranches {
    /// Create a new SimplifyBranches pass
    pub const fn new() -> Self {
        Self {
            evaluator: ConstEvaluator::new(),
        }
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
                Value::Literal(lit) => {
                    // Use the evaluator to convert literal to boolean
                    match self.evaluator.as_bool(*lit) {
                        Some(true) => Some(Terminator::jump(*then_target)),
                        Some(false) => Some(Terminator::jump(*else_target)),
                        None => None, // Cannot determine boolean value
                    }
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
                // Use the evaluator to evaluate the comparison
                let result_lit = self.evaluator.eval_binary_op(*op, *left_lit, *right_lit)?;

                // The result should be a boolean
                if let Literal::Boolean(result) = result_lit {
                    if result {
                        Some(Terminator::jump(*then_target))
                    } else {
                        Some(Terminator::jump(*else_target))
                    }
                } else {
                    None // Comparison didn't produce a boolean (shouldn't happen)
                }
            } else {
                None
            }
        } else {
            None
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
    use crate::{BinaryOp, Value};

    #[test]
    fn test_simplify_true_branch() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if true then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::boolean(true),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the branch was simplified to jump then_block
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.terminator, Terminator::jump(then_block));
    }

    #[test]
    fn test_simplify_false_branch() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if false then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::boolean(false),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the branch was simplified to jump else_block
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.terminator, Terminator::jump(else_block));
    }

    #[test]
    fn test_simplify_zero_branch() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 0 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::integer(0),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the branch was simplified to jump else_block (0 is false)
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.terminator, Terminator::jump(else_block));
    }

    #[test]
    fn test_simplify_nonzero_branch() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 42 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::integer(42),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the branch was simplified to jump then_block (non-zero is true)
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.terminator, Terminator::jump(then_block));
    }

    #[test]
    fn test_simplify_comparison_branch() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if 5 == 3 then jump then_block else jump else_block
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::BranchCmp {
            op: BinaryOp::Eq,
            left: Value::integer(5),
            right: Value::integer(3),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(modified);

        // Check that the comparison was evaluated and branch simplified to jump else_block
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(block.terminator, Terminator::jump(else_block));
    }

    #[test]
    fn test_no_simplification_for_variable() {
        let mut function = MirFunction::new("test".to_string());
        let entry = function.add_basic_block();
        let then_block = function.add_basic_block();
        let else_block = function.add_basic_block();
        function.entry_block = entry;

        // Create: if %1 then jump then_block else jump else_block
        let val_id = function.new_value_id();
        let block = function.get_basic_block_mut(entry).unwrap();
        block.set_terminator(Terminator::If {
            condition: Value::operand(val_id),
            then_target: then_block,
            else_target: else_block,
        });

        let mut pass = SimplifyBranches::new();
        let modified = pass.run(&mut function);

        assert!(!modified);

        // Check that the branch was not modified
        let block = function.get_basic_block(entry).unwrap();
        assert_eq!(
            block.terminator,
            Terminator::If {
                condition: Value::operand(val_id),
                then_target: then_block,
                else_target: else_block,
            }
        );
    }
}
