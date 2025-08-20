//! # Instruction Builder
//!
//! This module provides a fluent API for creating MIR instructions.
//! It centralizes instruction creation logic and provides type-safe builders
//! for different instruction types.

use cairo_m_compiler_parser::parser::UnaryOp;

use crate::instruction::CalleeSignature;
use crate::{
    BasicBlockId, BinaryOp, FunctionId, Instruction, Literal, MirFunction, MirType, Value, ValueId,
};

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
    pub fn add_instruction(&mut self, instruction: Instruction) {
        let block = self
            .function
            .basic_blocks
            .get_mut(self.current_block)
            .unwrap_or_else(|| panic!("Block {:?} not found", self.current_block));
        block.push_instruction(instruction);
    }

    /// Create and add a binary operation instruction with explicit destination
    pub fn binary_op_to(
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

    /// Create and add a unary operation instruction with explicit destination
    pub fn unary_op_to(&mut self, op: UnaryOp, dest: ValueId, operand: Value) -> &mut Self {
        let instr = Instruction::unary_op(op, dest, operand);
        self.add_instruction(instr);
        self
    }

    /// Create and add a load with automatic destination
    pub fn load(&mut self, src: Value, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(ty.clone());
        let instr = Instruction::load(dest, ty, src);
        self.add_instruction(instr);
        dest
    }

    /// Create and add a load instruction with a comment
    pub fn load_with(
        &mut self,
        ty: MirType,
        dest: ValueId,
        src: Value,
        comment: String,
    ) -> &mut Self {
        let instr = Instruction::load(dest, ty, src).with_comment(comment);
        self.add_instruction(instr);
        self
    }

    /// Create and add a store instruction
    pub fn store(&mut self, dest: Value, value: Value, ty: MirType) -> &mut Self {
        let instr = Instruction::store(dest, value, ty);
        self.add_instruction(instr);
        self
    }

    /// Create and add a store instruction with a comment
    pub fn store_with(
        &mut self,
        dest: Value,
        value: Value,
        ty: MirType,
        comment: String,
    ) -> &mut Self {
        let instr = Instruction::store(dest, value, ty).with_comment(comment);
        self.add_instruction(instr);
        self
    }

    /// Create and add a call instruction with signature
    pub fn call_with(
        &mut self,
        dests: Vec<ValueId>,
        func_id: FunctionId,
        args: Vec<Value>,
        signature: CalleeSignature,
    ) -> &mut Self {
        let instr = Instruction::call(dests, func_id, args, signature);
        self.add_instruction(instr);
        self
    }

    /// Create and add an assignment from a literal value
    ///
    /// ## Arguments
    /// * `lit` - The literal value
    /// * `ty` - The type of the literal
    ///
    /// ## Returns
    /// The destination ValueId
    pub fn literal(&mut self, lit: Literal, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(ty.clone());
        let instr = Instruction::assign(dest, Value::Literal(lit), ty);
        self.add_instruction(instr);
        dest
    }

    /// Create and add a function call instruction
    ///
    /// ## Arguments
    /// * `callee` - The ID of the function to call
    /// * `args` - The arguments to pass
    /// * `return_types` - The types of the return values
    ///
    /// ## Returns
    /// Vec of destination ValueIds
    pub fn call(
        &mut self,
        callee: crate::FunctionId,
        args: Vec<Value>,
        return_types: Vec<MirType>,
    ) -> Vec<ValueId> {
        // Since we don't have param_types here, create a signature with empty param_types
        // The lowering code should use call_with_signature instead when it has full type info
        let signature = CalleeSignature {
            param_types: vec![],
            return_types: return_types.clone(),
        };

        let dests: Vec<ValueId> = return_types
            .iter()
            .map(|ty| self.function.new_typed_value_id(ty.clone()))
            .collect();

        let instr = Instruction::call(dests.clone(), callee, args, signature);
        self.add_instruction(instr);
        dests
    }

    /// Create and add a frame allocation with automatic destination
    pub fn alloc_frame(&mut self, ty: MirType) -> ValueId {
        let dest = self
            .function
            .new_typed_value_id(MirType::pointer(ty.clone()));
        let instr = Instruction::frame_alloc(dest, ty);
        self.add_instruction(instr);
        dest
    }

    /// Create and add a binary operation with automatic destination
    pub fn binary_op(
        &mut self,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
        result_type: MirType,
    ) -> ValueId {
        let dest = self.function.new_typed_value_id(result_type);
        let instr = Instruction::binary_op(op, dest, lhs, rhs);
        self.add_instruction(instr);
        dest
    }

    /// Create and add a unary operation with automatic destination
    pub fn unary_op(&mut self, op: UnaryOp, operand: Value, result_type: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(result_type);
        let instr = Instruction::unary_op(op, dest, operand);
        self.add_instruction(instr);
        dest
    }
}
