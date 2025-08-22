//! # Control Flow Helpers
//!
//! This module contains helper functions for constructing control flow
//! structures in MIR, such as loops, branches, and conditional blocks.

use crate::{BasicBlockId, Value};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Creates an unconditional jump to the target block
    pub fn goto(&mut self, target: BasicBlockId) {
        let state = self.cfg().jump_to(target);
        self.state.current_block_id = state.current_block_id;
        self.state.is_terminated = state.is_terminated;
    }

    /// Creates a conditional branch
    pub fn branch(&mut self, condition: Value, then_block: BasicBlockId, else_block: BasicBlockId) {
        if !self.is_current_block_terminated() {
            self.terminate_with_branch(condition, then_block, else_block);
        }
    }

    /// Helper for creating if-then-else control flow
    pub fn if_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        self.create_if_blocks()
    }
}
