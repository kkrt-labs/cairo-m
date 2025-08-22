use index_vec::IndexVec;

use crate::{BasicBlockId, MirFunction, Terminator};

use super::MirPass;
use std::collections::{HashMap, HashSet};

/// Dead Code Elimination Pass
///
/// This pass identifies and removes unreachable basic blocks from the function,
/// and **compacts** the CFG by rebuilding the basic block arena without them.
/// All jump targets are remapped to the new (dense) block IDs so downstream
/// consumers (e.g., codegen) never see phantom/unlabeled blocks.
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
}

impl MirPass for DeadCodeElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        // 1) Identify unreachable blocks
        let unreachable_vec = function.unreachable_blocks();
        if unreachable_vec.is_empty() {
            return false; // No changes made
        }
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

        true // Modified the function
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}
