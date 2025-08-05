//! # Control Flow Helpers
//!
//! This module contains helper functions for constructing control flow
//! structures in MIR, such as loops, branches, and conditional blocks.

use crate::{BasicBlockId, Value};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Creates an unconditional jump to the target block
    pub fn goto(&mut self, target: BasicBlockId) {
        if !self.current_block().is_terminated() {
            self.terminate_with_jump(target);
        }
    }

    /// Creates a conditional branch
    pub fn branch(&mut self, condition: Value, then_block: BasicBlockId, else_block: BasicBlockId) {
        if !self.current_block().is_terminated() {
            self.terminate_with_branch(condition, then_block, else_block);
        }
    }

    /// Helper for creating if-then-else control flow
    pub fn if_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        self.create_if_blocks()
    }

    /// Executes a closure within a specific block, then switches back
    pub fn in_block<F>(&mut self, block_id: BasicBlockId, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let prev_block = self.state.current_block_id;
        let prev_terminated = self.state.is_terminated;

        self.switch_to_block(block_id);
        f(self);

        self.state.current_block_id = prev_block;
        self.state.is_terminated = prev_terminated;
    }
}
