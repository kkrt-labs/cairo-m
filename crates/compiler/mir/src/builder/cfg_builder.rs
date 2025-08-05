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
    pub fn new_block(&mut self, name: Option<String>) -> BasicBlockId {
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
    pub fn current_block(&self) -> &BasicBlock {
        self.function
            .basic_blocks
            .get(self.current_block_id)
            .expect("Current block should exist")
    }

    /// Returns a mutable reference to the current block
    pub fn current_block_mut(&mut self) -> &mut BasicBlock {
        self.function
            .basic_blocks
            .get_mut(self.current_block_id)
            .expect("Current block should exist")
    }

    /// Checks if the current block is terminated
    pub fn is_terminated(&self) -> bool {
        self.is_terminated || self.current_block().is_terminated()
    }

    /// Terminates the current block with the given terminator
    ///
    /// ## Arguments
    /// * `terminator` - The terminator to set
    ///
    /// ## Panics
    /// Panics if the block is already terminated
    pub fn terminate(&mut self, terminator: Terminator) {
        if self.is_terminated() {
            panic!("Attempting to terminate an already terminated block");
        }

        let block = self.current_block_mut();
        block.set_terminator(terminator);
        self.is_terminated = true;
    }

    /// Terminates the current block with a jump to the target block
    ///
    /// ## Arguments
    /// * `target` - The block to jump to
    pub fn terminate_with_jump(&mut self, target: BasicBlockId) {
        self.terminate(Terminator::jump(target));
    }

    /// Terminates the current block with a conditional branch
    ///
    /// ## Arguments
    /// * `condition` - The condition value to test
    /// * `then_target` - The block to jump to if condition is true
    /// * `else_target` - The block to jump to if condition is false
    pub fn terminate_with_branch(
        &mut self,
        condition: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) {
        self.terminate(Terminator::branch(condition, then_target, else_target));
    }

    /// Terminates the current block with a return
    ///
    /// ## Arguments
    /// * `values` - The values to return
    pub fn terminate_with_return(&mut self, values: Vec<Value>) {
        self.terminate(Terminator::return_values(values));
    }

    /// Creates a new block and switches to it
    ///
    /// This is a convenience method that combines new_block and switch_to_block.
    ///
    /// ## Arguments
    /// * `name` - Optional name for the new block
    ///
    /// ## Returns
    /// The ID of the newly created block
    pub fn create_and_switch_to_block(&mut self, name: Option<String>) -> BasicBlockId {
        let block_id = self.new_block(name);
        self.switch_to_block(block_id);
        block_id
    }

    /// Terminates the current block with a jump and switches to the target
    ///
    /// This is a common pattern in control flow construction.
    ///
    /// ## Arguments
    /// * `target` - The block to jump to and switch to
    pub fn jump_to(&mut self, target: BasicBlockId) {
        if !self.is_terminated() {
            self.terminate_with_jump(target);
        }
        self.switch_to_block(target);
    }

    /// Gets a reference to a specific block by ID
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to get
    ///
    /// ## Returns
    /// An optional reference to the block
    pub fn get_block(&self, block_id: BasicBlockId) -> Option<&BasicBlock> {
        self.function.basic_blocks.get(block_id)
    }

    /// Gets a mutable reference to a specific block by ID
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to get
    ///
    /// ## Returns
    /// An optional mutable reference to the block
    pub fn get_block_mut(&mut self, block_id: BasicBlockId) -> Option<&mut BasicBlock> {
        self.function.basic_blocks.get_mut(block_id)
    }

    /// Sets the terminator for a specific block
    ///
    /// This is useful when you need to patch up a block that was created earlier.
    ///
    /// ## Arguments
    /// * `block_id` - The ID of the block to terminate
    /// * `terminator` - The terminator to set
    pub fn set_block_terminator(&mut self, block_id: BasicBlockId, terminator: Terminator) {
        if let Some(block) = self.get_block_mut(block_id) {
            block.set_terminator(terminator);
        }
    }

    /// Creates blocks for an if-then-else pattern
    ///
    /// ## Returns
    /// A tuple of (then_block_id, else_block_id, merge_block_id)
    pub fn create_if_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let then_block = self.new_block(Some("then".to_string()));
        let else_block = self.new_block(Some("else".to_string()));
        let merge_block = self.new_block(Some("merge".to_string()));
        (then_block, else_block, merge_block)
    }

    /// Creates blocks for a loop pattern
    ///
    /// ## Returns
    /// A tuple of (header_block_id, body_block_id, exit_block_id)
    pub fn create_loop_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        let header = self.new_block(Some("loop_header".to_string()));
        let body = self.new_block(Some("loop_body".to_string()));
        let exit = self.new_block(Some("loop_exit".to_string()));
        (header, body, exit)
    }

    /// Creates blocks for a for-loop pattern
    ///
    /// ## Returns
    /// A tuple of (header_block_id, body_block_id, step_block_id, exit_block_id)
    pub fn create_for_loop_blocks(
        &mut self,
    ) -> (BasicBlockId, BasicBlockId, BasicBlockId, BasicBlockId) {
        let header = self.new_block(Some("for_header".to_string()));
        let body = self.new_block(Some("for_body".to_string()));
        let step = self.new_block(Some("for_step".to_string()));
        let exit = self.new_block(Some("for_exit".to_string()));
        (header, body, step, exit)
    }
}
