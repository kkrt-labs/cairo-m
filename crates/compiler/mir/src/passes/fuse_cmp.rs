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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOp, Instruction, MirType};

    #[test]
    fn test_fuse_basic_eq_to_branchcmp() {
        let mut f = MirFunction::new("fuse_eq".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let y = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = %x == %y
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Eq,
            cond,
            Value::operand(x),
            Value::operand(y),
        ));
        // if %cond then then_b else else_b
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        let modified = pass.run(&mut f);
        assert!(modified);

        let b = f.get_basic_block(entry).unwrap();
        // Binary op should be removed
        assert!(b.instructions.is_empty());
        // Terminator becomes BranchCmp with same operands
        assert_eq!(
            b.terminator,
            Terminator::branch_cmp(
                BinaryOp::Eq,
                Value::operand(x),
                Value::operand(y),
                then_b,
                else_b
            )
        );
    }

    #[test]
    fn test_fuse_u32eq_to_branchcmp() {
        let mut f = MirFunction::new("fuse_u32eq".to_string());
        let entry = f.add_basic_block();
        let t = f.add_basic_block();
        let e = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::u32());
        let y = f.new_typed_value_id(MirType::u32());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        b.push_instruction(Instruction::binary_op(
            BinaryOp::U32Eq,
            cond,
            Value::operand(x),
            Value::operand(y),
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), t, e));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        assert_eq!(
            b.terminator,
            Terminator::branch_cmp(BinaryOp::U32Eq, Value::operand(x), Value::operand(y), t, e)
        );
    }

    #[test]
    fn test_fuse_eq_zero_left_swaps_targets() {
        let mut f = MirFunction::new("fuse_eq_zero_left".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = 0 == %x  -> branch on %x with swapped targets
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Eq,
            cond,
            Value::integer(0),
            Value::operand(x),
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        // 0 == x  => !x, so jump else on true x, then on false -> swapped
        assert_eq!(
            b.terminator,
            Terminator::branch(Value::operand(x), else_b, then_b)
        );
    }

    #[test]
    fn test_fuse_eq_zero_right_swaps_targets() {
        let mut f = MirFunction::new("fuse_eq_zero_right".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = %x == 0  -> branch on %x with swapped targets
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Eq,
            cond,
            Value::operand(x),
            Value::integer(0),
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        assert_eq!(
            b.terminator,
            Terminator::branch(Value::operand(x), else_b, then_b)
        );
    }

    #[test]
    fn test_fuse_neq_zero_uses_direct_condition() {
        let mut f = MirFunction::new("fuse_neq_zero".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = %x != 0  -> branch on %x as-is
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Neq,
            cond,
            Value::operand(x),
            Value::integer(0),
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        assert_eq!(
            b.terminator,
            Terminator::branch(Value::operand(x), then_b, else_b)
        );
    }

    #[test]
    fn test_fuse_neq_zero_left_uses_direct_condition() {
        let mut f = MirFunction::new("fuse_neq_zero_left".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = 0 != %x  -> branch on %x as-is
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Neq,
            cond,
            Value::integer(0),
            Value::operand(x),
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        assert_eq!(
            b.terminator,
            Terminator::branch(Value::operand(x), then_b, else_b)
        );
    }

    #[test]
    fn test_not_condition_flips_targets() {
        let mut f = MirFunction::new("fuse_not".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::bool());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        // %cond = !%x
        b.push_instruction(Instruction::unary_op(UnaryOp::Not, cond, Value::operand(x)));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        assert!(pass.run(&mut f));

        let b = f.get_basic_block(entry).unwrap();
        assert!(b.instructions.is_empty());
        // !x then T else E  => x then E else T
        assert_eq!(
            b.terminator,
            Terminator::branch(Value::operand(x), else_b, then_b)
        );
    }

    #[test]
    fn test_no_fuse_when_last_instr_is_not_condition_def() {
        let mut f = MirFunction::new("no_fuse_last".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let y = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        let b = f.get_basic_block_mut(entry).unwrap();
        b.push_instruction(Instruction::binary_op(
            BinaryOp::Eq,
            cond,
            Value::operand(x),
            Value::operand(y),
        ));
        // Add another instruction after the comparison so it's not last
        b.push_instruction(Instruction::debug(
            "use".to_string(),
            vec![Value::operand(x)],
        ));
        b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));

        let mut pass = FuseCmpBranch::new();
        let modified = pass.run(&mut f);
        assert!(!modified);

        let b = f.get_basic_block(entry).unwrap();
        assert!(matches!(b.terminator, Terminator::If { .. }));
    }

    #[test]
    fn test_no_fuse_when_condition_used_multiple_times() {
        let mut f = MirFunction::new("no_fuse_multiuse".to_string());
        let entry = f.add_basic_block();
        let then_b = f.add_basic_block();
        let else_b = f.add_basic_block();
        f.entry_block = entry;

        let x = f.new_typed_value_id(MirType::felt());
        let y = f.new_typed_value_id(MirType::felt());
        let cond = f.new_typed_value_id(MirType::bool());

        // Entry: define cond as last instruction and branch on it
        {
            let b = f.get_basic_block_mut(entry).unwrap();
            b.push_instruction(Instruction::binary_op(
                BinaryOp::Eq,
                cond,
                Value::operand(x),
                Value::operand(y),
            ));
            b.set_terminator(Terminator::branch(Value::operand(cond), then_b, else_b));
        }

        // Also use cond in the then-block (second use) to prevent fusion
        {
            let tb = f.get_basic_block_mut(then_b).unwrap();
            tb.push_instruction(Instruction::debug(
                "use cond".to_string(),
                vec![Value::operand(cond)],
            ));
            tb.set_terminator(Terminator::return_void());
        }

        let mut pass = FuseCmpBranch::new();
        let modified = pass.run(&mut f);
        assert!(!modified);

        let b = f.get_basic_block(entry).unwrap();
        assert!(matches!(b.terminator, Terminator::If { .. }));
    }
}
