use index_vec::IndexVec;

use crate::{BasicBlockId, MirFunction, Terminator};

use super::MirPass;
use std::collections::{HashMap, HashSet};

/// Dead Code Elimination Pass
///
/// This pass:
/// - Removes unreachable basic blocks and compacts the CFG (existing behavior)
/// - Additionally removes dead, side-effect-free instructions whose results are
///   no longer used, iterating to a fixed point. This cleans up temporaries that
///   become unused after constant propagation/folding and CSE.
#[derive(Debug, Default)]
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// Create a new dead code elimination pass
    pub const fn new() -> Self {
        Self
    }

    /// Remap all basic-block targets inside a terminator according to `map`.
    /// Only control-flow terminators carry block IDs and need remapping.
    fn remap_terminator(term: Terminator, map: &HashMap<BasicBlockId, BasicBlockId>) -> Terminator {
        match term {
            Terminator::If {
                condition,
                then_target,
                else_target,
            } => {
                let then_target = *map
                    .get(&then_target)
                    .expect("then target should be reachable after DCE");
                let else_target = *map
                    .get(&else_target)
                    .expect("else target should be reachable after DCE");
                Terminator::If {
                    condition,
                    then_target,
                    else_target,
                }
            }
            Terminator::BranchCmp {
                op,
                left,
                right,
                then_target,
                else_target,
            } => {
                let then_target = *map
                    .get(&then_target)
                    .expect("then target should be reachable after DCE");
                let else_target = *map
                    .get(&else_target)
                    .expect("else target should be reachable after DCE");
                Terminator::BranchCmp {
                    op,
                    left,
                    right,
                    then_target,
                    else_target,
                }
            }
            // Remap an unconditional jump.
            // Note: `Terminator::jump(new_target)` is the canonical constructor.
            Terminator::Jump { target } => {
                let target = *map
                    .get(&target)
                    .expect("jump target should be reachable after DCE");
                Terminator::jump(target)
            }
            // Other terminators (e.g., Return, Unreachable, etc.) don't carry block IDs.
            other => other,
        }
    }

    /// Remove dead instructions (no uses, no side effects) to a fixed point.
    /// Returns true if any instructions were removed.
    fn remove_dead_instructions(&self, function: &mut MirFunction) -> bool {
        let mut changed = false;

        loop {
            let use_counts = function.get_value_use_counts();
            let mut removed_any = false;

            for block in function.basic_blocks.iter_mut() {
                let before = block.instructions.len();

                block.instructions.retain(|instr| {
                    // Preserve side-effecting operations unconditionally
                    if instr.has_side_effects() {
                        return true;
                    }

                    // Remove explicit NOPs
                    if matches!(instr.kind, crate::InstructionKind::Nop) {
                        return false;
                    }

                    // If the instruction defines a value, keep it only if used
                    if let Some(dest) = instr.destination() {
                        use_counts.get(&dest).copied().unwrap_or(0) > 0
                    } else {
                        // Keep instructions without destinations (e.g., assertions)
                        true
                    }
                });

                if block.instructions.len() != before {
                    removed_any = true;
                }
            }

            if !removed_any {
                break;
            }
            changed = true;
        }

        changed
    }
}

impl MirPass for DeadCodeElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // 1) Identify unreachable blocks
        let unreachable_vec = function.unreachable_blocks();
        #[allow(clippy::useless_let_if_seq)]
        let mut modified = false;
        if !unreachable_vec.is_empty() {
            let unreachable: HashSet<BasicBlockId> = unreachable_vec.into_iter().collect();

            // 2) Build a new compact `IndexVec` containing only reachable blocks.
            //    Keep an old->new block ID mapping as we rebuild.
            let old_blocks = std::mem::take(&mut function.basic_blocks);
            let mut new_blocks: IndexVec<BasicBlockId, _> = IndexVec::new();
            let mut old_to_new: HashMap<BasicBlockId, BasicBlockId> = HashMap::new();

            for (old_id, block) in old_blocks.into_iter_enumerated() {
                if unreachable.contains(&old_id) {
                    continue;
                }
                let new_id = BasicBlockId::new(new_blocks.len());
                old_to_new.insert(old_id, new_id);
                new_blocks.push(block);
            }

            // Sanity: the entry block must be reachable.
            let mapped_entry = *old_to_new
                .get(&function.entry_block)
                .expect("Entry block was marked unreachable during DCE");

            // 3) Remap all block targets inside terminators for the kept blocks.
            let ids: Vec<BasicBlockId> = new_blocks.indices().collect();
            for bid in ids {
                if let Some(block) = new_blocks.get_mut(bid) {
                    let new_term = Self::remap_terminator(block.terminator.clone(), &old_to_new);
                    block.terminator = new_term;
                }
            }

            // 4) Swap in the compacted blocks and update the entry block.
            function.basic_blocks = new_blocks;
            function.entry_block = mapped_entry;

            // 5) Ensure any internal CFG metadata is consistent by re-applying
            //    terminators via the utility that updates edge tables.
            let ids: Vec<BasicBlockId> = function.basic_blocks.indices().collect();
            for bid in ids {
                if let Some(block) = function.basic_blocks.get(bid) {
                    function.set_terminator_with_edges(bid, block.terminator.clone());
                }
            }
            modified = true;
        }

        // 6) Remove dead pure instructions to a fixed point
        if self.remove_dead_instructions(function) {
            modified = true;
        }

        modified
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOp, Instruction, InstructionKind, MirFunction, MirType, Terminator, Value};

    #[test]
    fn test_remove_dead_pure_instructions() {
        // %a = 1; %b = %a + 2; return %a
        // => %b is dead and should be removed by DCE's dead-instruction sweep.
        let mut f = MirFunction::new("dead_instr".to_string());
        let b = f.add_basic_block();
        f.entry_block = b;

        let a = f.new_typed_value_id(MirType::felt());
        let b_val = f.new_typed_value_id(MirType::felt());

        let block = f.get_basic_block_mut(b).unwrap();
        block.push_instruction(Instruction::assign(a, Value::integer(1), MirType::felt()));
        block.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            b_val,
            Value::operand(a),
            Value::integer(2),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(a)));

        let mut dce = DeadCodeElimination::new();
        let modified = dce.run(&mut f);
        assert!(modified);

        let block = f.get_basic_block(f.entry_block).unwrap();
        // The add to produce %b_val should have been removed
        assert_eq!(block.instructions.len(), 1); // only assignment to %a remains
        assert!(matches!(
            block.instructions[0].kind,
            InstructionKind::Assign { .. }
        ));
    }

    #[test]
    fn test_iterative_dead_removal_chain() {
        // %1 = 1; %2 = %1 + 1; %3 = %2 + 1; return 0
        // => all three defs are dead; should be removed iteratively.
        let mut f = MirFunction::new("dead_chain".to_string());
        let b = f.add_basic_block();
        f.entry_block = b;

        let v1 = f.new_typed_value_id(MirType::felt());
        let v2 = f.new_typed_value_id(MirType::felt());
        let v3 = f.new_typed_value_id(MirType::felt());

        let block = f.get_basic_block_mut(b).unwrap();
        block.push_instruction(Instruction::assign(v1, Value::integer(1), MirType::felt()));
        block.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            v2,
            Value::operand(v1),
            Value::integer(1),
        ));
        block.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            v3,
            Value::operand(v2),
            Value::integer(1),
        ));
        block.set_terminator(Terminator::return_value(Value::integer(0)));

        let mut dce = DeadCodeElimination::new();
        let modified = dce.run(&mut f);
        assert!(modified);

        let block = f.get_basic_block(f.entry_block).unwrap();
        // All three instructions should be gone
        assert!(block.instructions.is_empty());
    }
}
