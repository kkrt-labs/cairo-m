//! # Control Flow Graph Builder
//!
//! This module provides specialized operations for constructing and manipulating
//! the control flow graph of MIR functions. It centralizes block creation,
//! termination, and navigation logic.

use crate::{BasicBlock, BasicBlockId, MirFunction, Terminator, Value};

/// Result of CFG operations that modify state
///
/// This allows the CfgBuilder to return state changes without holding borrows
#[derive(Debug, Clone, Copy)]
pub struct CfgState {
    pub current_block_id: BasicBlockId,
    pub is_terminated: bool,
}

/// A builder for control flow graph operations
///
/// The CfgBuilder manages basic block creation, termination, and navigation
/// within a MIR function. It provides a clean API for control flow operations
/// while maintaining invariants about block termination and connectivity.
pub struct CfgBuilder<'f> {
    function: &'f mut MirFunction,
    current_block_id: BasicBlockId,
    is_terminated: bool,
}

impl<'f> CfgBuilder<'f> {
    /// Creates a new CFG builder for the given function
    ///
    /// ## Arguments
    /// * `function` - The MIR function to build the CFG for
    /// * `current_block_id` - The initially active block
    pub const fn new(function: &'f mut MirFunction, current_block_id: BasicBlockId) -> Self {
        Self {
            function,
            current_block_id,
            is_terminated: false,
        }
    }

    /// Creates a new basic block with an optional name
    ///
    /// ## Arguments
    /// * `name` - Optional name for debugging purposes
    ///
    /// ## Returns
    /// The ID of the newly created block
    pub(crate) fn new_block(&mut self, name: Option<String>) -> BasicBlockId {
        let block_name =
            name.unwrap_or_else(|| format!("block_{}", self.function.basic_blocks.len()));
        self.function.add_basic_block_with_name(block_name)
    }

    /// Returns the current CFG state
    ///
    /// This is useful for updating external state after CFG operations
    pub const fn state(&self) -> CfgState {
        CfgState {
            current_block_id: self.current_block_id,
            is_terminated: self.is_terminated,
        }
    }

    /// Switches the current block to the specified block
    ///
    /// This resets the termination flag since we're starting fresh in a new block.
    ///
    /// ## Arguments
    /// * `block_id` - The block to switch to
    ///
    /// ## Returns
    /// The new CFG state after switching
    pub const fn switch_to_block(&mut self, block_id: BasicBlockId) -> CfgState {
        self.current_block_id = block_id;
        self.is_terminated = false;
        self.state()
    }

    /// Returns the current block ID
    pub const fn current_block_id(&self) -> BasicBlockId {
        self.current_block_id
    }

    /// Returns a reference to the current block
    pub(crate) fn current_block(&self) -> &BasicBlock {
        self.function
            .basic_blocks
            .get(self.current_block_id)
            .expect("Current block should exist")
    }

    /// Checks if the current block is terminated
    pub(crate) fn is_terminated(&self) -> bool {
        self.is_terminated || self.current_block().has_terminator()
    }

    /// Sets the terminator for a block, handling edge cleanup properly
    ///
    /// This private helper consolidates terminator setting logic and ensures
    /// proper edge management for both new and replacement terminators.
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to set terminator for
    /// * `terminator` - The terminator to set
    fn set_terminator_internal(&mut self, block_id: BasicBlockId, terminator: Terminator) {
        // First collect the old targets if the block has a terminator
        let old_targets = {
            let block = self.function.basic_blocks.get(block_id).unwrap_or_else(|| {
                panic!("set_terminator_internal: invalid block_id {:?}", block_id)
            });
            if block.has_terminator() {
                block.terminator.target_blocks()
            } else {
                vec![]
            }
        };

        // Remove old edges
        for old_target in old_targets {
            self.function.disconnect(block_id, old_target);
        }

        // Set new terminator
        self.function
            .basic_blocks
            .get_mut(block_id)
            .unwrap_or_else(|| {
                panic!(
                    "set_terminator_internal: block {:?} disappeared during update",
                    block_id
                )
            })
            .set_terminator(terminator.clone());

        // Connect new edges
        let new_targets = terminator.target_blocks();
        for new_target in new_targets {
            self.function.connect(block_id, new_target);
        }
    }

    /// Terminates the current block with the given terminator
    /// Also updates pred/succ edges based on terminator targets
    ///
    /// ## Arguments
    /// * `terminator` - The terminator to set
    ///
    /// ## Returns
    /// The new CFG state after termination
    ///
    /// ## Panics
    /// Panics if the block is already terminated
    pub(crate) fn terminate(&mut self, terminator: Terminator) -> CfgState {
        if self.is_terminated() {
            panic!("Attempting to terminate an already terminated block");
        }

        self.set_terminator_internal(self.current_block_id, terminator);
        self.is_terminated = true;
        self.state()
    }

    /// Terminates the current block with a jump to the target block
    ///
    /// ## Arguments
    /// * `target` - The block to jump to
    ///
    /// ## Returns
    /// The new CFG state after termination
    pub(crate) fn terminate_with_jump(&mut self, target: BasicBlockId) -> CfgState {
        self.terminate(Terminator::jump(target))
    }

    /// Terminates the current block with a conditional branch
    ///
    /// ## Arguments
    /// * `condition` - The condition value to test
    /// * `then_target` - The block to jump to if condition is true
    /// * `else_target` - The block to jump to if condition is false
    ///
    /// ## Returns
    /// The new CFG state after termination
    pub(crate) fn terminate_with_branch(
        &mut self,
        condition: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) -> CfgState {
        self.terminate(Terminator::branch(condition, then_target, else_target))
    }

    /// Terminates the current block with a return
    ///
    /// ## Arguments
    /// * `values` - The values to return
    ///
    /// ## Returns
    /// The new CFG state after termination
    pub(crate) fn terminate_with_return(&mut self, values: Vec<Value>) -> CfgState {
        self.terminate(Terminator::return_values(values))
    }

    /// Gets a mutable reference to a specific block by ID
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to get
    ///
    /// ## Returns
    /// An optional mutable reference to the block
    pub(crate) fn get_block_mut(&mut self, block_id: BasicBlockId) -> Option<&mut BasicBlock> {
        self.function.basic_blocks.get_mut(block_id)
    }

    /// Sets the terminator for a specific block
    /// Also updates edges when replacing an existing terminator
    ///
    /// This is useful when you need to patch up a block that was created earlier.
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to terminate
    /// * `terminator` - The terminator to set
    pub(crate) fn set_block_terminator(&mut self, block_id: BasicBlockId, terminator: Terminator) {
        self.set_terminator_internal(block_id, terminator);
    }

    /// Creates blocks for a loop pattern
    ///
    /// ## Returns
    /// A tuple of (header_block_id, body_block_id, exit_block_id)
    pub(crate) fn create_loop_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let header = self.new_block(Some("loop_header".to_string()));
        let body = self.new_block(Some("loop_body".to_string()));
        let exit = self.new_block(Some("loop_exit".to_string()));
        (header, body, exit)
    }

    /// Creates blocks for a for-loop pattern
    ///
    /// ## Returns
    /// A tuple of (header_block_id, body_block_id, step_block_id, exit_block_id)
    pub(crate) fn create_for_loop_blocks(
        &mut self,
    ) -> (BasicBlockId, BasicBlockId, BasicBlockId, BasicBlockId) {
        let header = self.new_block(Some("for_header".to_string()));
        let body = self.new_block(Some("for_body".to_string()));
        let step = self.new_block(Some("for_step".to_string()));
        let exit = self.new_block(Some("for_exit".to_string()));
        (header, body, step, exit)
    }

    /// Mark a block as filled (all local statements processed)
    /// This is used by SSA construction to track when a block is complete
    pub(crate) fn mark_block_filled(&mut self, block_id: BasicBlockId) {
        let block = self
            .get_block_mut(block_id)
            .unwrap_or_else(|| panic!("Block {:?} not found", block_id));
        block.mark_filled();
    }

    /// Mark a block as sealed (no more predecessors)
    /// This is used by SSA construction - when called, it means the predecessor set is final
    pub(crate) fn seal_block(&mut self, block_id: BasicBlockId) {
        let block = self
            .get_block_mut(block_id)
            .unwrap_or_else(|| panic!("Block {:?} not found", block_id));
        block.seal();
        // NOTE: SSA builder will also need to track sealed blocks in its own set
        // This method is just for marking the BasicBlock itself
    }
}
