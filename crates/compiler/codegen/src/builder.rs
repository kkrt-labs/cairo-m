//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.

use cairo_m_common::instruction::*;
use cairo_m_compiler_mir::instruction::CalleeSignature;
use cairo_m_compiler_mir::{BinaryOp, Literal, Value, ValueId};
use cairo_m_compiler_parser::parser::UnaryOp;
use stwo_prover::core::fields::m31::M31;

use crate::{CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label, Operand};

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
    fn store_immediate(&mut self, value: i32, offset: i32, comment: String) {
        let instr = InstructionBuilder::new(STORE_IMM)
            .with_operand(Operand::Literal(value))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.instructions.push(instr);
        self.touch(offset, 1);
    }

    /// Store immediate value at a specific offset (public version)
    pub fn store_immediate_at(
        &mut self,
        value: i32,
        offset: i32,
        comment: String,
    ) -> CodegenResult<()> {
        self.store_immediate(value, offset, comment);
        Ok(())
    }

    /// Store a value at a specific offset
    pub fn store_at(&mut self, dest_id: ValueId, offset: i32, value: Value) -> CodegenResult<()> {
        // Map the destination to the specific offset
        self.layout.map_value(dest_id, offset);

        match value {
            Value::Literal(Literal::Integer(imm)) => {
                self.store_immediate(imm, offset, format!("[fp + {}] = {}", offset, imm));
            }
            Value::Operand(src_id) => {
                let src_off = self.layout.get_offset(src_id)?;
                let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
                    .with_operand(Operand::Literal(src_off))
                    .with_operand(Operand::Literal(0))
                    .with_operand(Operand::Literal(offset))
                    .with_comment(format!("[fp + {}] = [fp + {}] + 0", offset, src_off));
                self.instructions.push(instr);
                self.touch(offset, 1);
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported store value type".to_string(),
                ));
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
            // Get the pre-allocated offset from the layout
            self.layout.get_offset(dest)?
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Store immediate value
                self.store_immediate(imm, dest_off, format!("[fp + {dest_off}] = {imm}"));
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
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported assignment source".to_string(),
                ));
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
                todo!("Comparison opcodes not yet implemented");
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
                    .with_operand(Operand::Literal(*imm))
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
                            .with_operand(Operand::Literal(*imm))
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
                    BinaryOp::Div => (M31::from(*imm) / M31::from(*imm2)).0,
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported operation".to_string(),
                        ));
                    }
                };

                self.store_immediate(
                    result as i32,
                    dest_off,
                    format!("[fp + {dest_off}] = {result}"),
                );
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported operation".to_string(),
                ));
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
            Value::Literal(Literal::Integer(imm)) => {
                // For immediate values, we can directly compute the NOT result
                let result = if imm == 0 { 1 } else { 0 };
                self.store_immediate(result, dest_off, format!("[fp + {dest_off}] = {result}"));
                return Ok(());
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported source operand in NOT".to_string(),
                ));
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
        // For comparison operations, the result is a single felt (0 or 1)
        let is_comparison = matches!(
            op,
            BinaryOp::U32Eq
                | BinaryOp::U32Neq
                | BinaryOp::U32Less
                | BinaryOp::U32Greater
                | BinaryOp::U32LessEqual
                | BinaryOp::U32GreaterEqual
        );

        // Resolve operands - U32 values occupy 2 slots
        let (left_off, right_off) = self.resolve_u32_operands(left, right)?;

        // Get the appropriate opcode
        let opcode = self.opcode_for_u32_op(op)?;

        // Emit the instruction
        let instr = InstructionBuilder::new(opcode)
            .with_operand(Operand::Literal(left_off))
            .with_operand(Operand::Literal(right_off))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(format!(
                "[fp + {}, fp + {}] = u32_op([fp + {}, fp + {}], [fp + {}, fp + {}])",
                dest_off,
                dest_off + 1,
                left_off,
                left_off + 1,
                right_off,
                right_off + 1
            ));
        self.instructions.push(instr);

        // Track memory writes
        if is_comparison {
            self.touch(dest_off, 1); // Comparison results are single felt
        } else {
            self.touch(dest_off, 2); // Arithmetic results are u32 (2 slots)
        }

        Ok(())
    }

    /// Resolve U32 operands to their base offsets
    fn resolve_u32_operands(&mut self, left: Value, right: Value) -> CodegenResult<(i32, i32)> {
        let left_off = self.resolve_u32_operand(left)?;
        let right_off = self.resolve_u32_operand(right)?;
        Ok((left_off, right_off))
    }

    /// Resolve a single U32 operand to its base offset
    fn resolve_u32_operand(&self, operand: Value) -> CodegenResult<i32> {
        match operand {
            Value::Operand(id) => {
                // Get the base offset for this U32 value
                self.layout.get_offset(id)
            }
            Value::Literal(_) => {
                // U32 literals are not yet supported in MIR
                // When they are added, we'll need to:
                // 1. Reserve 2 temporary slots
                // 2. Split the u32 into low and high parts
                // 3. Store them using store_immediate
                // 4. Return the base offset
                Err(CodegenError::UnsupportedInstruction(
                    "U32 literal values are not yet supported".to_string(),
                ))
            }
            _ => Err(CodegenError::UnsupportedInstruction(
                "Unsupported U32 operand type".to_string(),
            )),
        }
    }

    /// Get the opcode for a U32 operation
    fn opcode_for_u32_op(&self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::U32Add => Ok(U32_STORE_ADD_FP_FP),
            BinaryOp::U32Sub => Ok(U32_STORE_SUB_FP_FP),
            BinaryOp::U32Mul => Ok(U32_STORE_MUL_FP_FP),
            BinaryOp::U32Div => Ok(U32_STORE_DIV_FP_FP),
            // For comparison operations, we need to implement them using the arithmetic opcodes
            // and conditional jumps, similar to how we handle felt comparisons
            BinaryOp::U32Eq
            | BinaryOp::U32Neq
            | BinaryOp::U32Less
            | BinaryOp::U32Greater
            | BinaryOp::U32LessEqual
            | BinaryOp::U32GreaterEqual => Err(CodegenError::UnsupportedInstruction(
                "U32 comparison operations need special handling".to_string(),
            )),
            // Non-U32 operations should not reach here
            _ => Err(CodegenError::InvalidMir(format!(
                "Expected U32 operation, got {:?}",
                op
            ))),
        }
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
        // M is the total number of slots occupied by arguments
        let m: usize = signature.param_types.iter().map(|ty| ty.size_units()).sum();
        let k = signature.return_types.len();

        // Step 2: Reserve space for return values and map the destination `ValueId`.
        // The first return value will be placed at `[fp_c + args_offset + M]`.
        let return_value_offset = args_offset + m as i32;
        self.layout.map_value(dest, return_value_offset);
        self.layout.reserve_stack(k);

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
        // M is the total number of slots occupied by arguments
        let m: usize = signature.param_types.iter().map(|ty| ty.size_units()).sum();
        let k = dests.len();

        // Step 2: Reserve space for return values and map each destination ValueId.
        // Return values are placed after the arguments
        for (i, dest) in dests.iter().enumerate() {
            let return_value_offset = args_offset + m as i32 + i as i32;
            self.layout.map_value(*dest, return_value_offset);
        }
        self.layout.reserve_stack(k);

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
        // M is the total number of slots occupied by arguments
        let m: usize = signature.param_types.iter().map(|ty| ty.size_units()).sum();
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
            current_offset += param_type.size_units() as i32;
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
                            let size = param_type.size_units();

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
                                signature.param_types.iter().map(|ty| ty.size_units()).sum();
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
            let arg_size = param_type.size_units();

            match arg {
                Value::Literal(Literal::Integer(imm)) => {
                    // For single-slot types, store directly
                    if arg_size == 1 {
                        self.store_immediate(
                            *imm,
                            arg_offset,
                            format!("Arg {i}: [fp + {arg_offset}] = {imm}"),
                        );
                    } else {
                        // For multi-slot types, we need special handling
                        // For now, error out as we don't support multi-slot literals
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Multi-slot literal arguments not yet supported (size={})",
                            arg_size
                        )));
                    }
                }
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
    pub fn return_values(&mut self, values: &[Value]) -> CodegenResult<()> {
        let k = self.layout.num_return_slots() as i32;

        // Store each return value in its designated slot
        let mut cumulative_slot_offset = 0;
        for (i, return_val) in values.iter().enumerate() {
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
                        let instr = InstructionBuilder::new(STORE_IMM)
                            .with_operand(Operand::Literal(*imm))
                            .with_operand(Operand::Literal(return_slot_offset))
                            .with_comment(format!(
                                "Return value {}: [fp {}] = {}",
                                i, return_slot_offset, imm
                            ));
                        self.instructions.push(instr);
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
                    _ => 1, // Literals are single-slot for now
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
    /// Translates `dest = *address` to `[fp + dest_off] = [[fp + addr_off]]`.
    /// This uses the `store_double_deref_fp` opcode.
    /// TODO: check with VM opcode if this is the expected, desired behavior.
    pub fn load(&mut self, _dest: ValueId, _address: Value) -> CodegenResult<()> {
        todo!("Load is not implemented yet");
        // let layout = self
        //     .layout
        //     .as_mut()
        //     .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        // let dest_off = layout.allocate_local(dest, 1)?;

        // let addr_off = match address {
        //     Value::Operand(id) => layout.get_offset(id)?,
        //     _ => {
        //         return Err(CodegenError::UnsupportedInstruction(
        //             "Load address must be an operand".to_string(),
        //         ))
        //     }
        // };

        // let instr = InstructionBuilder::new(opcodes::STORE_DOUBLE_DEREF_FP)
        //     .with_base_off(addr_off)
        //     .with_offset(Operand::Literal(0)) // No inner offset for simple dereference
        //     .with_dst_off(dest_off)
        //     .with_comment(format!("[fp + {dest_off}] = [[fp + {addr_off}]]"));

        // self.instructions.push(instr);
        // Ok(())
    }

    /// Generate a get element pointer instruction
    ///
    /// Translates `dest = getelementptr base, offset` to an addition.
    pub fn get_element_ptr(
        &mut self,
        _dest: ValueId,
        _base: Value,
        _offset: Value,
    ) -> CodegenResult<()> {
        todo!("Get element pointer is not implemented yet");
        // let layout = self
        //     .layout
        //     .as_mut()
        //     .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;
        // let dest_off = layout.allocate_local(dest, 1)?;

        // self.generate_arithmetic_op(
        //     opcodes::STORE_ADD_FP_FP,
        //     opcodes::STORE_ADD_FP_IMM,
        //     dest_off,
        //     base,
        //     offset,
        // )
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
    pub fn allocate_stack(&mut self, dest: ValueId, size: usize) -> CodegenResult<()> {
        // Allocate the requested size
        let _dest_off = self.layout.allocate_local(dest, size)?;

        // StackAlloc doesn't generate actual instructions, it just reserves space
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
                    Value::Literal(Literal::Integer(imm)) => {
                        self.store_immediate(
                            imm,
                            dest_offset,
                            format!("Store immediate: [fp + {dest_offset}] = {imm}"),
                        );
                    }

                    Value::Operand(val_id) => {
                        let val_offset = self.layout.get_offset(val_id)?;

                        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM) // StoreAddFpImm
                            .with_operand(Operand::Literal(val_offset))
                            .with_operand(Operand::Literal(0))
                            .with_operand(Operand::Literal(dest_offset))
                            .with_comment(format!(
                                "Store: [fp + {dest_offset}] = [fp + {val_offset}] + 0"
                            ));

                        self.instructions.push(instr);
                        self.touch(dest_offset, 1);
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
        let temp_var_offset = self.layout.reserve_stack(1);
        let temp_var_offset2 = self.layout.reserve_stack(1);

        let mut new_instructions = Vec::new();
        // Track how instruction indices change: original_index -> new_index_range
        let mut index_mapping: Vec<Option<std::ops::Range<usize>>> = Vec::new();

        for instr in self.instructions.iter() {
            let current_new_index = new_instructions.len();

            let replacement_instructions = match instr.opcode {
                STORE_ADD_FP_FP | STORE_SUB_FP_FP | STORE_MUL_FP_FP | STORE_DIV_FP_FP => {
                    Self::handle_fp_fp_duplicates(instr, temp_var_offset, temp_var_offset2)
                }
                STORE_ADD_FP_IMM | STORE_SUB_FP_IMM | STORE_MUL_FP_IMM | STORE_DIV_FP_IMM => {
                    Self::handle_fp_imm_duplicates(instr, temp_var_offset)?
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
