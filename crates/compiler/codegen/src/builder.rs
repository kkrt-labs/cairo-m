//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.

use cairo_m_common::Opcode;
use cairo_m_compiler_mir::{Literal, Value, ValueId};
use cairo_m_compiler_parser::parser::{BinaryOp, UnaryOp};
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
    layout: Option<FunctionLayout>,
    /// Counter for generating unique labels
    label_counter: usize,
}

impl CasmBuilder {
    /// Create a new CASM builder
    pub const fn new(label_counter: usize) -> Self {
        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            layout: None,
            label_counter,
        }
    }

    /// Set the function layout for this builder
    pub fn with_layout(mut self, layout: FunctionLayout) -> Self {
        self.layout = Some(layout);
        self
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            layout.map_value(dest, offset);
            offset
        } else {
            // Allocate a new local variable
            layout.allocate_local(dest, 1)?
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Store immediate value
                let instr = InstructionBuilder::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(imm)
                    .with_comment(format!("[fp + {dest_off}] = {imm}"));

                self.instructions.push(instr);
            }

            Value::Operand(src_id) => {
                // Copy from another value using StoreAddFpImm with imm=0
                let src_off = layout.get_offset(src_id)?;

                let instr = InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(src_off)
                    .with_imm(0)
                    .with_off2(dest_off)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {src_off}] + 0"));

                self.instructions.push(instr);
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            layout.map_value(dest, offset);
            offset
        } else {
            // Allocate a new local variable
            layout.allocate_local(dest, 1)?
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            layout.map_value(dest, offset);
            offset
        } else {
            // Allocate a new local variable
            layout.allocate_local(dest, 1)?
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
            BinaryOp::Add => Ok(Opcode::StoreAddFpFp.into()),
            BinaryOp::Sub => Ok(Opcode::StoreSubFpFp.into()),
            BinaryOp::Mul => Ok(Opcode::StoreMulFpFp.into()),
            BinaryOp::Div => Ok(Opcode::StoreDivFpFp.into()),
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Invalid binary operation: {op:?}"
            ))),
        }
    }

    pub fn fp_imm_opcode_for_binary_op(&mut self, op: BinaryOp) -> CodegenResult<u32> {
        match op {
            BinaryOp::Add => Ok(Opcode::StoreAddFpImm.into()),
            BinaryOp::Sub => Ok(Opcode::StoreSubFpImm.into()),
            BinaryOp::Mul => Ok(Opcode::StoreMulFpImm.into()),
            BinaryOp::Div => Ok(Opcode::StoreDivFpImm.into()),
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
        let layout = self.layout.as_mut().unwrap();

        match (&left, &right) {
            // Both operands are values: use fp_fp variant
            (Value::Operand(left_id), Value::Operand(right_id)) => {
                let left_off = layout.get_offset(*left_id)?;
                let right_off = layout.get_offset(*right_id)?;

                let instr = InstructionBuilder::new(self.fp_fp_opcode_for_binary_op(op)?)
                    .with_off0(left_off)
                    .with_off1(right_off)
                    .with_off2(dest_off)
                    .with_comment(format!(
                        "[fp + {dest_off}] = [fp + {left_off}] op [fp + {right_off}]"
                    ));

                self.instructions.push(instr);
            }

            // Left is value, right is immediate: use fp_imm variant
            (Value::Operand(left_id), Value::Literal(Literal::Integer(imm))) => {
                let left_off = layout.get_offset(*left_id)?;

                let instr = InstructionBuilder::new(self.fp_imm_opcode_for_binary_op(op)?)
                    .with_off0(left_off)
                    .with_off2(dest_off)
                    .with_imm(*imm)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {left_off}] op {imm}"));

                self.instructions.push(instr);
            }

            // Left is immediate, right is value: use fp_imm variant
            (Value::Literal(Literal::Integer(imm)), Value::Operand(right_id)) => {
                match op {
                    // For addition and multiplication, we can swap the operands
                    BinaryOp::Add | BinaryOp::Mul => {
                        let right_off = layout.get_offset(*right_id)?;
                        let instr = InstructionBuilder::new(self.fp_imm_opcode_for_binary_op(op)?)
                            .with_off0(right_off)
                            .with_off2(dest_off)
                            .with_imm(*imm)
                            .with_comment(format!(
                                "[fp + {dest_off}] = [fp + {right_off}] op {imm}"
                            ));
                        self.instructions.push(instr);
                    }
                    // For subtraction and division, we store the immediate in a temporary variable
                    // TODO: In the future we should add opcodes imm_fp_sub and imm_fp_div
                    BinaryOp::Sub | BinaryOp::Div => {
                        let right_off = layout.get_offset(*right_id)?;
                        // Allocate a new temporary slot for the immediate
                        let temp_off = layout.reserve_stack(1);

                        let copy_instr = InstructionBuilder::new(Opcode::StoreImm.into())
                            .with_off2(temp_off)
                            .with_imm(*imm)
                            .with_comment(format!("[fp + {temp_off}] = {imm}"));
                        self.instructions.push(copy_instr);

                        let instr = InstructionBuilder::new(self.fp_fp_opcode_for_binary_op(op)?)
                            .with_off0(temp_off)
                            .with_off1(right_off)
                            .with_off2(dest_off)
                            .with_comment(format!(
                                "[fp + {dest_off}] = [fp + {temp_off}] op [fp + {right_off}]"
                            ));
                        self.instructions.push(instr);
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

                let instr = InstructionBuilder::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(result as i32)
                    .with_comment(format!("[fp + {dest_off}] = {result}"));
                self.instructions.push(instr);
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
        let jnz_instr = InstructionBuilder::new(Opcode::JnzFpImm.into())
            .with_off0(dest_off)
            .with_operand(Operand::Label(not_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {not_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so left == right, set result to 1
        let set_false_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("[fp + {dest_off}] = 1"));
        self.instructions.push(set_false_instr);

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: not_equal label - set result to 0
        let not_equal_label_obj = Label::new(not_zero_label);
        self.add_label(not_equal_label_obj);

        let set_true_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment(format!("[fp + {dest_off}] = 0"));
        self.instructions.push(set_true_instr);

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
        let jnz_instr = InstructionBuilder::new(Opcode::JnzFpImm.into())
            .with_off0(dest_off)
            .with_operand(Operand::Label(non_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {non_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so left == right, set result to 0
        let set_false_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment(format!("[fp + {dest_off}] = 0"));
        self.instructions.push(set_false_instr);

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: non_zero label - set result to 1
        let non_zero_label_obj = Label::new(non_zero_label);
        self.add_label(non_zero_label_obj);

        let set_true_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("[fp + {dest_off}] = 1"));
        self.instructions.push(set_true_instr);

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
        let jnz_instr = InstructionBuilder::new(Opcode::JnzFpImm.into())
            .with_off0(dest_off)
            .with_operand(Operand::Label(non_zero_label.clone()))
            .with_comment(format!(
                "if [fp + {dest_off}] != 0, jump to {non_zero_label}"
            ));
        self.instructions.push(jnz_instr);

        // Step 4: If we reach here, temp == 0, so at least one operand was 0, set result to 0
        let set_false_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment(format!("[fp + {dest_off}] = 0"));
        self.instructions.push(set_false_instr);

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // Step 5: non_zero label - set result to 1
        let non_zero_label_obj = Label::new(non_zero_label);
        self.add_label(non_zero_label_obj);

        let set_true_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("[fp + {dest_off}] = 1"));
        self.instructions.push(set_true_instr);

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

        let layout = self.layout.as_mut().unwrap();

        // Step 1: Initialize result to 0
        let init_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment("Initialize OR result to 0".to_string());
        self.instructions.push(init_instr);

        // Step 2: Check left operand - if non-zero, jump to set result to 1
        match left {
            Value::Operand(left_id) => {
                let left_off = layout.get_offset(left_id)?;
                let jnz_left = InstructionBuilder::new(Opcode::JnzFpImm.into())
                    .with_off0(left_off)
                    .with_operand(Operand::Label(set_true_label.clone()))
                    .with_comment(format!(
                        "if [fp + {left_off}] != 0, jump to {set_true_label}"
                    ));
                self.instructions.push(jnz_left);
            }
            Value::Literal(Literal::Integer(imm)) => {
                // If left is a non-zero immediate, directly jump to set true
                if imm != 0 {
                    let jmp_true = InstructionBuilder::new(Opcode::JmpAbsImm.into())
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
                let right_off = layout.get_offset(right_id)?;
                let jnz_right = InstructionBuilder::new(Opcode::JnzFpImm.into())
                    .with_off0(right_off)
                    .with_operand(Operand::Label(set_true_label.clone()))
                    .with_comment(format!(
                        "if [fp + {right_off}] != 0, jump to {set_true_label}"
                    ));
                self.instructions.push(jnz_right);
            }
            Value::Literal(Literal::Integer(imm)) => {
                // If right is a non-zero immediate, directly jump to set true
                if imm != 0 {
                    let jmp_true = InstructionBuilder::new(Opcode::JmpAbsImm.into())
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
        let jmp_end = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end);

        // Step 5: set_true label - set result to 1
        self.add_label(Label::new(set_true_label));
        let set_true_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("[fp + {dest_off}] = 1"));
        self.instructions.push(set_true_instr);

        // Step 6: end label
        self.add_label(Label::new(end_label));

        Ok(())
    }

    pub fn generate_not_op(&mut self, dest_off: i32, source: Value) -> CodegenResult<()> {
        let set_zero_label = self.new_label_name("not_zero");
        let end_label = self.new_label_name("not_end");

        match source {
            Value::Operand(src_id) => {
                let src_off = self.layout.as_ref().unwrap().get_offset(src_id)?;
                // If source is non-zero, jump to set result to 0
                let jnz_instr = InstructionBuilder::new(Opcode::JnzFpImm.into())
                    .with_off0(src_off)
                    .with_operand(Operand::Label(set_zero_label.clone()))
                    .with_comment(format!(
                        "if [fp + {src_off}] != 0, jump to {set_zero_label}"
                    ));
                self.instructions.push(jnz_instr);
            }
            Value::Literal(Literal::Integer(imm)) => {
                // For immediate values, we can directly compute the NOT result
                let result = if imm == 0 { 1 } else { 0 };
                let instr = InstructionBuilder::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(result)
                    .with_comment(format!("[fp + {dest_off}] = {result}"));
                self.instructions.push(instr);
                return Ok(());
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported source operand in NOT".to_string(),
                ));
            }
        }

        // If we reach here, source was 0, so set result to 1
        let set_one_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("[fp + {dest_off}] = 1"));
        self.instructions.push(set_one_instr);

        // Jump to end
        let jmp_end_instr = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment(format!("jump to {end_label}"));
        self.instructions.push(jmp_end_instr);

        // set_zero label - set result to 0
        self.add_label(Label::new(set_zero_label));
        let set_zero_instr = InstructionBuilder::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment(format!("[fp + {dest_off}] = 0"));
        self.instructions.push(set_zero_instr);

        // end label
        self.add_label(Label::new(end_label));

        Ok(())
    }

    /// Generate a function call that returns a value.
    pub fn call(
        &mut self,
        dest: ValueId,
        callee_name: &str,
        args: &[Value],
        num_returns: usize,
    ) -> CodegenResult<()> {
        // Step 1: Pass arguments by storing them in the communication area.
        let l = self.pass_arguments(callee_name, args)?;
        let m = args.len();
        let k = num_returns;

        // Step 2: Reserve space for return values and map the destination `ValueId`.
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        // The first return value will be placed at `[fp_c + L + M]`.
        // TODO: Handle multiple return values by mapping each to its slot.
        let return_value_offset = l + m as i32;
        layout.map_value(dest, return_value_offset);
        layout.reserve_stack(k);

        // Step 3: Calculate `off0` and emit the `call` instruction.
        let off0 = l + m as i32 + k as i32;
        let instr = InstructionBuilder::new(Opcode::CallAbsImm.into())
            .with_off0(off0)
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
    ) -> CodegenResult<()> {
        // Step 1: Pass arguments by storing them in the communication area.
        let l = self.pass_arguments(callee_name, args)?;
        let m = args.len();
        let k = dests.len();

        // Step 2: Reserve space for return values and map each destination ValueId.
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        // Map each destination to its return value slot
        // Return value i is placed at [fp_c + L + M + i]
        for (i, dest) in dests.iter().enumerate() {
            let return_value_offset = l + m as i32 + i as i32;
            layout.map_value(*dest, return_value_offset);
        }
        layout.reserve_stack(k);

        // Step 3: Calculate `off0` and emit the `call` instruction.
        let off0 = l + m as i32 + k as i32;
        let instr = InstructionBuilder::new(Opcode::CallAbsImm.into())
            .with_off0(off0)
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
        num_returns: usize,
    ) -> CodegenResult<()> {
        let l = self.pass_arguments(callee_name, args)?;
        let m = args.len();
        let k = num_returns;

        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;
        layout.reserve_stack(k);

        let off0 = l + m as i32 + k as i32;
        let instr = InstructionBuilder::new(Opcode::CallAbsImm.into())
            .with_off0(off0)
            .with_operand(Operand::Label(callee_name.to_string()))
            .with_comment(format!("call {callee_name}"));
        self.instructions.push(instr);
        Ok(())
    }

    /// Helper to pass arguments for a function call.
    /// Returns the caller's frame usage (`L`) before placing arguments.
    fn pass_arguments(&mut self, _callee_name: &str, args: &[Value]) -> CodegenResult<i32> {
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let l = layout.current_frame_usage();

        // Optimization: Check if arguments are already positioned sequentially at the stack top.
        // This occurs when the last N stack values are exactly the arguments in order.
        // Example: With L=3 and args at [fp + 1], [fp + 2], we can avoid copying since
        // they're already positioned for the callee's frame layout.
        if !args.is_empty() {
            let args_start_offset = l - args.len() as i32;
            let args_in_place = args.iter().enumerate().all(|(i, arg)| {
                if let Value::Operand(arg_id) = arg {
                    layout
                        .get_offset(*arg_id)
                        .is_ok_and(|offset| offset == args_start_offset + i as i32)
                } else {
                    // Literals require explicit storage, preventing optimization
                    false
                }
            });

            if args_in_place {
                // Arguments are already correctly positioned - skip copying
                return Ok(args_start_offset);
            }
        }

        // Standard path: copy arguments to their positions
        for (i, arg) in args.iter().enumerate() {
            let arg_offset = l + i as i32; // Place i-th arg at `[fp_c + L + i]`.
            let instr = match arg {
                Value::Literal(Literal::Integer(imm)) => {
                    InstructionBuilder::new(Opcode::StoreImm.into())
                        .with_off2(arg_offset)
                        .with_imm(*imm)
                        .with_comment(format!("Arg {i}: [fp + {arg_offset}] = {imm}"))
                }
                Value::Operand(arg_id) => {
                    let src_off = layout.get_offset(*arg_id)?;
                    InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                        .with_off0(src_off)
                        .with_imm(0)
                        .with_off2(arg_offset)
                        .with_comment(format!(
                            "Arg {i}: [fp + {arg_offset}] = [fp + {src_off}] + 0"
                        ))
                }
                _ => {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Unsupported argument type".to_string(),
                    ));
                }
            };
            self.instructions.push(instr);
        }
        Ok(l)
    }

    /// Generate `return` instruction with multiple return values.
    pub fn return_values(&mut self, values: &[Value]) -> CodegenResult<()> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let k = layout.num_return_values() as i32;

        // Store each return value in its designated slot
        for (i, return_val) in values.iter().enumerate() {
            // Return value i goes to [fp - K - 2 + i]
            let return_slot_offset = -(k + 2) + i as i32;

            // Check if the value is already in the return slot (optimization for direct returns)
            let needs_copy = match return_val {
                Value::Operand(val_id) => {
                    let current_offset = layout.get_offset(*val_id).unwrap_or(0);
                    current_offset != return_slot_offset
                }
                _ => true, // Literals always need to be stored
            };

            if needs_copy {
                let instr = match return_val {
                    Value::Literal(Literal::Integer(imm)) => {
                        InstructionBuilder::new(Opcode::StoreImm.into())
                            .with_off2(return_slot_offset)
                            .with_imm(*imm)
                            .with_comment(format!(
                                "Return value {}: [fp {}] = {}",
                                i, return_slot_offset, imm
                            ))
                    }
                    Value::Operand(val_id) => {
                        let src_off = layout.get_offset(*val_id)?;
                        InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                            .with_off0(src_off)
                            .with_imm(0)
                            .with_off2(return_slot_offset)
                            .with_comment(format!(
                                "Return value {}: [fp {}] = [fp + {}] + 0",
                                i, return_slot_offset, src_off
                            ))
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported return value type".to_string(),
                        ));
                    }
                };
                self.instructions.push(instr);
            }
            // If !needs_copy, the value is already in the return slot, so we skip the copy
        }

        self.instructions
            .push(InstructionBuilder::new(Opcode::Ret.into()).with_comment("return".to_string()));
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
        //     .with_off0(addr_off)
        //     .with_off1(0) // No inner offset for simple dereference
        //     .with_off2(dest_off)
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
        let instr = InstructionBuilder::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("jump abs {target_label}"));

        self.instructions.push(instr);
        Ok(())
    }

    /// Generates a conditional jump instruction that triggers if the value at `cond_off` is non-zero.
    /// The `target_label` is a placeholder that will be resolved to a relative offset later.
    pub fn jnz(&mut self, condition: Value, target_label: &str) -> CodegenResult<()> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        // Get the condition value offset
        let cond_off = match condition {
            Value::Operand(cond_id) => layout.get_offset(cond_id)?,
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
        let instr = InstructionBuilder::new(Opcode::JnzFpImm.into())
            .with_off0(cond_off)
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        // Allocate the requested size
        let _dest_off = layout.allocate_local(dest, size)?;

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
    pub const fn layout_mut(&mut self) -> Option<&mut FunctionLayout> {
        self.layout.as_mut()
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
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        match address {
            Value::Operand(addr_id) => {
                // The address is actually the location where we want to store
                let dest_offset = layout.get_offset(addr_id)?;

                match value {
                    Value::Literal(Literal::Integer(imm)) => {
                        let instr = InstructionBuilder::new(Opcode::StoreImm.into())
                            .with_off2(dest_offset)
                            .with_imm(imm)
                            .with_comment(format!("Store immediate: [fp + {dest_offset}] = {imm}"));

                        self.instructions.push(instr);
                    }

                    Value::Operand(val_id) => {
                        let val_offset = layout.get_offset(val_id)?;

                        let instr = InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                            .with_off0(val_offset)
                            .with_imm(0)
                            .with_off2(dest_offset)
                            .with_comment(format!(
                                "Store: [fp + {dest_offset}] = [fp + {val_offset}] + 0"
                            ));

                        self.instructions.push(instr);
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let temp_var_offset = layout.reserve_stack(1);
        let temp_var_offset2 = layout.reserve_stack(1);

        let mut new_instructions = Vec::new();
        // Track how instruction indices change: original_index -> new_index_range
        let mut index_mapping: Vec<Option<std::ops::Range<usize>>> = Vec::new();

        for instr in self.instructions.iter() {
            let current_new_index = new_instructions.len();

            let replacement_instructions = match Opcode::from_u32(instr.opcode).unwrap() {
                Opcode::StoreAddFpFp
                | Opcode::StoreSubFpFp
                | Opcode::StoreMulFpFp
                | Opcode::StoreDivFpFp => {
                    Self::handle_fp_fp_duplicates(instr, temp_var_offset, temp_var_offset2)
                }
                Opcode::StoreAddFpImm
                | Opcode::StoreSubFpImm
                | Opcode::StoreMulFpImm
                | Opcode::StoreDivFpImm => Self::handle_fp_imm_duplicates(instr, temp_var_offset)?,
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
        let off0 = instr.off0.unwrap();
        let off1 = instr.off1.unwrap();
        let off2 = instr.off2.unwrap();

        if off0 == off1 && off1 == off2 {
            // The three offsets are the same, store off0 and off1 in temp vars and replace with 3 instructions
            vec![
                InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(off0)
                    .with_imm(0)
                    .with_off2(temp_var_offset)
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(off1)
                    .with_imm(0)
                    .with_off2(temp_var_offset2)
                    .with_comment(format!("[fp + {temp_var_offset2}] = [fp + {off1}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_off2(off2)
                    .with_off0(temp_var_offset)
                    .with_off1(temp_var_offset2)
                    .with_comment(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {temp_var_offset2}]"
                    )),
            ]
        } else if off0 == off1 || off0 == off2 {
            // off0 is a duplicate, store off0 in a temp var and replace with 2 instructions
            vec![
                InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(off0)
                    .with_imm(0)
                    .with_off2(temp_var_offset)
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_off2(off2)
                    .with_off0(temp_var_offset)
                    .with_off1(off1)
                    .with_comment(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {off1}]"
                    )),
            ]
        } else if off1 == off2 {
            // off1 is a duplicate, store off1 in a temp var and replace with 2 instructions
            vec![
                InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(off1)
                    .with_imm(0)
                    .with_off2(temp_var_offset)
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off1}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_off2(off2)
                    .with_off0(off0)
                    .with_off1(temp_var_offset)
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
        let off0 = instr.off0.unwrap();
        let off2 = instr.off2.unwrap();

        let imm = match instr.operand.clone().unwrap() {
            Operand::Literal(imm) => imm,
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Store immediate operand must be a literal".to_string(),
                ));
            }
        };

        if off0 == off2 {
            // off0 is a duplicate, store off0 in a temp var and replace with 2 instructions
            Ok(vec![
                InstructionBuilder::new(Opcode::StoreAddFpImm.into())
                    .with_off0(off0)
                    .with_imm(0)
                    .with_off2(temp_var_offset)
                    .with_comment(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                InstructionBuilder::new(instr.opcode)
                    .with_off2(off2)
                    .with_off0(temp_var_offset)
                    .with_imm(imm)
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

    #[test]
    fn test_handle_fp_fp_duplicates_all_same() {
        // Create instruction: [fp + 5] = [fp + 5] + [fp + 5]
        let instr = InstructionBuilder::new(Opcode::StoreAddFpFp.into())
            .with_off0(5)
            .with_off1(5)
            .with_off2(5);

        let temp1 = 10;
        let temp2 = 11;
        let result = CasmBuilder::handle_fp_fp_duplicates(&instr, temp1, temp2);
        assert_eq!(result.len(), 3, "Should expand to 3 instructions");

        // Check that we use temp variables
        assert_eq!(result[0].off2, Some(temp1));
        assert_eq!(result[1].off2, Some(temp2));
        assert_eq!(result[2].off0, Some(temp1));
        assert_eq!(result[2].off1, Some(temp2));
    }

    #[test]
    fn test_handle_fp_fp_duplicates_first_operand_conflict() {
        // Create instruction: [fp + 5] = [fp + 5] + [fp + 3]
        let instr = InstructionBuilder::new(Opcode::StoreAddFpFp.into())
            .with_off0(5)
            .with_off1(3)
            .with_off2(5);

        let temp1 = 10;
        let temp2 = 11;
        let result = CasmBuilder::handle_fp_fp_duplicates(&instr, temp1, temp2);
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that first operand is copied to temp
        assert_eq!(result[0].off2, Some(temp1));
        assert_eq!(result[0].off0, Some(5));
        assert_eq!(result[1].off0, Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_in_place() {
        // Create instruction: [fp + 5] = [fp + 5] + 42
        let instr = InstructionBuilder::new(Opcode::StoreAddFpImm.into())
            .with_off0(5)
            .with_off2(5)
            .with_operand(Operand::Literal(42));

        let temp1 = 10;
        let result = CasmBuilder::handle_fp_imm_duplicates(&instr, temp1).unwrap();
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that source is copied to temp first
        assert_eq!(result[0].off2, Some(temp1));
        assert_eq!(result[0].off0, Some(5));
        assert_eq!(result[1].off0, Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_no_conflict() {
        // Create instruction: [fp + 7] = [fp + 5] + 42 (no conflict)
        let instr = InstructionBuilder::new(Opcode::StoreAddFpImm.into())
            .with_off0(5)
            .with_off2(7)
            .with_operand(Operand::Literal(42));

        let temp1 = 10;
        let result = CasmBuilder::handle_fp_imm_duplicates(&instr, temp1).unwrap();
        assert_eq!(result.len(), 1, "Should keep as single instruction");

        // Should be unchanged
        assert_eq!(result[0].off0, Some(5));
        assert_eq!(result[0].off2, Some(7));
    }
}
