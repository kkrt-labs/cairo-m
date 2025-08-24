//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.

use cairo_m_common::instruction::*;
use cairo_m_compiler_mir::instruction::CalleeSignature;
use cairo_m_compiler_mir::{BinaryOp, DataLayout, Literal, MirType, Value, ValueId};
use cairo_m_compiler_parser::parser::UnaryOp;
use stwo_prover::core::fields::m31::M31;

use crate::layout::ValueLayout;
use crate::{CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label, Operand};

/// Helper to split a u32 value into low and high 16-bit parts
#[inline]
const fn split_u32_value(value: u32) -> (i32, i32) {
    ((value & 0xFFFF) as i32, ((value >> 16) & 0xFFFF) as i32)
}

/// Helper to split an i32 value (interpreted as u32) into low and high 16-bit parts
#[inline]
const fn split_u32_i32(value: i32) -> (i32, i32) {
    let u = value as u32;
    ((u & 0xFFFF) as i32, ((u >> 16) & 0xFFFF) as i32)
}

/// Builder for generating CASM instructions
///
/// This struct manages the generation of CASM instructions, handling the
/// translation from MIR values to fp-relative memory addresses.
#[derive(Debug)]
pub struct CasmBuilder {
    /// Generated instructions
    instructions: Vec<InstructionBuilder>,
    /// Labels that need to be resolved
    labels: Vec<Label>,
    /// Current function layout for offset lookups
    layout: FunctionLayout,
    /// Counter for generating unique labels
    label_counter: usize,
    /// Highest fp+ offset that has been written to (for optimization tracking)
    max_written_offset: i32,
}

/// Represents the type of array operation to perform
pub enum ArrayOperation {
    /// Load an element from array into dest
    Load { dest: ValueId },
    /// Store a value into an array element, creating a new array in dest
    Store { dest: ValueId, value: Value },
}

impl CasmBuilder {
    /// Create a new CASM builder with the required layout
    pub const fn new(layout: FunctionLayout, label_counter: usize) -> Self {
        // Initialize max_written_offset based on pre-allocated layout
        // For tests and scenarios with pre-allocated values, we assume
        // all slots up to frame_size are "potentially written"
        let max_written_offset = if layout.frame_size > 0 {
            layout.frame_size as i32 - 1
        } else {
            -1
        };

        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            layout,
            label_counter,
            max_written_offset,
        }
    }

    pub fn new_label_name(&mut self, prefix: &str) -> String {
        let label_id = self.label_counter;
        self.label_counter += 1;
        format!("{}_{}", prefix, label_id)
    }

    /// Add a label at the current position
    pub fn add_label(&mut self, label: Label) {
        let mut label = label;
        label.address = Some(self.instructions.len());
        self.labels.push(label);
    }

    /// Track that we've written to a memory location
    /// Updates the high-water mark for written offsets
    fn touch(&mut self, offset: i32, size: usize) {
        // Only track positive offsets (locals/temporaries)
        if offset >= 0 {
            let end_offset = offset + size as i32 - 1;
            self.max_written_offset = self.max_written_offset.max(end_offset);
        }
    }

    /// Get the current "live" frame usage based on what's actually been written
    pub const fn live_frame_usage(&self) -> i32 {
        self.max_written_offset + 1 // Convert from 0-based offset to size
    }

    /// Get the current pre-allocated frame usage
    pub const fn current_frame_usage(&self) -> i32 {
        self.layout.current_frame_usage()
    }

    /// Helper to emit a store immediate instruction and track the write
    fn store_immediate(&mut self, value: u32, offset: i32, comment: String) {
        let instr = InstructionBuilder::new(STORE_IMM)
            .with_operand(Operand::Literal(value as i32))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.instructions.push(instr);
        self.touch(offset, 1);
    }

    /// Helper to emit a u32 store immediate instruction and track the write
    fn store_u32_immediate(&mut self, value: u32, offset: i32, comment: String) {
        // Split the u32 value into low and high 16-bit parts
        let (lo, hi) = split_u32_value(value);

        let instr = InstructionBuilder::new(U32_STORE_IMM)
            .with_operand(Operand::Literal(lo))
            .with_operand(Operand::Literal(hi))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.instructions.push(instr);
        self.touch(offset, 2); // u32 takes 2 slots
    }

    /// Store immediate value at a specific offset (public version)
    pub fn store_immediate_at(
        &mut self,
        value: u32,
        offset: i32,
        comment: String,
    ) -> CodegenResult<()> {
        self.store_immediate(value, offset, comment);
        Ok(())
    }

    /// Store u32 immediate value at a specific offset (public version)
    pub fn store_u32_immediate_at(
        &mut self,
        value: u32,
        offset: i32,
        comment: String,
    ) -> CodegenResult<()> {
        self.store_u32_immediate(value, offset, comment);
        Ok(())
    }

    /// Store a value at a specific offset
    ///
    /// The dest_id is used to map the destination in the layout, but the size
    /// is determined by the value being stored, not the destination.
    pub fn store_at(&mut self, dest_id: ValueId, offset: i32, value: Value) -> CodegenResult<()> {
        // Map the destination to the specific offset
        self.layout.map_value(dest_id, offset);

        match value {
            Value::Literal(Literal::Integer(imm)) => {
                self.store_immediate(imm, offset, format!("[fp + {}] = {}", offset, imm));
            }
            Value::Operand(src_id) => {
                let src_off = self.layout.get_offset(src_id)?;
                // Get the size of the value being stored
                let size = self.layout.get_value_size(src_id);

                // For multi-slot values, copy each slot
                for i in 0..size {
                    let slot_src_off = src_off + i as i32;
                    let slot_dest_off = offset + i as i32;
                    let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(slot_src_off))
                        .with_operand(Operand::Literal(0))
                        .with_operand(Operand::Literal(slot_dest_off))
                        .with_comment(format!(
                            "[fp + {}] = [fp + {}] + 0",
                            slot_dest_off, slot_src_off
                        ));
                    self.instructions.push(instr);
                }
                self.touch(offset, size);
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported store value type: {:?}",
                    value
                )));
            }
        }
        Ok(())
    }

    /// Store u32 value at a specific offset
    pub fn store_u32_at(
        &mut self,
        dest_id: ValueId,
        offset: i32,
        value: Value,
    ) -> CodegenResult<()> {
        self.layout.map_value(dest_id, offset);

        match value {
            Value::Literal(Literal::Integer(imm)) => {
                let value = imm as u32;
                self.store_u32_immediate(
                    value,
                    offset,
                    format!("u32([fp + {}, fp + {}]) = u32({})", offset, offset + 1, imm),
                );
            }
            Value::Operand(src_id) => {
                let src_off = self.layout.get_offset(src_id)?;
                let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(offset))
                    .with_comment(format!(
                        "u32([fp + {}], [fp + {}]) = u32([fp + {}], [fp + {}]) + u32(0, 0)",
                        offset,
                        offset + 1,
                        src_off,
                        src_off + 1
                    ));
                self.instructions.push(instr);
                self.touch(offset, 2);
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported store value type: {:?}",
                    value
                )));
            }
        }

        Ok(())
    }

    /// Generate assignment instruction with optional target offset
    ///
    /// If target_offset is provided, writes directly to that location.
    /// Otherwise, allocates a new local variable.
    pub fn assign_with_target(
        &mut self,
        dest: ValueId,
        source: Value,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout, or allocate on demand
            match self.layout.get_offset(dest) {
                Ok(offset) => offset,
                Err(_) => {
                    // Value wasn't pre-allocated (likely an immediate assignment from SSA form)
                    // Allocate it now
                    self.layout.allocate_local(dest, 1)?
                }
            }
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Store immediate value
                self.store_immediate(imm, dest_off, format!("[fp + {dest_off}] = {imm}"));
                self.touch(dest_off, 1);
            }
            Value::Literal(Literal::Boolean(b)) => {
                // Store immediate value
                self.store_immediate(b as u32, dest_off, format!("[fp + {dest_off}] = {b}"));
                self.touch(dest_off, 1);
            }

            Value::Operand(src_id) => {
                // Copy from another value using StoreAddFpImm with imm=0
                let src_off = self.layout.get_offset(src_id)?;

                let instr = InstructionBuilder::new(STORE_ADD_FP_IMM) // StoreAddFpImm
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!("[fp + {dest_off}] = [fp + {src_off}] + 0"));

                self.instructions.push(instr);
                self.touch(dest_off, 1);
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported assignment source: {:?}",
                    source
                )));
            }
        }

        Ok(())
    }

    /// Generate assignment instruction
    ///
    /// Handles simple value assignments: dest = source
    pub fn assign(&mut self, dest: ValueId, source: Value) -> CodegenResult<()> {
        self.assign_with_target(dest, source, None)
    }

    /// Generate type-aware assignment instruction
    ///
    /// Handles assignments for all types including aggregates (structs, tuples)
    pub fn assign_typed(
        &mut self,
        dest: ValueId,
        source: Value,
        ty: &MirType,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // DataLayout methods are now static - no instance needed
        // Arrays are stored as pointers (1 slot) in the codegen
        let size = match ty {
            MirType::FixedArray { .. } => 1,
            _ => DataLayout::size_of(ty),
        };

        // Determine destination offset
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout, or allocate on demand
            match self.layout.get_offset(dest) {
                Ok(offset) => offset,
                Err(_) => {
                    // Value wasn't pre-allocated, allocate it now
                    // Arrays are stored as pointers (1 slot)
                    let alloc_size = match ty {
                        MirType::FixedArray { .. } => 1,
                        _ => size,
                    };
                    self.layout.allocate_local(dest, alloc_size)?
                }
            }
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Handle immediate values based on size
                if size == 1 {
                    // Single slot value (felt, bool, pointer)
                    self.store_immediate(imm, dest_off, format!("[fp + {dest_off}] = {imm}"));
                } else if size == 2 && matches!(ty, MirType::U32) {
                    // U32 value
                    let value = imm;
                    self.store_u32_immediate(
                        value,
                        dest_off,
                        format!(
                            "u32([fp + {dest_off}], [fp + {}]) = u32({value})",
                            dest_off + 1
                        ),
                    );
                } else {
                    return Err(CodegenError::UnsupportedInstruction(format!(
                        "Cannot assign immediate to aggregate type of size {}",
                        size
                    )));
                }
            }

            Value::Literal(Literal::Boolean(b)) => {
                if size != 1 {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Boolean literal must be single-slot".to_string(),
                    ));
                }
                self.store_immediate(b as u32, dest_off, format!("[fp + {dest_off}] = {b}"));
            }

            Value::Operand(src_id) => {
                // Copy from another value
                let src_off = self.layout.get_offset(src_id)?;

                // Special handling for arrays: only copy the pointer (first slot)
                if matches!(ty, MirType::FixedArray { .. }) {
                    // Arrays are stored as pointers - only copy the pointer value (first slot)
                    let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(src_off))
                        .with_operand(Operand::Literal(0))
                        .with_operand(Operand::Literal(dest_off))
                        .with_comment(format!(
                            "[fp + {dest_off}] = [fp + {src_off}] + 0 (array pointer)"
                        ));
                    self.instructions.push(instr);
                    self.touch(dest_off, 1);
                } else if size == 1 {
                    // Single slot copy
                    let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(src_off))
                        .with_operand(Operand::Literal(0))
                        .with_operand(Operand::Literal(dest_off))
                        .with_comment(format!("[fp + {dest_off}] = [fp + {src_off}] + 0"));
                    self.instructions.push(instr);
                    self.touch(dest_off, 1);
                } else if size == 2 && matches!(ty, MirType::U32) {
                    // U32 copy using dedicated instruction
                    let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(src_off))
                        .with_operand(Operand::Literal(0))  // imm_lo
                        .with_operand(Operand::Literal(0))  // imm_hi
                        .with_operand(Operand::Literal(dest_off))
                        .with_comment(format!(
                            "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {src_off}], [fp + {}]) + u32(0, 0)",
                            dest_off + 1, src_off + 1
                        ));
                    self.instructions.push(instr);
                    self.touch(dest_off, 2);
                } else {
                    // Multi-slot copy for aggregates (structs, tuples, etc.)
                    // Copy each slot individually
                    for i in 0..size {
                        let slot_src = src_off + i as i32;
                        let slot_dst = dest_off + i as i32;

                        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                            .with_operand(Operand::Literal(slot_src))
                            .with_operand(Operand::Literal(0))
                            .with_operand(Operand::Literal(slot_dst))
                            .with_comment(format!(
                                "[fp + {}] = [fp + {}] + 0 (slot {} of {})",
                                slot_dst,
                                slot_src,
                                i + 1,
                                size
                            ));
                        self.instructions.push(instr);
                    }
                    self.touch(dest_off, size);
                }
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported assignment source: {:?}",
                    source
                )));
            }
        }

        Ok(())
    }

    /// Generate u32 assignment instruction with optional target offset
    ///
    /// If target_offset is provided, writes directly to that location.
    /// Otherwise, allocates a new local variable.
    pub fn assign_u32_with_target(
        &mut self,
        dest: ValueId,
        source: Value,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout, or allocate on demand
            match self.layout.get_offset(dest) {
                Ok(offset) => offset,
                Err(_) => {
                    // Value wasn't pre-allocated (likely an immediate assignment from SSA form)
                    // Allocate it now (U32 needs 2 slots)
                    self.layout.allocate_local(dest, 2)?
                }
            }
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Store as u32 immediate
                let value = imm as u32;
                self.store_u32_immediate(
                    value,
                    dest_off,
                    format!(
                        "u32([fp + {dest_off}, fp + {}]) = u32({value})",
                        dest_off + 1
                    ),
                );
            }

            Value::Operand(src_id) => {
                // Copy from another value using U32_STORE_ADD_FP_IMM with zero immediates
                let src_off = self.layout.get_offset(src_id)?;

                let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!(
                        "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {src_off}], [fp + {}]) + u32(0, 0)",
                        dest_off + 1, src_off + 1
                    ));
                self.instructions.push(instr);

                self.touch(dest_off, 2);
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported assignment source for u32".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn unary_op(&mut self, op: UnaryOp, dest: ValueId, source: Value) -> CodegenResult<()> {
        self.unary_op_with_target(op, dest, source, None)
    }

    pub fn unary_op_with_target(
        &mut self,
        op: UnaryOp,
        dest: ValueId,
        source: Value,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout
            self.layout.get_offset(dest)?
        };

        match op {
            UnaryOp::Neg => {
                self.generate_arithmetic_op(
                    BinaryOp::Sub,
                    dest_off,
                    Value::Literal(Literal::Integer(0)),
                    source,
                )?;
            }
            UnaryOp::Not => {
                self.generate_not_op(dest_off, source)?;
            }
        }
        self.touch(dest_off, 1);
        Ok(())
    }

    /// Generate binary operation instruction with optional target offset
    ///
    /// If target_offset is provided, writes directly to that location.
    /// Otherwise, allocates a new local variable.
    pub fn binary_op_with_target(
        &mut self,
        op: BinaryOp,
        dest: ValueId,
        left: Value,
        right: Value,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout
            self.layout.get_offset(dest)?
        };

        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                self.generate_arithmetic_op(op, dest_off, left, right)?;
            }
            BinaryOp::Eq => self.generate_equals_op(dest_off, left, right)?,
            BinaryOp::Neq => self.generate_neq_op(dest_off, left, right)?,
            BinaryOp::And => {
                self.generate_and_op(dest_off, left, right)?;
            }
            BinaryOp::Or => {
                self.generate_or_op(dest_off, left, right)?;
            }
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                todo!("Comparison opcodes not supported on felt type");
            }
            BinaryOp::U32Add
            | BinaryOp::U32Sub
            | BinaryOp::U32Mul
            | BinaryOp::U32Div
            | BinaryOp::U32Eq
            | BinaryOp::U32Neq
            | BinaryOp::U32Less
            | BinaryOp::U32Greater
            | BinaryOp::U32LessEqual
            | BinaryOp::U32GreaterEqual => {
                self.generate_u32_op(op, dest_off, left, right)?;
            }
        }

        Ok(())
    }

    /// Generate a binary operation instruction
    ///
    /// Handles all arithmetic and comparison operations needed for fibonacci:
    /// - Addition: a + b
    /// - Subtraction: a - b
    /// - Equality: a == b
    pub fn binary_op(
        &mut self,
        op: BinaryOp,
        dest: ValueId,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        self.binary_op_with_target(op, dest, left, right, None)
    }

    pub fn fp_fp_opcode_for_binary_op(&mut self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::Add => Ok(STORE_ADD_FP_FP),
            BinaryOp::Sub => Ok(STORE_SUB_FP_FP),
            BinaryOp::Mul => Ok(STORE_MUL_FP_FP),
            BinaryOp::Div => Ok(STORE_DIV_FP_FP),
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Invalid binary operation: {op:?}"
            ))),
        }
    }

    pub fn fp_imm_opcode_for_binary_op(&mut self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::Add => Ok(STORE_ADD_FP_IMM),
            BinaryOp::Sub => Ok(STORE_SUB_FP_IMM),
            BinaryOp::Mul => Ok(STORE_MUL_FP_IMM),
            BinaryOp::Div => Ok(STORE_DIV_FP_IMM),
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Invalid binary operation: {op:?}"
            ))),
        }
    }

    pub fn fp_fp_opcode_for_u32_op(&mut self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::U32Add => Ok(U32_STORE_ADD_FP_FP),
            BinaryOp::U32Sub => Ok(U32_STORE_SUB_FP_FP),
            BinaryOp::U32Mul => Ok(U32_STORE_MUL_FP_FP),
            BinaryOp::U32Div => Ok(U32_STORE_DIV_FP_FP),
            BinaryOp::U32Eq => Ok(U32_STORE_EQ_FP_FP),
            BinaryOp::U32Neq => Ok(U32_STORE_NEQ_FP_FP),
            BinaryOp::U32Greater => Ok(U32_STORE_GT_FP_FP),
            BinaryOp::U32GreaterEqual => Ok(U32_STORE_GE_FP_FP),
            BinaryOp::U32Less => Ok(U32_STORE_LT_FP_FP),
            BinaryOp::U32LessEqual => Ok(U32_STORE_LE_FP_FP),
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Invalid binary operation: {op:?}"
            ))),
        }
    }

    pub fn fp_imm_opcode_for_u32_op(&mut self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::U32Add => Ok(U32_STORE_ADD_FP_IMM),
            BinaryOp::U32Sub => Ok(U32_STORE_SUB_FP_IMM),
            BinaryOp::U32Mul => Ok(U32_STORE_MUL_FP_IMM),
            BinaryOp::U32Div => Ok(U32_STORE_DIV_FP_IMM),
            BinaryOp::U32Eq => Ok(U32_STORE_EQ_FP_IMM),
            BinaryOp::U32Neq => Ok(U32_STORE_NEQ_FP_IMM),
            BinaryOp::U32Greater => Ok(U32_STORE_GT_FP_IMM),
            BinaryOp::U32GreaterEqual => Ok(U32_STORE_GE_FP_IMM),
            BinaryOp::U32Less => Ok(U32_STORE_LT_FP_IMM),
            BinaryOp::U32LessEqual => Ok(U32_STORE_LE_FP_IMM),
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Invalid binary operation: {op:?}"
            ))),
        }
    }

    /// Generate arithmetic operation (add, sub, mul, div)
    pub fn generate_arithmetic_op(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        match (&left, &right) {
            // Both operands are values: use fp_fp variant
            (Value::Operand(left_id), Value::Operand(right_id)) => {
                let left_off = self.layout.get_offset(*left_id)?;
                let right_off = self.layout.get_offset(*right_id)?;

                let instr = InstructionBuilder::new(self.fp_fp_opcode_for_binary_op(op)?)
                    .with_operand(Operand::Literal(left_off))
                    .with_operand(Operand::Literal(right_off))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!(
                        "[fp + {dest_off}] = [fp + {left_off}] op [fp + {right_off}]"
                    ));

                self.instructions.push(instr);
                self.touch(dest_off, 1);
            }

            // Left is value, right is immediate: use fp_imm variant
            (Value::Operand(left_id), Value::Literal(Literal::Integer(imm))) => {
                let left_off = self.layout.get_offset(*left_id)?;

                let instr = InstructionBuilder::new(self.fp_imm_opcode_for_binary_op(op)?)
                    .with_operand(Operand::Literal(left_off))
                    .with_operand(Operand::Literal(*imm as i32))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!("[fp + {dest_off}] = [fp + {left_off}] op {imm}"));

                self.instructions.push(instr);
                self.touch(dest_off, 1);
            }

            // Left is immediate, right is value: use fp_imm variant
            (Value::Literal(Literal::Integer(imm)), Value::Operand(right_id)) => {
                match op {
                    // For addition and multiplication, we can swap the operands
                    BinaryOp::Add | BinaryOp::Mul => {
                        let right_off = self.layout.get_offset(*right_id)?;
                        let instr = InstructionBuilder::new(self.fp_imm_opcode_for_binary_op(op)?)
                            .with_operand(Operand::Literal(right_off))
                            .with_operand(Operand::Literal(*imm as i32))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "[fp + {dest_off}] = [fp + {right_off}] op {imm}"
                            ));
                        self.instructions.push(instr);
                        self.touch(dest_off, 1);
                    }
                    // For subtraction and division, we store the immediate in a temporary variable
                    // TODO: In the future we should add opcodes imm_fp_sub and imm_fp_div
                    BinaryOp::Sub | BinaryOp::Div => {
                        let right_off = self.layout.get_offset(*right_id)?;
                        // Allocate a new temporary slot for the immediate
                        let temp_off = self.layout.reserve_stack(1);

                        self.store_immediate(*imm, temp_off, format!("[fp + {temp_off}] = {imm}"));

                        let instr = InstructionBuilder::new(self.fp_fp_opcode_for_binary_op(op)?)
                            .with_operand(Operand::Literal(temp_off))
                            .with_operand(Operand::Literal(right_off))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "[fp + {dest_off}] = [fp + {temp_off}] op [fp + {right_off}]"
                            ));
                        self.instructions.push(instr);
                        self.touch(dest_off, 1);
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported operation".to_string(),
                        ));
                    }
                }
            }

            // Both operands are immediate: fold constants
            // This is a workaround for the fact that we don't have a constant folding pass yet.
            (Value::Literal(Literal::Integer(imm)), Value::Literal(Literal::Integer(imm2))) => {
                let result = match op {
                    BinaryOp::Add => (M31::from(*imm) + M31::from(*imm2)).0,
                    BinaryOp::Sub => (M31::from(*imm) - M31::from(*imm2)).0,
                    BinaryOp::Mul => (M31::from(*imm) * M31::from(*imm2)).0,
                    BinaryOp::Div => {
                        if *imm2 == 0 {
                            return Err(CodegenError::InvalidMir(
                                "Division by zero in felt constant folding".to_string(),
                            ));
                        }
                        (M31::from(*imm) / M31::from(*imm2)).0
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported operation".to_string(),
                        ));
                    }
                };

                self.store_immediate(result, dest_off, format!("[fp + {dest_off}] = {result}"));
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported operation: {left:?} {op:?} {right:?}"
                )));
            }
        }

        Ok(())
    }

    pub fn generate_equals_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Step 1: Compute left - right into dest
        self.generate_arithmetic_op(BinaryOp::Sub, dest_off, left, right)?;

        // Step 2: Generate unique labels for this equality check
        let not_zero_label = self.new_label_name("not_zero");
        let end_label = self.new_label_name("end");

        // Step 3: Check if temp == 0 (meaning left == right)
        // jnz jumps if non-zero, so if temp != 0, jump to set result to 0 (or 1 if not equal)
        let jnz_instr = InstructionBuilder::new(JNZ_FP_IMM)
            .with_operand(Operand::Literal(dest_off))
            .with_operand(Operand::Label(not_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {not_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so left == right, set result to 1
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: not_equal label - set result to 0
        let not_equal_label_obj = Label::new(not_zero_label);
        self.add_label(not_equal_label_obj);

        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));

        // Step 6: end label
        let end_label_obj = Label::new(end_label);
        self.add_label(end_label_obj);

        Ok(())
    }

    pub fn generate_neq_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Step 1: Compute left - right into dest
        self.generate_arithmetic_op(BinaryOp::Sub, dest_off, left, right)?;

        // Step 2: Generate unique labels for this NOT-EQUAL operation
        let non_zero_label = self.new_label_name("neq_non_zero");
        let end_label = self.new_label_name("neq_end");

        // Step 3: Check if temp != 0 (meaning left != right)
        // jnz jumps if non-zero, so if temp != 0, jump to set result to 1
        let jnz_instr = InstructionBuilder::new(JNZ_FP_IMM)
            .with_operand(Operand::Literal(dest_off))
            .with_operand(Operand::Label(non_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {non_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so left == right, set result to 0
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: non_zero label - set result to 1
        let non_zero_label_obj = Label::new(non_zero_label);
        self.add_label(non_zero_label_obj);

        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // Step 6: end label
        let end_label_obj = Label::new(end_label);
        self.add_label(end_label_obj);

        Ok(())
    }

    pub fn generate_and_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Step 1: Compute left * right into dest
        self.generate_arithmetic_op(BinaryOp::Mul, dest_off, left, right)?;

        // Step 2: Generate unique labels for this AND operation
        let non_zero_label = self.new_label_name("and_non_zero");
        let end_label = self.new_label_name("and_end");

        // Step 3: Check if temp != 0 (meaning both operands were non-zero)
        // jnz jumps if non-zero, so if temp != 0, jump to set result to 1
        let jnz_instr = InstructionBuilder::new(JNZ_FP_IMM)
            .with_operand(Operand::Literal(dest_off))
            .with_operand(Operand::Label(non_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {non_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so at least one operand was 0, set result to 0
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: non_zero label - set result to 1
        let non_zero_label_obj = Label::new(non_zero_label);
        self.add_label(non_zero_label_obj);

        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // Step 6: end label
        let end_label_obj = Label::new(end_label);
        self.add_label(end_label_obj);

        Ok(())
    }

    pub fn generate_or_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Generate unique labels
        let set_true_label = self.new_label_name("or_true");
        let end_label = self.new_label_name("or_end");

        // Step 1: Initialize result to 0
        self.store_immediate(0, dest_off, "Initialize OR result to 0".to_string());

        // Step 2: Check left operand - if non-zero, jump to set result to 1
        match left {
            Value::Operand(left_id) => {
                let left_off = self.layout.get_offset(left_id)?;
                let jnz_left = InstructionBuilder::new(JNZ_FP_IMM)
                    .with_operand(Operand::Literal(left_off))
                    .with_operand(Operand::Label(set_true_label.clone()))
                    .with_comment(format!(
                        "if [fp + {left_off}] != 0, jump to {set_true_label}"
                    ));
                self.instructions.push(jnz_left);
            }
            Value::Literal(Literal::Integer(imm)) => {
                // If left is a non-zero immediate, directly jump to set true
                if imm != 0 {
                    let jmp_true = InstructionBuilder::new(JMP_ABS_IMM)
                        .with_operand(Operand::Label(set_true_label.clone()))
                        .with_comment(format!("jump to {set_true_label}"));
                    self.instructions.push(jmp_true);
                }
                // If left is 0, continue to check right
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported left operand in OR".to_string(),
                ));
            }
        }

        // Step 3: Check right operand - if non-zero, jump to set result to 1
        match right {
            Value::Operand(right_id) => {
                let right_off = self.layout.get_offset(right_id)?;
                let jnz_right = InstructionBuilder::new(JNZ_FP_IMM)
                    .with_operand(Operand::Literal(right_off))
                    .with_operand(Operand::Label(set_true_label.clone()))
                    .with_comment(format!(
                        "if [fp + {right_off}] != 0, jump to {set_true_label}"
                    ));
                self.instructions.push(jnz_right);
            }
            Value::Literal(Literal::Integer(imm)) => {
                // If right is a non-zero immediate, directly jump to set true
                if imm != 0 {
                    let jmp_true = InstructionBuilder::new(JMP_ABS_IMM)
                        .with_operand(Operand::Label(set_true_label.clone()))
                        .with_comment(format!("jump to {set_true_label}"));
                    self.instructions.push(jmp_true);
                }
                // If right is 0, fall through to end (result stays 0)
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported right operand in OR".to_string(),
                ));
            }
        }

        // Step 4: Jump to end (both operands were 0, result stays 0)
        let jmp_end = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end);

        // Step 5: set_true label - set result to 1
        self.add_label(Label::new(set_true_label));
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // Step 6: end label
        self.add_label(Label::new(end_label));

        Ok(())
    }

    pub fn generate_not_op(&mut self, dest_off: i32, source: Value) -> CodegenResult<()> {
        let set_zero_label = self.new_label_name("not_zero");
        let end_label = self.new_label_name("not_end");

        match source {
            Value::Operand(src_id) => {
                let src_off = self.layout.get_offset(src_id)?;
                // If source is non-zero, jump to set result to 0
                let jnz_instr = InstructionBuilder::new(JNZ_FP_IMM)
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Label(set_zero_label.clone()))
                    .with_comment(format!(
                        "if [fp + {src_off}] != 0, jump to {set_zero_label}"
                    ));
                self.instructions.push(jnz_instr);
            }
            Value::Literal(Literal::Boolean(imm)) => {
                // For immediate values, we can directly compute the NOT result
                self.store_immediate(
                    !imm as u32,
                    dest_off,
                    format!("[fp + {dest_off}] = {}", !imm),
                );
                return Ok(());
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported source operand in NOT: {:?}",
                    source
                )));
            }
        }

        // If we reach here, source was 0, so set result to 1
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // set_zero label - set result to 0
        self.add_label(Label::new(set_zero_label));
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));

        // end label
        self.add_label(Label::new(end_label));

        Ok(())
    }

    /// Generate U32 operation
    pub fn generate_u32_op(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Determine if this is a comparison operation (returns bool / felt) or arithmetic (returns u32)
        let is_comparison = matches!(
            op,
            BinaryOp::U32Eq
                | BinaryOp::U32Neq
                | BinaryOp::U32Greater
                | BinaryOp::U32GreaterEqual
                | BinaryOp::U32Less
                | BinaryOp::U32LessEqual
        );
        let result_size = if is_comparison { 1 } else { 2 };

        match (&left, &right) {
            (Value::Operand(left_id), Value::Operand(right_id)) => {
                let left_off = self.layout.get_offset(*left_id)?;
                let right_off = self.layout.get_offset(*right_id)?;

                let comment = if is_comparison {
                    format!(
                        "[fp + {dest_off}] = u32([fp + {left_off}], [fp + {}]) {op} u32([fp + {right_off}], [fp + {}])",
                        left_off + 1,
                        right_off + 1
                    )
                } else {
                    format!(
                        "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {left_off}], [fp + {}]) {op} u32([fp + {right_off}], [fp + {}])",
                        dest_off + 1,
                        left_off + 1,
                        right_off + 1
                    )
                };

                let instr = InstructionBuilder::new(self.fp_fp_opcode_for_u32_op(op)?)
                    .with_operand(Operand::Literal(left_off))
                    .with_operand(Operand::Literal(right_off))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(comment);

                self.instructions.push(instr);
                self.touch(dest_off, result_size);
            }

            // Left is value, right is immediate: use fp_imm variant
            (Value::Operand(left_id), Value::Literal(Literal::Integer(imm))) => {
                let left_off = self.layout.get_offset(*left_id)?;

                let (imm_16b_low, imm_16b_high) = split_u32_i32(*imm as i32);

                let comment = if is_comparison {
                    format!(
                        "[fp + {dest_off}] = u32([fp + {left_off}], [fp + {}]) {op} u32({imm_16b_low}, {imm_16b_high})",
                        left_off + 1
                    )
                } else {
                    format!(
                        "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {left_off}], [fp + {}]) {op} u32({imm_16b_low}, {imm_16b_high})",
                        dest_off + 1,
                        left_off + 1
                    )
                };

                // Use immediate versions
                let instr = InstructionBuilder::new(self.fp_imm_opcode_for_u32_op(op)?)
                    .with_operand(Operand::Literal(left_off))
                    .with_operand(Operand::Literal(imm_16b_low))
                    .with_operand(Operand::Literal(imm_16b_high))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(comment);
                self.instructions.push(instr);
                self.touch(dest_off, result_size);
            }

            // Left is immediate, right is value: use fp_imm variant
            (Value::Literal(Literal::Integer(imm)), Value::Operand(right_id)) => {
                let (imm_16b_low, imm_16b_high) = split_u32_i32(*imm as i32);

                match op {
                    // For addition and multiplication, we can swap the operands
                    BinaryOp::U32Add | BinaryOp::U32Mul => {
                        let right_off = self.layout.get_offset(*right_id)?;
                        let instr = InstructionBuilder::new(self.fp_imm_opcode_for_u32_op(op)?)
                            .with_operand(Operand::Literal(right_off))
                            .with_operand(Operand::Literal(imm_16b_low))
                            .with_operand(Operand::Literal(imm_16b_high))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {right_off}], [fp + {}]) op u32({imm_16b_low}, {imm_16b_high})",
                                dest_off + 1,
                                right_off + 1
                            ));
                        self.instructions.push(instr);
                        self.touch(dest_off, result_size);
                    }
                    // For subtraction and division, we store the immediate in a temporary variable
                    // TODO: In the future we should add opcodes imm_fp_sub and imm_fp_div
                    BinaryOp::U32Sub | BinaryOp::U32Div => {
                        let right_off = self.layout.get_offset(*right_id)?;
                        // Allocate a new temporary slot for the immediate (u32 takes 2 slots)
                        let temp_off = self.layout.reserve_stack(2);

                        self.store_u32_immediate(
                            *imm as u32,
                            temp_off,
                            format!("[fp + {temp_off}, fp + {}] = u32({imm})", temp_off + 1),
                        );

                        let instr = InstructionBuilder::new(self.fp_fp_opcode_for_u32_op(op)?)
                            .with_operand(Operand::Literal(temp_off))
                            .with_operand(Operand::Literal(right_off))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {temp_off}], [fp + {}]) op u32([fp + {right_off}], [fp + {}])",
                                dest_off + 1,
                                temp_off + 1,
                                right_off + 1
                            ));
                        self.instructions.push(instr);
                        self.touch(dest_off, result_size);
                    }
                    // For comparison operations with immediate values, we need to use fp_fp variant
                    BinaryOp::U32Eq
                    | BinaryOp::U32Neq
                    | BinaryOp::U32Greater
                    | BinaryOp::U32GreaterEqual
                    | BinaryOp::U32Less
                    | BinaryOp::U32LessEqual => {
                        let right_off = self.layout.get_offset(*right_id)?;
                        // Allocate a new temporary slot for the immediate (u32 takes 2 slots)
                        let temp_off = self.layout.reserve_stack(2);

                        self.store_u32_immediate(
                            *imm as u32,
                            temp_off,
                            format!("[fp + {temp_off}, fp + {}] = u32({imm})", temp_off + 1),
                        );

                        let instr = InstructionBuilder::new(self.fp_fp_opcode_for_u32_op(op)?)
                            .with_operand(Operand::Literal(temp_off))
                            .with_operand(Operand::Literal(right_off))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "[fp + {dest_off}] = u32([fp + {temp_off}], [fp + {}]) op u32([fp + {right_off}], [fp + {}])",
                                temp_off + 1,
                                right_off + 1
                            ));
                        self.instructions.push(instr);
                        self.touch(dest_off, result_size);
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported operation: {op:?}"
                        )));
                    }
                }
            }

            // Both operands are immediate: fold constants
            (Value::Literal(Literal::Integer(imm)), Value::Literal(Literal::Integer(imm2))) => {
                // Perform constant folding for U32 operations
                let left_u32 = { *imm };
                let right_u32 = { *imm2 };

                match op {
                    // Arithmetic operations - result is U32
                    BinaryOp::U32Add => {
                        let result = left_u32.wrapping_add(right_u32);
                        self.store_u32_immediate(
                            result,
                            dest_off,
                            format!("u32([fp + {dest_off}], [fp + {}]) = u32({result}) // {left_u32} + {right_u32}", dest_off + 1),
                        );
                    }
                    BinaryOp::U32Sub => {
                        let result = left_u32.wrapping_sub(right_u32);
                        self.store_u32_immediate(
                            result,
                            dest_off,
                            format!("u32([fp + {dest_off}], [fp + {}]) = u32({result}) // {left_u32} - {right_u32}", dest_off + 1),
                        );
                    }
                    BinaryOp::U32Mul => {
                        let result = left_u32.wrapping_mul(right_u32);
                        self.store_u32_immediate(
                            result,
                            dest_off,
                            format!("u32([fp + {dest_off}], [fp + {}]) = u32({result}) // {left_u32} * {right_u32}", dest_off + 1),
                        );
                    }
                    BinaryOp::U32Div => {
                        if right_u32 == 0 {
                            return Err(CodegenError::InvalidMir(
                                "Division by zero in U32 constant folding".to_string(),
                            ));
                        }
                        let result = left_u32 / right_u32;
                        self.store_u32_immediate(
                            result,
                            dest_off,
                            format!("u32([fp + {dest_off}], [fp + {}]) = u32({result}) // {left_u32} / {right_u32}", dest_off + 1),
                        );
                    }
                    // Comparison operations - result is felt (1 slot, 1 for true, 0 for false)
                    BinaryOp::U32Eq => {
                        let result = if left_u32 == right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) == u32({right_u32})"),
                        );
                    }
                    BinaryOp::U32Neq => {
                        let result = if left_u32 != right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) != u32({right_u32})"),
                        );
                    }
                    BinaryOp::U32Greater => {
                        let result = if left_u32 > right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) > u32({right_u32})"),
                        );
                    }
                    BinaryOp::U32GreaterEqual => {
                        let result = if left_u32 >= right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) >= u32({right_u32})"),
                        );
                    }
                    BinaryOp::U32Less => {
                        let result = if left_u32 < right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) < u32({right_u32})"),
                        );
                    }
                    BinaryOp::U32LessEqual => {
                        let result = if left_u32 <= right_u32 { 1 } else { 0 };
                        self.store_immediate(
                            result,
                            dest_off,
                            format!("[fp + {dest_off}] = {result} // u32({left_u32}) <= u32({right_u32})"),
                        );
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported U32 operation for constant folding: {op:?}"
                        )));
                    }
                }
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported operation".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Generate a function call that returns a value.
    pub fn call(
        &mut self,
        dest: ValueId,
        callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
    ) -> CodegenResult<()> {
        // Step 1: Pass arguments by storing them in the communication area.
        let args_offset = self.pass_arguments(callee_name, args, signature)?;
        // M is the total number of slots occupied by arguments (arrays as pointers)
        let m: usize = signature
            .param_types
            .iter()
            .map(|ty| match ty {
                MirType::FixedArray { .. } => 1,
                _ => DataLayout::size_of(ty),
            })
            .sum();
        // K is the total number of slots occupied by return values (U32 takes 2 slots)
        let k: usize = signature.return_types.iter().map(DataLayout::size_of).sum();

        // Step 2: Reserve space for return values and map the destination `ValueId`.
        // The first return value will be placed at `[fp_c + args_offset + M]`.
        let return_value_offset = args_offset + m as i32;
        self.layout.map_value(dest, return_value_offset);
        self.layout.reserve_stack(k);

        // Update max_written_offset to include the return value slots
        // This ensures the next call won't reuse these slots for arguments
        self.max_written_offset = self
            .max_written_offset
            .max(return_value_offset + k as i32 - 1);

        // Step 3: Calculate `frame_off` and emit the `call` instruction.
        // frame_off = where arguments start + size of arguments + size of return values
        let frame_off = args_offset + m as i32 + k as i32;
        let instr = InstructionBuilder::new(CALL_ABS_IMM)
            .with_operand(Operand::Literal(frame_off))
            .with_operand(Operand::Label(callee_name.to_string()))
            .with_comment(format!("call {callee_name}"));
        self.instructions.push(instr);

        // Step 4: No copy is needed after the call. The `dest` ValueId is already mapped
        // to the correct stack slot where the callee will place the return value.

        Ok(())
    }

    /// Generate a function call that returns multiple values.
    pub fn call_multiple(
        &mut self,
        dests: &[ValueId],
        callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
    ) -> CodegenResult<()> {
        // Step 1: Pass arguments by storing them in the communication area.
        let args_offset = self.pass_arguments(callee_name, args, signature)?;
        // M is the total number of slots occupied by arguments (arrays as pointers)
        let m: usize = signature
            .param_types
            .iter()
            .map(|ty| match ty {
                MirType::FixedArray { .. } => 1,
                _ => DataLayout::size_of(ty),
            })
            .sum();
        // K is the total number of slots occupied by return values (U32 takes 2 slots)
        let k: usize = signature.return_types.iter().map(DataLayout::size_of).sum();

        // Step 2: Reserve space for return values and map each destination ValueId.
        // Return values are placed after the arguments, accounting for multi-slot types
        let mut current_offset = args_offset + m as i32;
        for (i, dest) in dests.iter().enumerate() {
            self.layout.map_value(*dest, current_offset);
            // Move offset by the size of this return type
            if i < signature.return_types.len() {
                current_offset += DataLayout::size_of(&signature.return_types[i]) as i32;
            }
        }
        self.layout.reserve_stack(k);

        // Update max_written_offset to include the return value slots
        // This ensures the next call won't reuse these slots for arguments
        let last_return_offset = args_offset + m as i32 + k as i32 - 1;
        self.max_written_offset = self.max_written_offset.max(last_return_offset);

        // Step 3: Calculate `frame_off` and emit the `call` instruction.
        let frame_off = args_offset + m as i32 + k as i32;
        let instr = InstructionBuilder::new(CALL_ABS_IMM)
            .with_operand(Operand::Literal(frame_off))
            .with_operand(Operand::Label(callee_name.to_string()))
            .with_comment(format!("call {callee_name}"));
        self.instructions.push(instr);

        Ok(())
    }

    /// Generate a function call that does not return a value.
    pub fn void_call(
        &mut self,
        callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
    ) -> CodegenResult<()> {
        // For void calls, verify that the signature has no return types
        if !signature.return_types.is_empty() {
            return Err(CodegenError::InvalidMir(
                "void_call used with non-void signature".to_string(),
            ));
        }

        let args_offset = self.pass_arguments(callee_name, args, signature)?;
        // M is the total number of slots occupied by arguments (arrays as pointers)
        let m: usize = signature
            .param_types
            .iter()
            .map(|ty| match ty {
                MirType::FixedArray { .. } => 1,
                _ => DataLayout::size_of(ty),
            })
            .sum();
        let k = 0; // Void calls have no returns

        self.layout.reserve_stack(k);

        let frame_off = args_offset + m as i32 + k as i32;
        let instr = InstructionBuilder::new(CALL_ABS_IMM)
            .with_operand(Operand::Literal(frame_off))
            .with_operand(Operand::Label(callee_name.to_string()))
            .with_comment(format!("call {callee_name}"));
        self.instructions.push(instr);
        Ok(())
    }

    /// Helper to pass arguments for a function call.
    ///
    /// This method implements the "Argument-in-Place" optimization that avoids unnecessary
    /// copying when arguments are already positioned correctly at the top of the stack.
    ///
    /// ## How the optimization works
    ///
    /// The optimization checks if all arguments are already contiguous at the top of the
    /// caller's stack frame. If they are, no copy instructions are generated.
    ///
    /// ### Example 1: Optimization applies (single-slot types)
    /// ```text
    /// Before call:
    /// | fp + 0 | value_a |  <- arg 0
    /// | fp + 1 | value_b |  <- arg 1
    /// | fp + 2 | value_c |  <- arg 2
    /// L = 3 (current frame size)
    ///
    /// Since args are at [L-3, L-2, L-1] = [0, 1, 2], no copies needed.
    /// Returns L - total_slots = 0
    /// ```
    ///
    /// ### Example 2: Optimization applies (multi-slot types)
    /// ```text
    /// Before call with f(u32, felt):
    /// | fp + 0 | u32_lo  |  <- arg 0 (u32, slot 0)
    /// | fp + 1 | u32_hi  |  <- arg 0 (u32, slot 1)
    /// | fp + 2 | felt_val|  <- arg 1 (felt)
    /// L = 3
    ///
    /// Args occupy slots [0-1] and [2], contiguous at stack top.
    /// Returns L - total_slots = 0
    /// ```
    ///
    /// ### Example 3: Optimization does NOT apply
    /// ```text
    /// Before call:
    /// | fp + 0 | value_a |
    /// | fp + 1 | temp    |  <- intermediate value
    /// | fp + 2 | value_b |
    /// L = 3
    ///
    /// Args at [0] and [2] are not contiguous, must copy to [3] and [4].
    /// Returns L = 3
    /// ```
    ///
    /// ## Return value
    ///
    /// Returns the starting offset where arguments begin:
    /// - If optimization applied: `L - total_arg_slots`
    /// - If optimization not applied: `L` (after copying args to [L, L+1, ...])
    fn pass_arguments(
        &mut self,
        _callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
    ) -> CodegenResult<i32> {
        let l = self.layout.current_frame_usage();

        // Calculate the cumulative sizes and offsets for arguments
        let mut arg_offsets = Vec::new();
        let mut current_offset = l;

        for param_type in &signature.param_types {
            arg_offsets.push(current_offset);
            current_offset += DataLayout::size_of(param_type) as i32;
        }

        // Check for mismatch in argument count
        if args.len() != signature.param_types.len() {
            return Err(CodegenError::InvalidMir(format!(
                "Argument count mismatch: expected {}, got {}",
                signature.param_types.len(),
                args.len()
            )));
        }

        // Check if we can use the "argument-in-place" optimization
        {
            // First, check if all arguments are operands (not literals)
            let all_operands = args.iter().all(|arg| matches!(arg, Value::Operand(_)));

            if all_operands && !args.is_empty() {
                // Get the offset of the first argument
                if let Value::Operand(first_arg_id) = &args[0] {
                    if let Ok(first_offset) = self.layout.get_offset(*first_arg_id) {
                        // Check if all arguments are contiguous starting from the first one
                        let mut expected_offset = first_offset;
                        let mut all_args_contiguous = true;

                        for (arg, param_type) in args.iter().zip(&signature.param_types) {
                            let size = DataLayout::size_of(param_type);

                            if let Value::Operand(arg_id) = arg {
                                if !self.layout.is_contiguous(*arg_id, expected_offset, size) {
                                    all_args_contiguous = false;
                                    break;
                                }
                                expected_offset += size as i32;
                            }
                        }

                        if all_args_contiguous {
                            // With pre-allocated layouts, we can only apply the optimization
                            // if the arguments are at the top of the current frame
                            let total_arg_size: usize =
                                signature.param_types.iter().map(DataLayout::size_of).sum();
                            let args_end = first_offset + total_arg_size as i32;

                            // Check both conditions:
                            // 1. Arguments must be at the top of the pre-allocated frame
                            // 2. OR arguments must be at the top of what we've actually written
                            if args_end == self.layout.current_frame_usage()
                                || (self.max_written_offset >= 0
                                    && args_end == self.live_frame_usage())
                            {
                                // Arguments are at the top of the stack - safe to optimize
                                return Ok(first_offset);
                            }
                            // else: Arguments are contiguous but not at stack top - must copy
                        }
                    }
                }
            }
        }

        // Standard path: copy arguments to their positions
        for (i, (arg, param_type)) in args.iter().zip(&signature.param_types).enumerate() {
            let arg_offset = arg_offsets[i];

            // Fixed-Size arrays are passed as pointers - size 1.
            let arg_size = if matches!(param_type, MirType::FixedArray { .. }) {
                1
            } else {
                DataLayout::size_of(param_type)
            };

            match arg {
                Value::Literal(Literal::Integer(imm)) => match param_type {
                    MirType::Bool | MirType::Felt => {
                        self.store_immediate(
                            *imm,
                            arg_offset,
                            format!("Arg {i}: [fp + {arg_offset}] = {imm}"),
                        );
                    }
                    MirType::U32 => {
                        self.store_u32_immediate(
                            *imm,
                            arg_offset,
                            format!("Arg {i}: [fp + {arg_offset}] = {imm}"),
                        );
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported literal argument type: {:?}",
                            param_type
                        )));
                    }
                },
                Value::Operand(arg_id) => {
                    let src_off = self.layout.get_offset(*arg_id)?;

                    // Check if the argument is already in the correct position
                    // This can happen due to Direct Argument Placement optimization
                    if src_off == arg_offset
                        && self.layout.is_contiguous(*arg_id, arg_offset, arg_size)
                    {
                        // Argument is already in place, skip the copy
                        continue;
                    }

                    // Copy each slot of the argument
                    for slot in 0..arg_size {
                        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                            .with_operand(Operand::Literal(src_off + slot as i32))
                            .with_operand(Operand::Literal(0))
                            .with_operand(Operand::Literal(arg_offset + slot as i32))
                            .with_comment(format!(
                                "Arg {i} slot {slot}: [fp + {}] = [fp + {}] + 0",
                                arg_offset + slot as i32,
                                src_off + slot as i32
                            ));
                        self.instructions.push(instr);
                    }
                    self.touch(arg_offset, arg_size);
                }
                _ => {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Unsupported argument type".to_string(),
                    ));
                }
            }
        }
        Ok(l)
    }

    /// Generate `return` instruction with multiple return values.
    pub fn return_values(
        &mut self,
        values: &[Value],
        return_types: &[MirType],
    ) -> CodegenResult<()> {
        let k = self.layout.num_return_slots() as i32;

        // Store each return value in its designated slot
        let mut cumulative_slot_offset = 0;
        for (i, return_val) in values.iter().enumerate() {
            // Get the type of this return value
            let return_type = return_types.get(i).expect("Missing return type");
            // Return value starts at [fp - K - 2 + cumulative_slot_offset]
            let return_slot_offset = -(k + 2) + cumulative_slot_offset;

            // Check if the value is already in the return slot (optimization for direct returns)
            let needs_copy = match return_val {
                Value::Operand(val_id) => {
                    let current_offset = self.layout.get_offset(*val_id).unwrap_or(0);
                    current_offset != return_slot_offset
                }
                _ => true, // Literals always need to be stored
            };

            if needs_copy {
                match return_val {
                    Value::Literal(Literal::Integer(imm)) => {
                        // Check if this is a U32 return type
                        if matches!(return_type, MirType::U32) {
                            // For U32, we need to use U32_STORE_IMM
                            let value = *imm as u32;
                            self.store_u32_immediate(value, return_slot_offset, format!("Return value {i}: [fp {return_slot_offset}, fp {return_slot_offset} + 1] = u32({imm})"));
                        } else {
                            // For felt, use regular STORE_IMM
                            self.store_immediate(
                                *imm,
                                return_slot_offset,
                                format!("Return value {i}: [fp {return_slot_offset}] = {imm}"),
                            );
                        }
                    }
                    Value::Operand(val_id) => {
                        let src_off = self.layout.get_offset(*val_id)?;
                        let value_size = self.layout.get_value_size(*val_id);

                        // For multi-slot values, we need to copy each slot
                        for slot in 0..value_size {
                            let slot_instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                                .with_operand(Operand::Literal(src_off + slot as i32))
                                .with_operand(Operand::Literal(0))
                                .with_operand(Operand::Literal(return_slot_offset + slot as i32))
                                .with_comment(if value_size > 1 {
                                    format!(
                                        "Return value {} slot {}: [fp {}] = [fp + {}] + 0",
                                        i,
                                        slot,
                                        return_slot_offset + slot as i32,
                                        src_off + slot as i32
                                    )
                                } else {
                                    format!(
                                        "Return value {}: [fp {}] = [fp + {}] + 0",
                                        i, return_slot_offset, src_off
                                    )
                                });
                            self.instructions.push(slot_instr);
                        }
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported return value type".to_string(),
                        ));
                    }
                }

                // Determine the size of this return value
                let value_size = match return_val {
                    Value::Operand(val_id) => self.layout.get_value_size(*val_id),
                    Value::Literal(_) => {
                        // Check the type to determine size
                        if matches!(return_type, MirType::U32) {
                            2 // U32 takes 2 slots
                        } else {
                            1 // Felt takes 1 slot
                        }
                    }
                    _ => panic!("Unexpected value type: {:?}", return_val),
                };

                self.touch(return_slot_offset, value_size);
                cumulative_slot_offset += value_size as i32;
            } else {
                // Value is already in place, but we still need to update the offset
                let value_size = match return_val {
                    Value::Operand(val_id) => self.layout.get_value_size(*val_id),
                    _ => 1,
                };
                cumulative_slot_offset += value_size as i32;
            }
        }

        self.instructions
            .push(InstructionBuilder::new(RET).with_comment("return".to_string()));
        Ok(())
    }

    /// Generate a load instruction
    ///
    /// Translates `dest = *address` to a **stack-to-stack copy** from the source slots
    /// starting at `addr_off` into the destination slots starting at `dest_off`.
    /// In the flattened pointer model, `address` is a compile-time-known fp-relative slot,
    /// so a load is implemented as one or more `STORE_ADD_FP_IMM` copies.
    ///
    /// For multi-slot aggregates (e.g., structs/arrays), this copies **all slots**
    /// corresponding to the destination's size in the current function layout.
    pub fn load(&mut self, dest: ValueId, address: Value) -> CodegenResult<()> {
        match address {
            Value::Operand(addr_id) => {
                // The address operand represents a compile-time-known stack slot
                // computed via stackalloc/getelementptr.
                let src_offset = self.layout.get_offset(addr_id)?;
                let dest_offset = self.layout.get_offset(dest)?;
                let size = self.layout.get_value_size(dest).max(1);

                // Copy each slot (handles both single-slot and multi-slot aggregates)
                for i in 0..size {
                    let slot_src_off = src_offset + i as i32;
                    let slot_dest_off = dest_offset + i as i32;

                    let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(slot_src_off))
                        .with_operand(Operand::Literal(0))
                        .with_operand(Operand::Literal(slot_dest_off))
                        .with_comment(format!(
                            "Load: [fp + {slot_dest_off}] = [fp + {slot_src_off}] + 0"
                        ));
                    self.instructions.push(instr);
                }

                // Track writes for the whole aggregate
                self.touch(dest_offset, size);
                Ok(())
            }
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Load from non-operand address not supported: {:?}",
                address
            ))),
        }
    }

    pub fn load_u32(&mut self, dest: ValueId, address: Value) -> CodegenResult<()> {
        match address {
            Value::Operand(addr_id) => {
                let src_offset = self.layout.get_offset(addr_id)?;
                let dest_offset = self.layout.get_offset(dest)?;

                let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src_offset))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(dest_offset))
                    .with_comment(format!("LoadU32: [fp + {dest_offset}, fp + {dest_offset} + 1] = [fp + {src_offset}, fp + {src_offset} + 1] + 0"));
                self.instructions.push(instr);
                self.touch(dest_offset, 2);
                Ok(())
            }
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "LoadU32 from non-operand address not supported: {:?}",
                address
            ))),
        }
    }

    /// Generate a get element pointer instruction
    ///
    /// **IMPORTANT: Compile-Time Offset Calculation**
    ///
    /// In our flattened pointer model, `getelementptr` is purely a compile-time
    /// operation. It doesn't generate ANY runtime code! The FunctionLayout phase
    /// pre-calculates all offsets when building the stack frame layout.
    ///
    /// What happens:
    /// 1. During MIR lowering: `%ptr = getelementptr %base, offset`
    /// 2. During layout calculation: FunctionLayout computes `fp_offset(%ptr) = fp_offset(%base) + offset`
    /// 3. During codegen: This function is called but generates NO instructions
    /// 4. Later uses of %ptr will use the pre-calculated offset
    ///
    /// This works because all struct layouts and array indices are known at compile time.
    /// For dynamic indexing or heap pointers, this model would need to be completely redesigned.
    pub const fn get_element_ptr(
        &mut self,
        _dest: ValueId,
        _base: Value,
        _offset: Value,
    ) -> CodegenResult<()> {
        // The FunctionLayout has already calculated the offset for dest
        // This instruction doesn't generate any CASM code - it's purely compile-time
        Ok(())
    }

    /// Generate unconditional jump
    pub fn jump(&mut self, target_label: &str) -> CodegenResult<()> {
        let instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("jump abs {target_label}"));

        self.instructions.push(instr);
        Ok(())
    }

    /// Generates a conditional jump instruction that triggers if the value at `cond_off` is non-zero.
    /// The `target_label` is a placeholder that will be resolved to a relative offset later.
    pub fn jnz(&mut self, condition: Value, target_label: &str) -> CodegenResult<()> {
        // Get the condition value offset
        let cond_off = match condition {
            Value::Operand(cond_id) => self.layout.get_offset(cond_id)?,
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Condition must be a value operand".to_string(),
                ));
            }
        };

        self.jnz_offset(cond_off, target_label)
    }

    /// Generates a conditional jump based on a direct fp-relative offset.
    pub fn jnz_offset(&mut self, cond_off: i32, target_label: &str) -> CodegenResult<()> {
        let instr = InstructionBuilder::new(JNZ_FP_IMM)
            .with_operand(Operand::Literal(cond_off))
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("if [fp + {cond_off}] != 0 jmp rel {target_label}"));

        self.instructions.push(instr);
        Ok(())
    }

    /// Allocate stack space for StackAlloc instruction
    ///
    /// This allocates the requested number of slots for the destination. This is a no-op, it just increases
    /// the current frame usage.
    pub fn allocate_frame_slots(&mut self, dest: ValueId, size: usize) -> CodegenResult<()> {
        // Allocate the requested size
        let _dest_off = self.layout.allocate_local(dest, size)?;

        // FrameAlloc doesn't generate actual instructions, it just reserves space
        // The allocation is tracked in the layout for later use
        Ok(())
    }

    /// Add a raw CASM instruction
    pub fn add_instruction(&mut self, instruction: InstructionBuilder) {
        self.instructions.push(instruction);
    }

    /// Get the generated instructions
    pub fn instructions(&self) -> &[InstructionBuilder] {
        &self.instructions
    }

    /// Get the labels
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Get a mutable reference to the layout
    pub const fn layout_mut(&mut self) -> &mut FunctionLayout {
        &mut self.layout
    }

    /// Get the label counter
    pub const fn label_counter(&self) -> usize {
        self.label_counter
    }
    /// Take ownership of the generated instructions
    pub fn into_instructions(self) -> Vec<InstructionBuilder> {
        self.instructions
    }

    /// Take ownership of the labels
    pub fn into_labels(self) -> Vec<Label> {
        self.labels
    }

    /// Generate a store instruction
    ///
    /// Handles stores to stackalloc addresses (common for parameter copying)
    /// Since we don't have indirect store in the ISA, we treat stackalloc
    /// addresses as direct local variable slots
    pub fn store(&mut self, address: Value, value: Value) -> CodegenResult<()> {
        match address {
            Value::Operand(addr_id) => {
                // The address is actually the location where we want to store
                let dest_offset = self.layout.get_offset(addr_id)?;

                match value {
                    Value::Literal(inner) => {
                        let imm = match inner {
                            Literal::Integer(imm) => imm,
                            Literal::Boolean(imm) => imm as u32,
                            _ => {
                                return Err(CodegenError::UnsupportedInstruction(format!(
                                    "Unsupported store value type: {:?}",
                                    value
                                )));
                            }
                        };

                        self.store_immediate(
                            imm,
                            dest_offset,
                            format!("Store immediate: [fp + {dest_offset}] = {imm}"),
                        );
                    }

                    Value::Operand(val_id) => {
                        let val_offset = self.layout.get_offset(val_id)?;
                        // Get the size of the value being stored
                        let size = self.layout.get_value_size(val_id);

                        // For multi-slot values, copy each slot
                        for i in 0..size {
                            let slot_src_off = val_offset + i as i32;
                            let slot_dest_off = dest_offset + i as i32;
                            let instr = InstructionBuilder::new(STORE_ADD_FP_IMM) // StoreAddFpImm
                                .with_operand(Operand::Literal(slot_src_off))
                                .with_operand(Operand::Literal(0))
                                .with_operand(Operand::Literal(slot_dest_off))
                                .with_comment(format!(
                                    "Store: [fp + {slot_dest_off}] = [fp + {slot_src_off}] + 0"
                                ));

                            self.instructions.push(instr);
                        }
                        self.touch(dest_offset, size);
                    }

                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported store value type: {:?}",
                            value
                        )));
                    }
                }
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Store to non-operand address not supported".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn store_u32(&mut self, address: Value, value: Value) -> CodegenResult<()> {
        match address {
            Value::Operand(addr_id) => {
                let dest_offset = self.layout.get_offset(addr_id)?;

                match value {
                    Value::Literal(Literal::Integer(imm)) => {
                        self.store_u32_immediate(
                            imm as u32,
                            dest_offset,
                            format!(
                                "[fp + {}, fp + {}] = u32({imm})",
                                dest_offset,
                                dest_offset + 1
                            ),
                        );
                    }
                    Value::Operand(val_id) => {
                        let val_offset = self.layout.get_offset(val_id)?;

                        let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                            .with_operand(Operand::Literal(val_offset))
                            .with_operand(Operand::Literal(0))
                            .with_operand(Operand::Literal(0))
                            .with_operand(Operand::Literal(dest_offset))
                            .with_comment(format!("[fp + {dest_offset}], [fp + {dest_offset} + 1] = [fp + {val_offset}], [fp + {val_offset}  +1] + 0"));
                        self.instructions.push(instr);
                        self.touch(dest_offset, 2);
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported store value type".to_string(),
                        ));
                    }
                }
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Store to non-operand address not supported".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Removes any occurrences of instructions where two or more offsets are the same.
    /// This is required by the prover, which does not currently support memory operations on the same memory location in a single instruction.
    /// This fix was designed to be as non-invasive as possible to be reverted easily in case of design changes in the prover.
    pub fn resolve_duplicate_offsets(&mut self) -> CodegenResult<()> {
        let mut new_instructions = Vec::new();
        // Track how instruction indices change: original_index -> new_index_range
        let mut index_mapping: Vec<Option<std::ops::Range<usize>>> = Vec::new();

        // Reserve space for temporary variables only when needed
        // Keep separate temps for felt (1-slot) and U32 (2-slot) operations to avoid size conflicts
        let mut felt_temp1: Option<i32> = None;
        let mut felt_temp2: Option<i32> = None;
        let mut u32_temp1: Option<i32> = None;
        let mut u32_temp2: Option<i32> = None;

        for instr in self.instructions.iter() {
            let current_new_index = new_instructions.len();

            let replacement_instructions = match instr.opcode {
                STORE_ADD_FP_FP | STORE_SUB_FP_FP | STORE_MUL_FP_FP | STORE_DIV_FP_FP => {
                    // Reserve temp variables on demand for felt operations (need 2 single-slot temps)
                    if felt_temp1.is_none() || felt_temp2.is_none() {
                        felt_temp1 = Some(self.layout.reserve_stack(1));
                        felt_temp2 = Some(self.layout.reserve_stack(1));
                    }
                    Self::handle_fp_fp_duplicates(instr, felt_temp1.unwrap(), felt_temp2.unwrap())
                }
                STORE_ADD_FP_IMM | STORE_SUB_FP_IMM | STORE_MUL_FP_IMM | STORE_DIV_FP_IMM => {
                    // Reserve temp variable on demand for felt operations with immediate (need 1 single-slot temp)
                    if felt_temp1.is_none() {
                        felt_temp1 = Some(self.layout.reserve_stack(1));
                    }
                    Self::handle_fp_imm_duplicates(instr, felt_temp1.unwrap())?
                }
                // U32 arithmetic operations with FP operands
                U32_STORE_ADD_FP_FP | U32_STORE_SUB_FP_FP | U32_STORE_MUL_FP_FP
                | U32_STORE_DIV_FP_FP => {
                    // Reserve temp variables on demand for U32 operations (need 2 slots each)
                    if u32_temp1.is_none() || u32_temp2.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                        u32_temp2 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_fp_duplicates(instr, u32_temp1.unwrap(), u32_temp2.unwrap())
                }
                // U32 arithmetic operations with immediate operands
                U32_STORE_ADD_FP_IMM | U32_STORE_SUB_FP_IMM | U32_STORE_MUL_FP_IMM
                | U32_STORE_DIV_FP_IMM => {
                    // Reserve temp variable on demand for U32 operations (need 2 slots)
                    if u32_temp1.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_imm_duplicates(instr, u32_temp1.unwrap())?
                }
                // U32 comparison operations with FP operands (result is felt, not u32)
                U32_STORE_EQ_FP_FP | U32_STORE_NEQ_FP_FP | U32_STORE_GT_FP_FP
                | U32_STORE_GE_FP_FP | U32_STORE_LT_FP_FP | U32_STORE_LE_FP_FP => {
                    // Reserve temp variables on demand for U32 comparisons (need 2 slots each for operands)
                    if u32_temp1.is_none() || u32_temp2.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                        u32_temp2 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_fp_duplicates(instr, u32_temp1.unwrap(), u32_temp2.unwrap())
                }
                // U32 comparison operations with immediate operands
                U32_STORE_EQ_FP_IMM | U32_STORE_NEQ_FP_IMM | U32_STORE_GT_FP_IMM
                | U32_STORE_GE_FP_IMM | U32_STORE_LT_FP_IMM | U32_STORE_LE_FP_IMM => {
                    // Reserve temp variable on demand for U32 comparisons (need 2 slots)
                    if u32_temp1.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_imm_duplicates(instr, u32_temp1.unwrap())?
                }
                _ => {
                    // Keep instruction as-is
                    vec![instr.clone()]
                }
            };

            if replacement_instructions.is_empty() {
                // Instruction was removed
                index_mapping.push(None);
            } else {
                let start_index = current_new_index;
                let end_index = current_new_index + replacement_instructions.len();
                new_instructions.extend(replacement_instructions);
                index_mapping.push(Some(start_index..end_index));
            }
        }

        // Update label addresses based on the index mapping
        for label in &mut self.labels {
            if let Some(original_address) = label.address {
                if original_address < index_mapping.len() {
                    if let Some(ref range) = index_mapping[original_address] {
                        // Point to the first instruction in the replacement range
                        // This preserves the semantic meaning: execution starts at the first replacement
                        label.address = Some(range.start);
                    } else {
                        // The instruction was removed - this should not happen for labeled instructions
                        // due to our check above, but if it does, we need to find the next valid instruction
                        let next_valid = index_mapping
                            .iter()
                            .skip(original_address + 1)
                            .find_map(|opt_range| opt_range.as_ref().map(|range| range.start));
                        label.address = next_valid;
                    }
                }
            }
        }

        self.instructions = new_instructions;

        Ok(())
    }

    /// Handles duplicate offsets in fp+fp binary operations.
    /// Expands in-place operations using temporary variables to avoid undefined behavior.
    fn handle_fp_fp_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
        temp_var_offset2: i32,
    ) -> Vec<InstructionBuilder> {
        // Extract operands - fp-fp instructions have format: [off0, off1, off2]
        let off0 = instr.op0().unwrap();
        let off1 = instr.op1().unwrap();
        let off2 = instr.op2().unwrap();

        if off0 == off1 && off1 == off2 {
            // The three offsets are the same, store off0 and off1 in temp vars and replace with 3 instructions
            vec![
                InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(off0))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(off1))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(temp_var_offset2))
                    .with_comment(format!("[fp + {temp_var_offset2}] = [fp + {off1}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(temp_var_offset2))
                    .with_operand(Operand::Literal(off2))
                    .with_comment(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {temp_var_offset2}]"
                    )),
            ]
        } else if off0 == off1 || off0 == off2 {
            // off0 is a duplicate, store off0 in a temp var and replace with 2 instructions
            vec![
                InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(off0))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(off1))
                    .with_operand(Operand::Literal(off2))
                    .with_comment(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {off1}]"
                    )),
            ]
        } else if off1 == off2 {
            // off1 is a duplicate, store off1 in a temp var and replace with 2 instructions
            vec![
                InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(off1))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off1}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(off0))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(off2))
                    .with_comment(format!(
                        "[fp + {off2}] = [fp + {off0}] op [fp + {temp_var_offset}]"
                    )),
            ]
        } else {
            // No duplicates, keep as-is
            vec![instr.clone()]
        }
    }

    /// Handles duplicate offsets in fp+immediate binary operations.
    /// Expands in-place operations using a temporary variable when source equals destination.
    fn handle_fp_imm_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        // Extract operands - fp-imm instructions have format: [off0, imm, off2]
        let off0 = instr.op0().unwrap();
        let off2 = instr.op2().unwrap();

        // Get the immediate value (should be at position 1)
        let imm = if let Some(Operand::Literal(imm)) = instr.operands.get(1) {
            *imm
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "Store immediate operand must be a literal".to_string(),
            ));
        };

        if off0 == off2 {
            // off0 is a duplicate, store off0 in a temp var and replace with 2 instructions
            Ok(vec![
                InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(off0))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(imm))
                    .with_operand(Operand::Literal(off2))
                    .with_comment(format!("[fp + {off2}] = [fp + {temp_var_offset}] op {imm}")),
            ])
        } else {
            // No duplicates, keep as-is
            Ok(vec![instr.clone()])
        }
    }

    /// Handles duplicate offsets in U32 fp+fp operations.
    /// Similar to handle_fp_fp_duplicates but needs to handle 2-slot U32 values.
    /// For U32 comparisons, only the destination is 1 slot (felt result).
    fn handle_u32_fp_fp_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
        temp_var_offset2: i32,
    ) -> Vec<InstructionBuilder> {
        // Extract operands - U32 fp-fp instructions have format: [src0_off, src1_off, dst_off]
        let src0_off = instr.op0().unwrap();
        let src1_off = instr.op1().unwrap();
        let dst_off = instr.op2().unwrap();

        // Check if this is a comparison (result is felt) or arithmetic (result is u32)
        let is_comparison = matches!(
            instr.opcode,
            U32_STORE_EQ_FP_FP
                | U32_STORE_NEQ_FP_FP
                | U32_STORE_GT_FP_FP
                | U32_STORE_GE_FP_FP
                | U32_STORE_LT_FP_FP
                | U32_STORE_LE_FP_FP
        );

        // For U32 values, we need to check overlaps considering 2-slot values
        // src0 uses [src0_off, src0_off+1], src1 uses [src1_off, src1_off+1]
        // dst uses [dst_off] for comparisons, [dst_off, dst_off+1] for arithmetic

        let src0_overlaps_src1 = src0_off == src1_off
            || src0_off == src1_off + 1
            || src0_off + 1 == src1_off
            || src0_off + 1 == src1_off + 1;

        let src0_overlaps_dst = if is_comparison {
            src0_off == dst_off || src0_off + 1 == dst_off
        } else {
            src0_off == dst_off
                || src0_off == dst_off + 1
                || src0_off + 1 == dst_off
                || src0_off + 1 == dst_off + 1
        };

        let src1_overlaps_dst = if is_comparison {
            src1_off == dst_off || src1_off + 1 == dst_off
        } else {
            src1_off == dst_off
                || src1_off == dst_off + 1
                || src1_off + 1 == dst_off
                || src1_off + 1 == dst_off + 1
        };

        if src0_overlaps_src1 && src0_overlaps_dst {
            // All three overlap, need to copy both sources to temp locations
            // We need 4 temp slots total (2 for each U32)
            vec![
                // Copy src0 to temp using U32_STORE_ADD_FP_IMM
                InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src0_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1, src0_off + 1)),
                // Copy src1 to temp using U32_STORE_ADD_FP_IMM
                InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src1_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(temp_var_offset2))
                    .with_comment(format!("u32([fp + {temp_var_offset2}], [fp + {}]) = u32([fp + {src1_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset2 + 1, src1_off + 1)),
                // Perform operation with temp locations
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(temp_var_offset2))
                    .with_operand(Operand::Literal(dst_off))
                    .with_comment(if is_comparison {
                        format!("[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {temp_var_offset2}], [fp + {}])",
                            temp_var_offset + 1, temp_var_offset2 + 1)
                    } else {
                        format!("u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {temp_var_offset2}], [fp + {}])",
                            dst_off + 1, temp_var_offset + 1, temp_var_offset2 + 1)
                    }),
            ]
        } else if src0_overlaps_dst {
            // src0 overlaps with dst, copy src0 to temp
            vec![
                // Copy src0 to temp using U32_STORE_ADD_FP_IMM
                InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src0_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1, src0_off + 1)),
                // Perform operation
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(src1_off))
                    .with_operand(Operand::Literal(dst_off))
                    .with_comment(if is_comparison {
                        format!("[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {src1_off}], [fp + {}])",
                            temp_var_offset + 1, src1_off + 1)
                    } else {
                        format!("u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {src1_off}], [fp + {}])",
                            dst_off + 1, temp_var_offset + 1, src1_off + 1)
                    }),
            ]
        } else if src1_overlaps_dst {
            // src1 overlaps with dst, copy src1 to temp
            vec![
                // Copy src1 to temp using U32_STORE_ADD_FP_IMM
                InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src1_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src1_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1, src1_off + 1)),
                // Perform operation
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(src0_off))
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(dst_off))
                    .with_comment(if is_comparison {
                        format!("[fp + {dst_off}] = u32([fp + {src0_off}], [fp + {}]) op u32([fp + {temp_var_offset}], [fp + {}])",
                            src0_off + 1, temp_var_offset + 1)
                    } else {
                        format!("u32([fp + {dst_off}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) op u32([fp + {temp_var_offset}], [fp + {}])",
                            dst_off + 1, src0_off + 1, temp_var_offset + 1)
                    }),
            ]
        } else {
            // No overlaps, keep as-is
            vec![instr.clone()]
        }
    }

    /// Handles duplicate offsets in U32 fp+immediate operations.
    /// Similar to handle_fp_imm_duplicates but needs to handle 2-slot U32 values.
    fn handle_u32_fp_imm_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        // Extract operands - U32 fp-imm instructions have format: [src_off, imm_lo, imm_hi, dst_off]
        let src_off = instr.op0().unwrap();
        let dst_off = if let Some(Operand::Literal(off)) = instr.operands.get(3) {
            *off
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 fp-imm instruction missing destination offset".to_string(),
            ));
        };

        // Get the immediate values (should be at positions 1 and 2)
        let imm_lo = if let Some(Operand::Literal(imm)) = instr.operands.get(1) {
            *imm
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 Store immediate low operand must be a literal".to_string(),
            ));
        };

        let imm_hi = if let Some(Operand::Literal(imm)) = instr.operands.get(2) {
            *imm
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 Store immediate high operand must be a literal".to_string(),
            ));
        };

        // Check if this is a comparison (result is felt) or arithmetic (result is u32)
        let is_comparison = matches!(
            instr.opcode,
            U32_STORE_EQ_FP_IMM
                | U32_STORE_NEQ_FP_IMM
                | U32_STORE_GT_FP_IMM
                | U32_STORE_GE_FP_IMM
                | U32_STORE_LT_FP_IMM
                | U32_STORE_LE_FP_IMM
        );

        // Check for problematic overlap between source and destination
        // For fp+imm operations, in-place operations (src_off == dst_off) are fine!
        // We only need to handle partial overlaps where the source and destination
        // overlap but are not identical.
        let has_overlap = if is_comparison {
            // For comparisons, dst is only 1 slot
            // Problematic if high word of source overlaps with dest
            src_off + 1 == dst_off
        } else {
            // For arithmetic, dst is 2 slots
            // In-place operation (src_off == dst_off) is fine
            // Problematic only if there's a partial overlap
            (src_off == dst_off + 1) || (src_off + 1 == dst_off)
        };

        if has_overlap {
            // Source overlaps with destination, copy source to temp
            Ok(vec![
                // Copy src to temp using U32_STORE_ADD_FP_IMM
                InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Literal(0))  // imm_lo
                    .with_operand(Operand::Literal(0))  // imm_hi
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_comment(format!("u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1, src_off + 1)),
                // Perform operation with temp location
                InstructionBuilder::new(instr.opcode)
                    .with_operand(Operand::Literal(temp_var_offset))
                    .with_operand(Operand::Literal(imm_lo))
                    .with_operand(Operand::Literal(imm_hi))
                    .with_operand(Operand::Literal(dst_off))
                    .with_comment(if is_comparison {
                        format!("[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32({imm_lo}, {imm_hi})",
                            temp_var_offset + 1)
                    } else {
                        format!("u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32({imm_lo}, {imm_hi})",
                            dst_off + 1, temp_var_offset + 1)
                    }),
            ])
        } else {
            // No overlap, keep as-is
            Ok(vec![instr.clone()])
        }
    }

    // ===== Aggregate Operations =====

    /// Creates a struct by allocating consecutive registers and copying field values
    pub fn make_struct(
        &mut self,
        dest: ValueId,
        fields: &[(String, Value)],
        struct_ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        let total_size = DataLayout::size_of(struct_ty);

        // Allocate destination
        let base_offset = self.layout.allocate_local(dest, total_size)?;

        // Copy each field to its offset
        for (field_name, field_value) in fields {
            let field_offset =
                DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
                    CodegenError::InvalidMir(format!(
                        "Field '{}' not found in struct type",
                        field_name
                    ))
                })?;

            let target_offset = base_offset + field_offset as i32;

            // Get the field type to determine its size
            let field_ty = struct_ty.field_type(field_name).ok_or_else(|| {
                CodegenError::InvalidMir(format!("Could not get type for field '{}'", field_name))
            })?;
            let field_size = DataLayout::size_of(field_ty);

            // Copy the field value to the target offset
            self.copy_value_to_offset(field_value, target_offset, field_size)?;
        }

        Ok(())
    }

    /// Extracts a field from a struct by mapping the destination to the field's offset
    pub fn extract_struct_field(
        &mut self,
        dest: ValueId,
        struct_val: Value,
        field_name: &str,
        field_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // DataLayout methods are now static - no instance needed

        // Get struct base offset and ID
        let (struct_offset, struct_id) = match struct_val {
            Value::Operand(id) => (self.layout.get_offset(id)?, id),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "ExtractStructField requires operand source".to_string(),
                ))
            }
        };

        // Get the struct type from the function's value_types
        let struct_ty = function.value_types.get(&struct_id).ok_or_else(|| {
            CodegenError::InvalidMir(format!("No type found for struct value {:?}", struct_id))
        })?;

        // Calculate field offset within the struct
        let field_offset = DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Field '{}' not found in struct", field_name))
        })?;

        let field_size = DataLayout::size_of(field_ty);
        let absolute_offset = struct_offset + field_offset as i32;

        // Map destination to the field's location
        if field_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size: field_size,
                },
            );
        }

        Ok(())
    }

    /// Inserts a new value into a struct field (in-place update)
    pub fn insert_struct_field(
        &mut self,
        dest: ValueId,
        struct_val: Value,
        field_name: &str,
        new_value: Value,
        struct_ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Get struct base offset
        let struct_offset = match struct_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "InsertField requires operand source".to_string(),
                ))
            }
        };

        // Calculate field offset
        let field_offset = DataLayout::field_offset(struct_ty, field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Field '{}' not found in struct", field_name))
        })?;

        // Get field type and size
        let field_ty = struct_ty.field_type(field_name).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Could not get type for field '{}'", field_name))
        })?;
        let field_size = DataLayout::size_of(field_ty);

        // Calculate target offset for the field
        let target_offset = struct_offset + field_offset as i32;

        // Overwrite the field with the new value
        self.copy_value_to_offset(&new_value, target_offset, field_size)?;

        // Map the destination to the same location as the source struct
        // (since it's an in-place update)
        let struct_size = DataLayout::size_of(struct_ty);
        if struct_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: struct_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: struct_offset,
                    size: struct_size,
                },
            );
        }

        Ok(())
    }

    /// Creates a tuple by allocating consecutive registers and copying element values
    pub fn make_tuple(
        &mut self,
        dest: ValueId,
        elements: &[Value],
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // DataLayout methods are now static - no instance needed

        // Determine the types of elements to calculate sizes
        let mut total_size = 0;
        let mut element_offsets = Vec::new();
        let mut element_sizes = Vec::new();

        for element in elements {
            element_offsets.push(total_size);

            // Determine element size from type information
            let element_size = match element {
                Value::Operand(id) => {
                    if let Some(ty) = function.value_types.get(id) {
                        DataLayout::size_of(ty)
                    } else {
                        self.layout.get_value_size(*id)
                    }
                }
                Value::Literal(_) => 1, // Literals are always single-slot for now
                _ => 1,
            };

            element_sizes.push(element_size);
            total_size += element_size;
        }

        // Allocate destination
        let base_offset = self.layout.allocate_local(dest, total_size)?;

        // Copy each element to its offset
        for (i, element) in elements.iter().enumerate() {
            let target_offset = base_offset + element_offsets[i] as i32;
            let element_size = element_sizes[i];

            self.copy_value_to_offset(element, target_offset, element_size)?;
        }

        Ok(())
    }

    /// Extracts an element from a tuple by mapping the destination to the element's offset
    pub fn extract_tuple_element(
        &mut self,
        dest: ValueId,
        tuple: Value,
        index: usize,
        element_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // DataLayout methods are now static - no instance needed

        // Get tuple base offset and ID
        let (tuple_offset, tuple_id) = match tuple {
            Value::Operand(id) => (self.layout.get_offset(id)?, id),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "ExtractTupleElement requires operand source".to_string(),
                ))
            }
        };

        // Get the tuple type from the function's value_types
        let tuple_ty = function.value_types.get(&tuple_id).ok_or_else(|| {
            CodegenError::InvalidMir(format!("No type found for tuple value {:?}", tuple_id))
        })?;

        // Calculate element offset within the tuple
        let element_offset = DataLayout::tuple_offset(tuple_ty, index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Tuple index {} out of bounds", index))
        })?;

        let element_size = DataLayout::size_of(element_ty);
        let absolute_offset = tuple_offset + element_offset as i32;

        // Map destination to the element's location
        if element_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size: element_size,
                },
            );
        }

        Ok(())
    }

    /// Inserts a new value into a tuple element (in-place update)
    pub fn insert_tuple_element(
        &mut self,
        dest: ValueId,
        tuple_val: Value,
        index: usize,
        new_value: Value,
        tuple_ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // DataLayout methods are now static - no instance needed

        // Get tuple base offset
        let tuple_offset = match tuple_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "InsertTuple requires operand source".to_string(),
                ))
            }
        };

        // Calculate element offset
        let element_offset = DataLayout::tuple_offset(tuple_ty, index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Tuple index {} out of bounds", index))
        })?;

        // Get element type and size
        let element_ty = tuple_ty.tuple_element_type(index).ok_or_else(|| {
            CodegenError::InvalidMir(format!("Could not get type for tuple element {}", index))
        })?;
        let element_size = DataLayout::size_of(element_ty);

        // Calculate target offset for the element
        let target_offset = tuple_offset + element_offset as i32;

        // Overwrite the element with the new value
        self.copy_value_to_offset(&new_value, target_offset, element_size)?;

        // Map the destination to the same location as the source tuple
        // (since it's an in-place update)
        let tuple_size = DataLayout::size_of(tuple_ty);
        if tuple_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: tuple_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: tuple_offset,
                    size: tuple_size,
                },
            );
        }

        Ok(())
    }

    /// Helper method to copy a value to a specific offset
    fn copy_value_to_offset(
        &mut self,
        value: &Value,
        target_offset: i32,
        size: usize,
    ) -> CodegenResult<()> {
        // Nothing to copy for zero-sized values (e.g., unit)
        if size == 0 {
            return Ok(());
        }
        match value {
            Value::Literal(Literal::Integer(imm)) => {
                if size == 1 {
                    self.store_immediate(
                        *imm,
                        target_offset,
                        format!("[fp + {}] = {}", target_offset, imm),
                    );
                } else if size == 2 {
                    // Handle u32 literal
                    self.store_u32_immediate(
                        *imm,
                        target_offset,
                        format!(
                            "[fp + {}], [fp + {}] = u32({})",
                            target_offset,
                            target_offset + 1,
                            imm
                        ),
                    );
                } else {
                    return Err(CodegenError::UnsupportedInstruction(format!(
                        "Unsupported literal size: {}",
                        size
                    )));
                }
            }
            Value::Literal(Literal::Boolean(b)) => {
                // Booleans are single-slot values storing 0/1
                let imm = if *b { 1 } else { 0 };
                self.store_immediate(
                    imm,
                    target_offset,
                    format!("[fp + {}] = {}", target_offset, imm),
                );
            }
            Value::Literal(Literal::Unit) => {
                // Unit has size 0 by layout; nothing to store
            }
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(*src_id)?;

                // Copy each slot
                for i in 0..size {
                    let slot_src = src_offset + i as i32;
                    let slot_dst = target_offset + i as i32;

                    let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(slot_src))
                        .with_operand(Operand::Literal(0))
                        .with_operand(Operand::Literal(slot_dst))
                        .with_comment(format!("[fp + {}] = [fp + {}] + 0", slot_dst, slot_src));
                    self.instructions.push(instr);
                    self.touch(slot_dst, 1);
                }
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported value type in aggregate: {:?}",
                    value
                )));
            }
        }

        Ok(())
    }

    /// Create a fixed-size array from elements
    /// Materializes elements in contiguous locals and returns a pointer (fp + base)
    pub fn make_fixed_array(
        &mut self,
        dest: ValueId,
        elements: &[Value],
        element_ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Calculate per-element size and total size needed for the array
        let element_size = DataLayout::size_of(element_ty);
        let total_size = element_size * elements.len();

        // Reserve space for the array elements (anonymous region)
        let base_offset = if total_size > 0 {
            self.layout.reserve_stack(total_size)
        } else {
            // Zero-sized array: still produce a pointer to the current top (valid but unused)
            self.layout.current_frame_usage()
        };

        // Copy each element to its position in the array
        for (index, element) in elements.iter().enumerate() {
            let target_offset = base_offset + (index * element_size) as i32;
            self.copy_value_to_offset(element, target_offset, element_size)?;
        }

        // Allocate a single-slot destination for the array pointer
        let dest_offset = self.layout.allocate_local(dest, 1)?;
        // Store the address (fp + base_offset) into the destination slot
        let instr = InstructionBuilder::new(STORE_FP_IMM)
            .with_operand(Operand::Literal(base_offset))
            .with_operand(Operand::Literal(dest_offset))
            .with_comment(format!("[fp + {dest_offset}] = fp + {base_offset}"));
        self.instructions.push(instr);
        self.touch(dest_offset, 1);

        Ok(())
    }

    // ===== Unified Array Operations =====

    /// Unified array operation handler that dispatches based on index type and operation
    pub fn array_operation(
        &mut self,
        array: Value,
        index: Value,
        element_ty: &MirType,
        operation: ArrayOperation,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Get array base pointer (arrays are always stored as pointers)
        let array_offset = match array {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Array must be an operand (pointer)".to_string(),
                ))
            }
        };

        // Calculate element size
        let element_size = DataLayout::size_of(element_ty);

        // Handle based on index type
        match index {
            Value::Literal(Literal::Integer(idx)) => {
                // Static index - compile-time offset calculation
                let element_offset = (idx as i32) * (element_size as i32);

                match operation {
                    ArrayOperation::Load { dest } => {
                        self.load_from_memory_static(
                            dest,
                            array_offset,
                            element_offset,
                            element_ty,
                        )?;
                    }
                    ArrayOperation::Store { dest, value } => {
                        self.store_to_memory_static(
                            dest,
                            value,
                            array_offset,
                            element_offset,
                            element_ty,
                            function,
                        )?;
                    }
                }
            }
            Value::Operand(idx_id) => {
                // Dynamic index - runtime offset calculation
                let idx_offset = self.layout.get_offset(idx_id)?;

                // For multi-slot elements, multiply index by element_size
                let scaled_offset = if element_size > 1 {
                    let temp_offset = self.layout.reserve_stack(1);
                    let instr = InstructionBuilder::new(STORE_MUL_FP_IMM)
                        .with_operand(Operand::Literal(idx_offset))
                        .with_operand(Operand::Literal(element_size as i32))
                        .with_operand(Operand::Literal(temp_offset))
                        .with_comment(format!(
                            "[fp + {}] = [fp + {}] * {} (scale index by element size)",
                            temp_offset, idx_offset, element_size
                        ));
                    self.instructions.push(instr);
                    self.touch(temp_offset, 1);
                    temp_offset
                } else {
                    idx_offset
                };

                match operation {
                    ArrayOperation::Load { dest } => {
                        self.load_from_memory_dynamic(
                            dest,
                            array_offset,
                            scaled_offset,
                            element_ty,
                        )?;
                    }
                    ArrayOperation::Store { dest, value } => {
                        self.store_to_memory_dynamic(
                            dest,
                            value,
                            array_offset,
                            scaled_offset,
                            element_ty,
                            function,
                        )?;
                    }
                }
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Array index must be literal or operand".to_string(),
                ))
            }
        }

        Ok(())
    }

    /// Helper for static loads - arrays store pointers so we load from computed address
    fn load_from_memory_static(
        &mut self,
        dest: ValueId,
        base_offset: i32,
        element_offset: i32,
        ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Since arrays store pointers to their first element, and elements are
        // laid out sequentially, element N is at memory address [array_ptr + N*elem_size]
        // For static index, we can compute the absolute offset
        let absolute_offset = base_offset + element_offset;

        // Map destination to this location (no copy needed, just aliasing)
        let size = DataLayout::size_of(ty);
        if size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size,
                },
            );
        }

        Ok(())
    }

    /// Helper for dynamic loads - use STORE_DOUBLE_DEREF_FP_FP to load from memory
    fn load_from_memory_dynamic(
        &mut self,
        dest: ValueId,
        base_offset: i32,
        scaled_offset: i32,
        ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        let elem_size = DataLayout::size_of(ty);
        let dest_off = self.layout.allocate_local(dest, elem_size)?;

        // Load slot 0
        let instr0 = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP_FP)
            .with_operand(Operand::Literal(base_offset))
            .with_operand(Operand::Literal(scaled_offset))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(format!(
                "[fp + {}] = [[fp + {}] + [fp + {}]]",
                dest_off, base_offset, scaled_offset
            ));
        self.instructions.push(instr0);
        self.touch(dest_off, 1);

        // Additional slots if element spans multiple words (e.g., U32)
        for s in 1..elem_size {
            // temp_index = scaled_offset + s
            let tmp_idx = self.layout.reserve_stack(1);
            let add = InstructionBuilder::new(STORE_ADD_FP_IMM)
                .with_operand(Operand::Literal(scaled_offset))
                .with_operand(Operand::Literal(s as i32))
                .with_operand(Operand::Literal(tmp_idx))
                .with_comment(format!(
                    "[fp + {}] = [fp + {}] + {} (offset for slot {})",
                    tmp_idx, scaled_offset, s, s
                ));
            self.instructions.push(add);

            let dst_slot = dest_off + s as i32;
            let instr = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP_FP)
                .with_operand(Operand::Literal(base_offset))
                .with_operand(Operand::Literal(tmp_idx))
                .with_operand(Operand::Literal(dst_slot))
                .with_comment(format!(
                    "[fp + {}] = [[fp + {}] + [fp + {}]] (slot {})",
                    dst_slot, base_offset, tmp_idx, s
                ));
            self.instructions.push(instr);
            self.touch(dst_slot, 1);
        }

        Ok(())
    }

    /// Helper for static stores using StoreToDoubleDerefFpImm
    fn store_to_memory_static(
        &mut self,
        dest: ValueId,
        value: Value,
        base_offset: i32,
        element_offset: i32,
        ty: &MirType,
        _function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // For static stores, we just copy the pointer (arrays are immutable)
        // The dest gets the same pointer as the original array
        let dest_offset = self.layout.allocate_local(dest, 1)?;

        // Store the array pointer to dest
        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(base_offset))
            .with_operand(Operand::Literal(0))
            .with_operand(Operand::Literal(dest_offset))
            .with_comment(format!(
                "[fp + {}] = [fp + {}] + 0 (copy array pointer)",
                dest_offset, base_offset
            ));
        self.instructions.push(instr);
        self.touch(dest_offset, 1);

        // Now store the value to the array element
        match value {
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(src_id)?;

                match ty {
                    MirType::U32 => {
                        // Store 2 slots for u32
                        for i in 0..2 {
                            let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
                                .with_operand(Operand::Literal(base_offset))
                                .with_operand(Operand::Literal(element_offset + i))
                                .with_operand(Operand::Literal(src_offset + i))
                                .with_comment(format!(
                                    "[[fp + {}] + {}] = [fp + {}]",
                                    base_offset,
                                    element_offset + i,
                                    src_offset + i
                                ));
                            self.instructions.push(instr);
                        }
                    }
                    _ => {
                        // Single slot types
                        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
                            .with_operand(Operand::Literal(base_offset))
                            .with_operand(Operand::Literal(element_offset))
                            .with_operand(Operand::Literal(src_offset))
                            .with_comment(format!(
                                "[[fp + {}] + {}] = [fp + {}]",
                                base_offset, element_offset, src_offset
                            ));
                        self.instructions.push(instr);
                    }
                }
            }
            Value::Literal(Literal::Integer(val)) => {
                // Store literal to a temp location first
                let temp_offset = self.layout.reserve_stack(1);
                self.store_immediate(
                    val,
                    temp_offset,
                    format!("[fp + {}] = {}", temp_offset, val),
                );

                // Then store from temp to array
                let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
                    .with_operand(Operand::Literal(base_offset))
                    .with_operand(Operand::Literal(element_offset))
                    .with_operand(Operand::Literal(temp_offset))
                    .with_comment(format!(
                        "[[fp + {}] + {}] = [fp + {}]",
                        base_offset, element_offset, temp_offset
                    ));
                self.instructions.push(instr);
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Invalid value for array store".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// Helper for dynamic stores using StoreToDoubleDerefFpFp
    fn store_to_memory_dynamic(
        &mut self,
        dest: ValueId,
        value: Value,
        base_offset: i32,
        scaled_offset: i32,
        ty: &MirType,
        _function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        // For dynamic stores, we just copy the pointer (arrays are immutable)
        // The dest gets the same pointer as the original array
        let dest_offset = self.layout.allocate_local(dest, 1)?;

        // Store the array pointer to dest
        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(base_offset))
            .with_operand(Operand::Literal(0))
            .with_operand(Operand::Literal(dest_offset))
            .with_comment(format!(
                "[fp + {}] = [fp + {}] + 0 (copy array pointer)",
                dest_offset, base_offset
            ));
        self.instructions.push(instr);
        self.touch(dest_offset, 1);

        // Now store the value to the array element
        match value {
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(src_id)?;

                match ty {
                    MirType::U32 => {
                        // Store 2 slots for u32
                        for i in 0..2 {
                            if i > 0 {
                                // Add i to the scaled offset for subsequent slots
                                let adjusted_offset = self.layout.reserve_stack(1);
                                let add_instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                                    .with_operand(Operand::Literal(scaled_offset))
                                    .with_operand(Operand::Literal(i))
                                    .with_operand(Operand::Literal(adjusted_offset))
                                    .with_comment(format!(
                                        "[fp + {}] = [fp + {}] + {} (adjust for slot {})",
                                        adjusted_offset, scaled_offset, i, i
                                    ));
                                self.instructions.push(add_instr);

                                let store_instr =
                                    InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
                                        .with_operand(Operand::Literal(base_offset))
                                        .with_operand(Operand::Literal(adjusted_offset))
                                        .with_operand(Operand::Literal(src_offset + i))
                                        .with_comment(format!(
                                            "[[fp + {}] + [fp + {}]] = [fp + {}]",
                                            base_offset,
                                            adjusted_offset,
                                            src_offset + i
                                        ));
                                self.instructions.push(store_instr);
                            } else {
                                let store_instr =
                                    InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
                                        .with_operand(Operand::Literal(base_offset))
                                        .with_operand(Operand::Literal(scaled_offset))
                                        .with_operand(Operand::Literal(src_offset))
                                        .with_comment(format!(
                                            "[[fp + {}] + [fp + {}]] = [fp + {}]",
                                            base_offset, scaled_offset, src_offset
                                        ));
                                self.instructions.push(store_instr);
                            }
                        }
                    }
                    _ => {
                        // Single slot types
                        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
                            .with_operand(Operand::Literal(base_offset))
                            .with_operand(Operand::Literal(scaled_offset))
                            .with_operand(Operand::Literal(src_offset))
                            .with_comment(format!(
                                "[[fp + {}] + [fp + {}]] = [fp + {}]",
                                base_offset, scaled_offset, src_offset
                            ));
                        self.instructions.push(instr);
                    }
                }
            }
            Value::Literal(Literal::Integer(val)) => {
                // Store literal to a temp location first
                let temp_offset = self.layout.reserve_stack(1);
                self.store_immediate(
                    val,
                    temp_offset,
                    format!("[fp + {}] = {}", temp_offset, val),
                );

                // Then store from temp to array
                let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
                    .with_operand(Operand::Literal(base_offset))
                    .with_operand(Operand::Literal(scaled_offset))
                    .with_operand(Operand::Literal(temp_offset))
                    .with_comment(format!(
                        "[[fp + {}] + [fp + {}]] = [fp + {}]",
                        base_offset, scaled_offset, temp_offset
                    ));
                self.instructions.push(instr);
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "Invalid value for array store".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// Extract an element from an array by static index
    pub fn extract_array_element(
        &mut self,
        dest: ValueId,
        array: Value,
        index: usize,
        element_ty: &MirType,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Get array base offset
        let array_offset = match array {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "ExtractArrayElement requires operand source".to_string(),
                ))
            }
        };

        // Calculate element offset (arrays are laid out sequentially)
        let element_size = DataLayout::size_of(element_ty);
        let element_offset = index * element_size;
        let absolute_offset = array_offset + element_offset as i32;

        // Map destination to the element's location
        if element_size == 1 {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::Slot {
                    offset: absolute_offset,
                },
            );
        } else {
            self.layout.value_layouts.insert(
                dest,
                ValueLayout::MultiSlot {
                    offset: absolute_offset,
                    size: element_size,
                },
            );
        }

        Ok(())
    }

    /// Inserts a new value into an array element (creates new array with updated element)
    pub fn insert_array_element(
        &mut self,
        dest: ValueId,
        array_val: Value,
        index: Value,
        new_value: Value,
        array_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;

        // Get array base offset
        let array_offset = match array_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "DynamicArrayInsert requires operand array source".to_string(),
                ))
            }
        };

        // Get array size and element info
        let total_size = DataLayout::size_of(array_ty);
        let (element_ty, _array_size) = match array_ty {
            MirType::FixedArray { element_type, size } => (element_type.as_ref(), *size),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "DynamicArrayInsert requires array type".to_string(),
                ))
            }
        };
        let elem_size = DataLayout::size_of(element_ty);

        // Process index value
        let index_val = match index {
            Value::Operand(idx_id) => {
                return Err(CodegenError::InternalError(
                    "Static array indexing expects a static index".to_string(),
                ));
            }
            Value::Literal(Literal::Integer(val)) => val,
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported index value for DynamicArrayInsert".to_string(),
                ));
            }
        };

        // Scale index by element size (in slots)
        let scaled_index = index_val as usize * elem_size;

        // Now write the new value to the computed position using StoreToDoubleDerefFpFp
        // Process the new value
        let new_val_offsets = match new_value {
            Value::Operand(id) => {
                let offset = self.layout.get_offset(id)?;
                let val_type = function.value_types.get(&id).ok_or_else(|| {
                    CodegenError::InvalidMir("Missing type for new value".to_string())
                })?;
                let size = DataLayout::size_of(val_type);
                (0..size).map(|i| offset + i as i32).collect::<Vec<_>>()
            }
            Value::Literal(Literal::Integer(val)) => {
                // Store literal in temp slot
                // TODO: what if the array's elements are u32s and taking two slots?
                let tmp = self.layout.reserve_stack(1);
                self.store_immediate(val, tmp, format!("[fp + {tmp}] = {val}"));
                vec![tmp]
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported new_value for DynamicArrayInsert".to_string(),
                ));
            }
        };

        // Write each slot of the new value using StoreToDoubleDerefFpFp
        for (slot_idx, &src_off) in new_val_offsets.iter().enumerate() {
            let slot_offset = if slot_idx > 0 {
                // For multi-slot elements, add slot_idx to scaled_off
                let tmp = self.layout.reserve_stack(1);
                let add = InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(scaled_index as i32))
                    .with_operand(Operand::Literal(slot_idx as i32))
                    .with_operand(Operand::Literal(tmp))
                    .with_comment(format!(
                        "[fp + {tmp}] = [fp + {scaled_index}] + {slot_idx} - Offset for slot {slot_idx}"
                    ));
                self.instructions.push(add);
                tmp
            } else {
                scaled_index as i32
            };

            // Use StoreToDoubleDerefFpFp: [[fp + base_ptr_off] + imm] = [fp + src_off]
            let store_instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
                .with_operand(Operand::Literal(array_offset))
                .with_operand(Operand::Literal(slot_offset))
                .with_operand(Operand::Literal(src_off))
                .with_comment(format!(
                    "[[fp + {array_offset}] + {slot_offset}] = [fp + {src_off}] - Store element slot {slot_idx}"
                ));
            self.instructions.push(store_instr);
        }

        Ok(())
    }

    /// Extract an element from an array by dynamic index (runtime value)
    ///
    /// Preferred strategy: if the array value is a pointer (MirType::Pointer to FixedArray),
    /// use STORE_DOUBLE_DEREF_FP_FP to fetch `[base_ptr + index]` directly (and repeat for
    /// additional slots if the element spans multiple slots).
    ///
    /// Fallback (only when array is a local value-based aggregate): linear dispatch that
    /// selects the correct element and copies it. This path remains until we add an
    /// address-of helper/opcode to materialize `fp + base_off` as a runtime pointer.
    pub fn dynamic_array_index(
        &mut self,
        dest: ValueId,
        array: Value,
        index: Value,
        element_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;
        use cairo_m_compiler_mir::MirType as MT;

        // Get array base offset and validate types
        let (array_offset, array_id) = match array {
            Value::Operand(id) => (self.layout.get_offset(id)?, id),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "DynamicArrayIndex requires operand array source".to_string(),
                ))
            }
        };

        // Determine array type from the MIR value type
        let array_ty = function
            .value_types
            .get(&array_id)
            .ok_or_else(|| CodegenError::InvalidMir("Missing type for array value".to_string()))?;

        let elem_size = DataLayout::size_of(element_ty);

        // If index is a literal, fallback to static extraction
        if let Value::Literal(Literal::Integer(_)) = index {
            return Err(CodegenError::InternalError(
                "Dynamic array indexing expects an operand index".to_string(),
            ));
        }

        // Pointer fast-path using StoreDoubleDerefFpFp
        if let MT::FixedArray { .. } = array_ty {
            // Ensure index is a single-slot felt we can reference.
            // If index is u32 (two slots), convert to felt: idx = lo + (hi * 65536).
            let (index_off, index_ty) = match index {
                Value::Operand(idx_id) => (
                    self.layout.get_offset(idx_id)?,
                    function.value_types.get(&idx_id).cloned(),
                ),
                Value::Literal(Literal::Integer(val)) => {
                    // TODO: Ensure this is correct - normally this should not be needed as literal offsets generate non-dynamic array indexes.
                    // Materialize literal index into a temp slot
                    let tmp = self.layout.reserve_stack(1);
                    self.store_immediate(val, tmp, format!("[fp + {tmp}] = {val}"));
                    (tmp, Some(MT::Felt))
                }
                _ => {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Unsupported index value for DynamicArrayIndex".to_string(),
                    ));
                }
            };

            // If index is u32, fold limbs into a felt temporary
            let index_off = if matches!(index_ty, Some(MT::U32)) {
                let lo_off = index_off;
                let hi_off = index_off + 1;
                let mul_off = self.layout.reserve_stack(1);
                // mul_off = [fp + hi_off] * 65536
                let mul = InstructionBuilder::new(STORE_MUL_FP_IMM)
                    .with_operand(Operand::Literal(hi_off))
                    .with_operand(Operand::Literal(65536))
                    .with_operand(Operand::Literal(mul_off))
                    .with_comment(format!(
                        "Converting u32 to felt [fp + {mul_off}] = [fp + {hi_off}] * 65536"
                    ));
                self.instructions.push(mul);
                // sum_off = mul_off + [fp + lo_off]
                let sum_off = self.layout.reserve_stack(1);
                let add = InstructionBuilder::new(STORE_ADD_FP_FP)
                        .with_operand(Operand::Literal(mul_off))
                        .with_operand(Operand::Literal(lo_off))
                        .with_operand(Operand::Literal(sum_off))
                        .with_comment(format!(
                            "Converting u32 to felt [fp + {sum_off}] = [fp + {mul_off}] + [fp + {lo_off}]"
                        ));
                self.instructions.push(add);
                sum_off
            } else {
                index_off
            };

            // Allocate destination for the element
            let dest_off = self.layout.allocate_local(dest, elem_size)?;

            // Scale index by element size (in slots)
            let scaled_off = if elem_size > 1 {
                let scaled = self.layout.reserve_stack(1);
                let mul = InstructionBuilder::new(STORE_MUL_FP_IMM)
                    .with_operand(Operand::Literal(index_off))
                    .with_operand(Operand::Literal(elem_size as i32))
                    .with_operand(Operand::Literal(scaled))
                    .with_comment(format!(
                        "[fp + {scaled}] = [fp + {index_off}] * {elem_size}"
                    ));
                self.instructions.push(mul);
                scaled
            } else {
                index_off
            };

            // Slot 0
            let instr0 = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP_FP)
                    .with_operand(Operand::Literal(array_offset))
                    .with_operand(Operand::Literal(scaled_off))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!(
                        "[fp + {dest_off}] = [[fp + {array_offset}] + [fp + {scaled_off}]] - StoreDoubleDerefFpFp"
                    ));
            self.instructions.push(instr0);
            self.touch(dest_off, 1);

            // Additional slots if element spans multiple words
            for s in 1..elem_size {
                // temp_index = scaled + s
                let tmp_idx = self.layout.reserve_stack(1);
                let add = InstructionBuilder::new(STORE_ADD_FP_IMM)
                        .with_operand(Operand::Literal(scaled_off))
                        .with_operand(Operand::Literal(s as i32))
                        .with_operand(Operand::Literal(tmp_idx))
                        .with_comment(format!(
                            "[fp + {tmp_idx}] = [fp + {scaled_off}] + {s} - Temp index for additional slot {s}"
                        ));
                self.instructions.push(add);

                let dst_slot = dest_off + s as i32;
                let instr = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP_FP)
                        .with_operand(Operand::Literal(array_offset))
                        .with_operand(Operand::Literal(tmp_idx))
                        .with_operand(Operand::Literal(dst_slot))
                        .with_comment(format!(
                            "[fp + {dst_slot}] = [[fp + {array_offset}] + [fp + {tmp_idx}]] - Additional slot {s}"
                        ));
                self.instructions.push(instr);
                self.touch(dst_slot, 1);
            }

            return Ok(());
        }

        // If we get here, array is a local value-based aggregate. Without an address-of
        // capability to materialize `fp + base_off` into a slot, we cannot use the
        // double-deref opcode safely. Prefer surfacing a clear error to avoid emitting
        // large dispatch sequences.
        Err(CodegenError::UnsupportedInstruction(
            "Dynamic array indexing on local arrays requires pointer materialization; pass arrays as pointers or add address-of support".to_string(),
        ))
    }

    /// Insert an element into an array by dynamic index (runtime value)
    ///
    /// This creates a new array with the element at the specified index replaced.
    /// Uses the new StoreToDoubleDerefFpFp instruction to write to dynamically computed addresses.
    pub fn dynamic_array_insert(
        &mut self,
        dest: ValueId,
        array_val: Value,
        index: Value,
        new_value: Value,
        array_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        use cairo_m_compiler_mir::layout::DataLayout;
        use cairo_m_compiler_mir::MirType as MT;

        // Get array base offset
        let array_offset = match array_val {
            Value::Operand(id) => self.layout.get_offset(id)?,
            _ => {
                return Err(CodegenError::InvalidMir(
                    "DynamicArrayInsert requires operand array source".to_string(),
                ))
            }
        };

        // Get array size and element info
        let total_size = DataLayout::size_of(array_ty);
        let (element_ty, _array_size) = match array_ty {
            MirType::FixedArray { element_type, size } => (element_type.as_ref(), *size),
            _ => {
                return Err(CodegenError::InvalidMir(
                    "DynamicArrayInsert requires array type".to_string(),
                ))
            }
        };
        let elem_size = DataLayout::size_of(element_ty);

        // Process index value
        let (index_off, index_ty) = match index {
            Value::Operand(idx_id) => (
                self.layout.get_offset(idx_id)?,
                function.value_types.get(&idx_id).cloned(),
            ),
            Value::Literal(Literal::Integer(val)) => {
                return Err(CodegenError::InternalError(
                    "Dynamic array indexing expects an operand index".to_string(),
                ));
                // This shouldn't happen for dynamic path, but handle it
                let tmp = self.layout.reserve_stack(1);
                self.store_immediate(val, tmp, format!("[fp + {tmp}] = {val}"));
                (tmp, Some(MT::Felt))
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported index value for DynamicArrayInsert".to_string(),
                ));
            }
        };

        // If index is u32, fold limbs into a felt temporary
        let index_off = if matches!(index_ty, Some(MT::U32)) {
            let lo_off = index_off;
            let hi_off = index_off + 1;
            let mul_off = self.layout.reserve_stack(1);
            // mul_off = [fp + hi_off] * 65536
            let mul = InstructionBuilder::new(STORE_MUL_FP_IMM)
                .with_operand(Operand::Literal(hi_off))
                .with_operand(Operand::Literal(65536))
                .with_operand(Operand::Literal(mul_off))
                .with_comment(format!(
                    "Converting u32 to felt [fp + {mul_off}] = [fp + {hi_off}] * 65536"
                ));
            self.instructions.push(mul);
            // sum_off = mul_off + [fp + lo_off]
            let sum_off = self.layout.reserve_stack(1);
            let add = InstructionBuilder::new(STORE_ADD_FP_FP)
                .with_operand(Operand::Literal(mul_off))
                .with_operand(Operand::Literal(lo_off))
                .with_operand(Operand::Literal(sum_off))
                .with_comment(format!(
                    "Converting u32 to felt [fp + {sum_off}] = [fp + {mul_off}] + [fp + {lo_off}]"
                ));
            self.instructions.push(add);
            sum_off
        } else {
            index_off
        };

        // Scale index by element size (in slots)
        let scaled_off = if elem_size > 1 {
            let scaled = self.layout.reserve_stack(1);
            let mul = InstructionBuilder::new(STORE_MUL_FP_IMM)
                .with_operand(Operand::Literal(index_off))
                .with_operand(Operand::Literal(elem_size as i32))
                .with_operand(Operand::Literal(scaled))
                .with_comment(format!(
                    "[fp + {scaled}] = [fp + {index_off}] * {elem_size} - Scale index by element size"
                ));
            self.instructions.push(mul);
            scaled
        } else {
            index_off
        };

        // Now write the new value to the computed position using StoreToDoubleDerefFpFp
        // Process the new value
        let new_val_offsets = match new_value {
            Value::Operand(id) => {
                let offset = self.layout.get_offset(id)?;
                let val_type = function.value_types.get(&id).ok_or_else(|| {
                    CodegenError::InvalidMir("Missing type for new value".to_string())
                })?;
                let size = DataLayout::size_of(val_type);
                (0..size).map(|i| offset + i as i32).collect::<Vec<_>>()
            }
            Value::Literal(Literal::Integer(val)) => {
                // Store literal in temp slot
                let tmp = self.layout.reserve_stack(1);
                self.store_immediate(val, tmp, format!("[fp + {tmp}] = {val}"));
                vec![tmp]
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported new_value for DynamicArrayInsert".to_string(),
                ));
            }
        };

        // Write each slot of the new value using StoreToDoubleDerefFpFp
        for (slot_idx, &src_off) in new_val_offsets.iter().enumerate() {
            let slot_offset = if slot_idx > 0 {
                // For multi-slot elements, add slot_idx to scaled_off
                let tmp = self.layout.reserve_stack(1);
                let add = InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(scaled_off))
                    .with_operand(Operand::Literal(slot_idx as i32))
                    .with_operand(Operand::Literal(tmp))
                    .with_comment(format!(
                        "[fp + {tmp}] = [fp + {scaled_off}] + {slot_idx} - Offset for slot {slot_idx}"
                    ));
                self.instructions.push(add);
                tmp
            } else {
                scaled_off
            };

            // Use StoreToDoubleDerefFpFp: [[fp + base_ptr_off] + [fp + slot_offset]] = [fp + src_off]
            let store_instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
                .with_operand(Operand::Literal(array_offset))
                .with_operand(Operand::Literal(slot_offset))
                .with_operand(Operand::Literal(src_off))
                .with_comment(format!(
                    "[[fp + {array_offset}] + [fp + {slot_offset}]] = [fp + {src_off}] - Store element slot {slot_idx}"
                ));
            self.instructions.push(store_instr);
        }

        Ok(())
    }

    /// Unified array index helper: dispatches to static or dynamic path
    pub fn array_index(
        &mut self,
        dest: ValueId,
        array: Value,
        index: Value,
        element_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        match index {
            Value::Literal(Literal::Integer(i)) => {
                self.extract_array_element(dest, array, i as usize, element_ty)
            }
            _ => self.dynamic_array_index(dest, array, index, element_ty, function),
        }
    }

    /// Unified array insert helper: dispatches to static or dynamic path
    pub fn array_insert(
        &mut self,
        dest: ValueId,
        array_val: Value,
        index: Value,
        new_value: Value,
        array_ty: &MirType,
        function: &cairo_m_compiler_mir::MirFunction,
    ) -> CodegenResult<()> {
        match index {
            Value::Literal(Literal::Integer(i)) => {
                self.insert_array_element(dest, array_val, index, new_value, array_ty, function)
            }
            _ => self.dynamic_array_insert(dest, array_val, index, new_value, array_ty, function),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairo_m_compiler_mir::MirType;

    #[test]
    fn test_handle_fp_fp_duplicates_all_same() {
        // Create instruction: [fp + 5] = [fp + 5] + [fp + 5]
        let instr = InstructionBuilder::new(STORE_ADD_FP_FP)
            .with_operand(Operand::Literal(5))
            .with_operand(Operand::Literal(5))
            .with_operand(Operand::Literal(5));

        let temp1 = 10;
        let temp2 = 11;
        let result = CasmBuilder::handle_fp_fp_duplicates(&instr, temp1, temp2);
        assert_eq!(result.len(), 3, "Should expand to 3 instructions");

        // Check that we use temp variables
        assert_eq!(result[0].op2(), Some(temp1));
        assert_eq!(result[1].op2(), Some(temp2));
        assert_eq!(result[2].op0(), Some(temp1));
        assert_eq!(result[2].op1(), Some(temp2));
    }

    #[test]
    fn test_handle_fp_fp_duplicates_first_operand_conflict() {
        // Create instruction: [fp + 5] = [fp + 5] + [fp + 3]
        let instr = InstructionBuilder::new(STORE_ADD_FP_FP)
            .with_operand(Operand::Literal(5))
            .with_operand(Operand::Literal(3))
            .with_operand(Operand::Literal(5));

        let temp1 = 10;
        let temp2 = 11;
        let result = CasmBuilder::handle_fp_fp_duplicates(&instr, temp1, temp2);
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that first operand is copied to temp
        assert_eq!(result[0].op2(), Some(temp1));
        assert_eq!(result[0].op0(), Some(5));
        assert_eq!(result[1].op0(), Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_in_place() {
        // Create instruction: [fp + 5] = [fp + 5] + 42
        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(5))
            .with_operand(Operand::Literal(42))
            .with_operand(Operand::Literal(5));

        let temp1 = 10;
        let result = CasmBuilder::handle_fp_imm_duplicates(&instr, temp1).unwrap();
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that source is copied to temp first
        assert_eq!(result[0].op2(), Some(temp1));
        assert_eq!(result[0].op0(), Some(5));
        assert_eq!(result[1].op0(), Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_no_conflict() {
        // Create instruction: [fp + 7] = [fp + 5] + 42 (no conflict)
        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(5))
            .with_operand(Operand::Literal(42))
            .with_operand(Operand::Literal(7));

        let temp1 = 10;
        let result = CasmBuilder::handle_fp_imm_duplicates(&instr, temp1).unwrap();
        assert_eq!(result.len(), 1, "Should keep as single instruction");

        // Should be unchanged
        assert_eq!(result[0].op0(), Some(5));
        assert_eq!(result[0].op2(), Some(7));
    }

    #[test]
    fn test_pass_arguments_optimization_single_slot() {
        // Test that single-slot arguments at the top of stack are not copied
        let mut layout = FunctionLayout::new_for_test();

        // First, allocate some other values to simulate existing stack usage
        let dummy1 = ValueId::from_raw(10);
        let dummy2 = ValueId::from_raw(11);
        let dummy3 = ValueId::from_raw(12);
        layout.allocate_value(dummy1, 1).unwrap(); // offset 0
        layout.allocate_value(dummy2, 1).unwrap(); // offset 1
        layout.allocate_value(dummy3, 1).unwrap(); // offset 2

        // Now allocate our actual arguments at the top of the stack (offsets 3, 4, 5)
        let arg1 = ValueId::from_raw(1);
        let arg2 = ValueId::from_raw(2);
        let arg3 = ValueId::from_raw(3);

        layout.allocate_value(arg1, 1).unwrap(); // offset 3
        layout.allocate_value(arg2, 1).unwrap(); // offset 4
        layout.allocate_value(arg3, 1).unwrap(); // offset 5

        let mut builder = CasmBuilder::new(layout, 0);

        // Create a signature with 3 felt arguments
        let signature = CalleeSignature {
            param_types: vec![MirType::Felt, MirType::Felt, MirType::Felt],
            return_types: vec![],
        };

        // Arguments in order at top of stack
        let args = vec![
            Value::Operand(arg1),
            Value::Operand(arg2),
            Value::Operand(arg3),
        ];

        let start_offset = builder
            .pass_arguments("test_func", &args, &signature)
            .unwrap();

        // Should return 3 (start of args at fp+3) and generate no copy instructions
        assert_eq!(
            start_offset, 3,
            "Should return the start offset of arguments"
        );
        assert_eq!(
            builder.instructions.len(),
            0,
            "Should generate no copy instructions"
        );
    }

    #[test]
    fn test_pass_arguments_optimization_multi_slot() {
        // Test that multi-slot arguments at the top of stack are not copied
        let mut layout = FunctionLayout::new_for_test();

        // First, allocate some dummy values to simulate existing stack usage
        let dummy = ValueId::from_raw(10);
        layout.allocate_value(dummy, 3).unwrap(); // offsets 0-2

        // Now allocate a u32 (2 slots) and a felt (1 slot) at the top
        let u32_arg = ValueId::from_raw(1);
        let felt_arg = ValueId::from_raw(2);

        layout.allocate_value(u32_arg, 2).unwrap(); // offsets 3-4
        layout.allocate_value(felt_arg, 1).unwrap(); // offset 5

        let mut builder = CasmBuilder::new(layout, 0);

        // Create a signature with u32 and felt
        let signature = CalleeSignature {
            param_types: vec![MirType::U32, MirType::Felt],
            return_types: vec![],
        };

        // Arguments in order at top of stack
        let args = vec![Value::Operand(u32_arg), Value::Operand(felt_arg)];

        let start_offset = builder
            .pass_arguments("test_func", &args, &signature)
            .unwrap();

        // Should return 3 (start of args at fp+3) and generate no copy instructions
        assert_eq!(
            start_offset, 3,
            "Should return the start offset of arguments"
        );
        assert_eq!(
            builder.instructions.len(),
            0,
            "Should generate no copy instructions for multi-slot args"
        );
    }

    #[test]
    fn test_pass_arguments_no_optimization_out_of_order() {
        // Test that out-of-order arguments are copied
        let mut layout = FunctionLayout::new_for_test();

        // First, allocate a dummy value
        let dummy = ValueId::from_raw(10);
        layout.allocate_value(dummy, 2).unwrap(); // offsets 0-1

        // Allocate values but pass them out of order
        let arg1 = ValueId::from_raw(1);
        let arg2 = ValueId::from_raw(2);

        layout.allocate_value(arg1, 1).unwrap(); // offset 2
        layout.allocate_value(arg2, 1).unwrap(); // offset 3

        let mut builder = CasmBuilder::new(layout, 0);

        let signature = CalleeSignature {
            param_types: vec![MirType::Felt, MirType::Felt],
            return_types: vec![],
        };

        // Arguments out of order
        let args = vec![
            Value::Operand(arg2), // This is at offset 1 but should be at 0
            Value::Operand(arg1), // This is at offset 0 but should be at 1
        ];

        let start_offset = builder
            .pass_arguments("test_func", &args, &signature)
            .unwrap();

        // Should generate copy instructions
        assert_eq!(start_offset, 4, "Should return the frame usage");
        assert_eq!(
            builder.instructions.len(),
            2,
            "Should generate copy instructions for out-of-order args"
        );
    }
}
