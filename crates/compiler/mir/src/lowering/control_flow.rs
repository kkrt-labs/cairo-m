//! # Control Flow Helpers
//!
//! This module contains helper functions for constructing control flow
//! structures in MIR, such as loops, branches, and conditional blocks.

use crate::{BasicBlockId, Terminator, Value};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Creates a new basic block and returns its ID
    pub fn new_block(&mut self) -> BasicBlockId {
        self.mir_function.add_basic_block()
    }

    /// Switches to a different basic block for instruction generation
    pub const fn switch_to_block(&mut self, block_id: BasicBlockId) {
        self.current_block_id = block_id;
        self.is_terminated = false;
    }

    /// Creates an unconditional jump to the target block
    pub fn goto(&mut self, target: BasicBlockId) {
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::Jump { target });
        }
    }

    /// Creates a conditional branch
    pub fn branch(&mut self, condition: Value, then_block: BasicBlockId, else_block: BasicBlockId) {
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::If {
                condition,
                then_target: then_block,
                else_target: else_block,
            });
        }
    }

    /// Helper for creating if-then-else control flow
    pub fn if_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();
        (then_block, else_block, merge_block)
    }

    /// Executes a closure within a specific block, then switches back
    pub fn in_block<F>(&mut self, block_id: BasicBlockId, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let prev_block = self.current_block_id;
        let prev_terminated = self.is_terminated;

        self.switch_to_block(block_id);
        f(self);

        self.current_block_id = prev_block;
        self.is_terminated = prev_terminated;
    }
}
