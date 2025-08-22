use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BinaryOp, InstructionKind, MirFunction, Terminator, Value};

use super::{const_eval::ConstEvaluator, MirPass};

/// Fuse Compare and Branch Pass
///
/// This pass identifies a `BinaryOp` performing a comparison (e.g., `Eq`)
/// whose result is only used in a subsequent `If` terminator, and fuses them
/// into a single, more efficient `BranchCmp` terminator.
///
/// ### Before:
/// ```mir
/// block_N:
///   %1 = binary_op Eq, %a, %b
///   if %1 then jump then_block else jump else_block
/// ```
///
/// ### After:
/// ```mir
/// block_N:
///   if %a Eq %b then jump then_block else jump else_block
/// ```
#[derive(Debug, Default)]
pub struct FuseCmpBranch {
    evaluator: ConstEvaluator,
}

impl FuseCmpBranch {
    /// Create a new pass
    pub const fn new() -> Self {
        Self {
            evaluator: ConstEvaluator::new(),
        }
    }

    /// Returns true if an op is a comparison that can be fused.
    const fn is_fusible_comparison(op: BinaryOp) -> bool {
        // For now, only Eq is guaranteed to work. In the future we might have Le / Ge opcodes
        matches!(
            op,
            BinaryOp::Eq | BinaryOp::Neq | BinaryOp::U32Eq | BinaryOp::U32Neq
        )
    }
}

impl MirPass for FuseCmpBranch {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;
        let use_counts = function.get_value_use_counts();

        for block in function.basic_blocks.iter_mut() {
            // We are looking for a block that ends in `If`.
            if let Terminator::If {
                condition: Value::Operand(cond_val_id),
                then_target,
                else_target,
            } = block.terminator
            {
                // The condition's result must be used exactly once (in this `If`).
                if use_counts.get(&cond_val_id).cloned() != Some(1) {
                    continue;
                }

                // The instruction defining the condition must be the last one in the block.
                if let Some(last_instr) = block.instructions.last() {
                    if last_instr.destination() == Some(cond_val_id) {
                        if let InstructionKind::BinaryOp {
                            op, left, right, ..
                        } = &last_instr.kind
                        {
                            if Self::is_fusible_comparison(*op) {
                                // We found the pattern! Perform the fusion.

                                // We first check for comparisons with 0 which can be optimized
                                match (
                                    *op,
                                    self.evaluator.is_zero(left),
                                    self.evaluator.is_zero(right),
                                ) {
                                    (BinaryOp::Eq | BinaryOp::U32Eq, true, false) => {
                                        // 0 == x is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(*right, else_target, then_target);
                                    }
                                    (BinaryOp::Eq | BinaryOp::U32Eq, false, true) => {
                                        // x == 0 is equivalent to !x, so we switch the targets
                                        block.terminator =
                                            Terminator::branch(*left, else_target, then_target);
                                    }
                                    (BinaryOp::Neq | BinaryOp::U32Neq, true, false) => {
                                        // 0 != x is equivalent to x, so we use x as the condition
                                        block.terminator =
                                            Terminator::branch(*right, then_target, else_target);
                                    }
                                    (BinaryOp::Neq | BinaryOp::U32Neq, false, true) => {
                                        // x != 0 is equivalent to x, so we use x as the condition
                                        block.terminator =
                                            Terminator::branch(*left, then_target, else_target);
                                    }
                                    _ => {
                                        // For all other cases, we can fuse the comparison and branch
                                        block.terminator = Terminator::branch_cmp(
                                            *op,
                                            *left,
                                            *right,
                                            then_target,
                                            else_target,
                                        );
                                    }
                                }

                                // Remove the now-redundant BinaryOp instruction.
                                block.instructions.pop();

                                modified = true;
                            }
                        } else if let InstructionKind::UnaryOp { op, source, .. } = &last_instr.kind
                        {
                            if matches!(op, UnaryOp::Not) {
                                // If the condition is a not, we switch the targets
                                // For simplicity, we assume dumb conditions such as !42 will never appear in the source code
                                block.terminator =
                                    Terminator::branch(*source, else_target, then_target);

                                // Remove the now-redundant UnaryOp instruction.
                                block.instructions.pop();

                                modified = true;
                            }
                        }
                    }
                }
            }
        }

        modified
    }

    fn name(&self) -> &'static str {
        "FuseCmpBranch"
    }
}
