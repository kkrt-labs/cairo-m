//! # Instruction Builder
//!
//! This module provides a fluent API for creating MIR instructions.
//! It centralizes instruction creation logic and provides type-safe builders
//! for different instruction types.

use cairo_m_compiler_parser::parser::UnaryOp;

use crate::{BasicBlockId, BinaryOp, Instruction, MirFunction, MirType, Value, ValueId};

/// A builder for creating MIR instructions with a fluent API
///
/// The InstrBuilder manages instruction creation and automatically handles
/// destination allocation. It provides methods for each instruction type
/// that return both the instruction and destination for flexible use.
pub struct InstrBuilder<'f> {
    function: &'f mut MirFunction,
    current_block: BasicBlockId,
}

impl<'f> InstrBuilder<'f> {
    /// Creates a new instruction builder for the given function and current block
    pub const fn new(function: &'f mut MirFunction, current_block: BasicBlockId) -> Self {
        Self {
            function,
            current_block,
        }
    }

    /// Add an instruction to the current block
    pub(crate) fn add_instruction(&mut self, instruction: Instruction) {
        let block = self
            .function
            .basic_blocks
            .get_mut(self.current_block)
            .unwrap_or_else(|| panic!("Block {:?} not found", self.current_block));
        block.push_instruction(instruction);
    }

    /// Create and add a binary operation instruction with explicit destination
    pub(crate) fn binary_op_to(
        &mut self,
        op: BinaryOp,
        dest: ValueId,
        lhs: Value,
        rhs: Value,
    ) -> &mut Self {
        let instr = Instruction::binary_op(op, dest, lhs, rhs);
        self.add_instruction(instr);
        self
    }

    /// Create and add a unary operation with automatic destination
    pub(crate) fn unary_op(
        &mut self,
        op: UnaryOp,
        operand: Value,
        result_type: MirType,
    ) -> ValueId {
        let dest = self.function.new_typed_value_id(result_type);
        let instr = Instruction::unary_op(op, dest, operand);
        self.add_instruction(instr);
        dest
    }
}
