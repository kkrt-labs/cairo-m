use crate::MirFunction;

use super::MirPass;

/// Dead Code Elimination Pass
///
/// This pass identifies and removes unreachable basic blocks from the function.
/// It uses the function's built-in reachability analysis to find dead blocks.
#[derive(Debug, Default)]
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// Create a new dead code elimination pass
    pub const fn new() -> Self {
        Self
    }
}

impl MirPass for DeadCodeElimination {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let unreachable = function.unreachable_blocks();

        if unreachable.is_empty() {
            return false; // No changes made
        }

        // Sort in reverse order to avoid index invalidation when removing
        let mut unreachable = unreachable;
        unreachable.sort_by_key(|a| std::cmp::Reverse(a.index()));

        // Remove unreachable blocks
        for block_id in unreachable {
            // Note: We need to be careful about removing blocks because IndexVec doesn't
            // directly support removal. For now, we'll mark them as "dead" by replacing
            // with empty blocks. A more sophisticated implementation would compact the CFG.
            if let Some(block) = function.get_basic_block_mut(block_id) {
                block.instructions.clear();
                block.set_terminator(crate::Terminator::Unreachable);
            }
        }

        true // Modified the function
    }

    fn name(&self) -> &'static str {
        "DeadCodeElimination"
    }
}
