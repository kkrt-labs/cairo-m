//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.

use cairo_m_compiler_mir::{Literal, Value, ValueId};
use cairo_m_compiler_parser::parser::BinaryOp;

use crate::{CasmInstruction, CodegenError, CodegenResult, FunctionLayout, Label, Opcode, Operand};

/// Builder for generating CASM instructions
///
/// This struct manages the generation of CASM instructions, handling the
/// translation from MIR values to fp-relative memory addresses.
#[derive(Debug)]
pub struct CasmBuilder {
    /// Generated instructions
    instructions: Vec<CasmInstruction>,
    /// Labels that need to be resolved
    labels: Vec<Label>,
    /// Current function layout for offset lookups
    layout: Option<FunctionLayout>,
}

impl CasmBuilder {
    /// Create a new CASM builder
    pub const fn new() -> Self {
        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            layout: None,
        }
    }

    /// Set the function layout for this builder
    pub fn with_layout(mut self, layout: FunctionLayout) -> Self {
        self.layout = Some(layout);
        self
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
                let instr = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(imm)
                    .with_comment(format!("[fp + {dest_off}] = {imm}"));

                self.instructions.push(instr);
            }

            Value::Operand(src_id) => {
                // Copy from another value
                let src_off = layout.get_offset(src_id)?;

                let instr = CasmInstruction::new(Opcode::StoreDerefFp.into())
                    .with_off0(src_off)
                    .with_off2(dest_off)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {src_off}]"));

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
            BinaryOp::Add => {
                self.generate_arithmetic_op(
                    Opcode::StoreAddFpFp.into(),
                    Opcode::StoreAddFpImm.into(),
                    dest_off,
                    left,
                    right,
                )?;
            }
            BinaryOp::Sub => {
                self.generate_arithmetic_op(
                    Opcode::StoreSubFpFp.into(),
                    Opcode::StoreSubFpImm.into(),
                    dest_off,
                    left,
                    right,
                )?;
            }
            BinaryOp::Mul => {
                self.generate_arithmetic_op(
                    Opcode::StoreMulFpFp.into(),
                    Opcode::StoreMulFpImm.into(),
                    dest_off,
                    left,
                    right,
                )?;
            }
            BinaryOp::Div => {
                self.generate_arithmetic_op(
                    Opcode::StoreDivFpFp.into(),
                    Opcode::StoreDivFpImm.into(),
                    dest_off,
                    left,
                    right,
                )?;
            }
            BinaryOp::Eq => {
                // Generate unique labels for this equality check
                let eq_false_label = format!("eq_false_{}", dest.index());
                let eq_end_label = format!("eq_end_{}", dest.index());

                // First, compute the difference (a - b) and get the offset where it's stored
                let check_offset = self.generate_equality_check(dest_off, left, right)?;

                // Create a temporary value for the subtraction result if needed
                let _layout = self
                    .layout
                    .as_mut()
                    .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

                // If check_offset != dest_off (happens for x == 0 optimizations),
                // we need to use a different value for the jnz
                let check_value = if check_offset != dest_off {
                    // For optimized cases like x == 0, check_offset points to x directly
                    // Find the ValueId that maps to check_offset
                    // For now, we'll create a synthetic value or use the original
                    match (&left, &right) {
                        (Value::Operand(id), Value::Literal(Literal::Integer(0)))
                        | (Value::Literal(Literal::Integer(0)), Value::Operand(id)) => {
                            Value::Operand(*id)
                        }
                        _ => Value::Operand(dest),
                    }
                } else {
                    Value::Operand(dest)
                };

                // If the difference is non-zero (a != b), jump to false case
                self.jnz(check_value, &eq_false_label)?;

                // a == b, so store 1
                let instr = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(1)
                    .with_comment(format!("[fp + {}] = 1", dest_off));
                self.add_instruction(instr);

                // Jump to end
                self.jump(&eq_end_label)?;

                // Add label for false case
                self.add_label(Label::new(eq_false_label));

                // a != b, so store 0
                let instr = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(0)
                    .with_comment(format!("[fp + {}] = 0", dest_off));
                self.add_instruction(instr);

                // Add end label
                self.add_label(Label::new(eq_end_label));
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Binary operation {op:?} not implemented"
                )));
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

    /// Generate arithmetic operation (add, sub, mul, div)
    pub fn generate_arithmetic_op(
        &mut self,
        fp_fp_opcode: u32,
        fp_imm_opcode: u32,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        let layout = self.layout.as_ref().unwrap();

        match (&left, &right) {
            // Both operands are values: use fp_fp variant
            (Value::Operand(left_id), Value::Operand(right_id)) => {
                let left_off = layout.get_offset(*left_id)?;
                let right_off = layout.get_offset(*right_id)?;

                let instr = CasmInstruction::new(fp_fp_opcode)
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

                let instr = CasmInstruction::new(fp_imm_opcode)
                    .with_off0(left_off)
                    .with_off2(dest_off)
                    .with_imm(*imm)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {left_off}] op {imm}"));

                self.instructions.push(instr);
            }

            // For right operand being a value and left being immediate, we'd need to reverse
            // the operation or use different opcodes. For now, mark as unsupported.
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Immediate as left operand not supported".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Generate equality check
    ///
    /// Equality is implemented as: result = (a - b == 0 ? 1 : 0)
    /// This is a simplified implementation using subtraction
    fn generate_equality_check(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<i32> {
        // For now, implement equality as a subtraction and let the caller handle the comparison
        // A more sophisticated implementation would use conditional logic
        self.generate_arithmetic_op(
            Opcode::StoreSubFpFp.into(),
            Opcode::StoreSubFpImm.into(),
            dest_off,
            left,
            right,
        )?;
        let check_offset = dest_off;

        // Add a comment to indicate this is an equality check
        if let Some(last_instr) = self.instructions.last_mut() {
            let layout = self
                .layout
                .as_ref()
                .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;
            let left_str = match left {
                Value::Operand(id) => match layout.get_offset(id) {
                    Ok(off) => format!("[fp + {off}]"),
                    Err(_) => format!("%{}", id.index()),
                },
                Value::Literal(lit) => match lit {
                    Literal::Integer(val) => val.to_string(),
                    Literal::Boolean(val) => val.to_string(),
                    Literal::Unit => "()".to_string(),
                },
                Value::Error => "error".to_string(),
            };

            let right_str = match right {
                Value::Operand(id) => match layout.get_offset(id) {
                    Ok(off) => format!("[fp + {off}]"),
                    Err(_) => format!("%{}", id.index()),
                },
                Value::Literal(lit) => match lit {
                    Literal::Integer(val) => val.to_string(),
                    Literal::Boolean(val) => val.to_string(),
                    Literal::Unit => "()".to_string(),
                },
                Value::Error => "error".to_string(),
            };

            last_instr.comment = Some(format!(
                "Equality check: [fp + {dest_off}] = {left_str} - {right_str}"
            ));
        }

        Ok(check_offset)
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
        let instr = CasmInstruction::new(Opcode::CallAbsImm.into())
            .with_off0(off0)
            .with_operand(Operand::Label(callee_name.to_string()))
            .with_comment(format!("call {callee_name}"));
        self.instructions.push(instr);

        // Step 4: No copy is needed after the call. The `dest` ValueId is already mapped
        // to the correct stack slot where the callee will place the return value.

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
        let instr = CasmInstruction::new(Opcode::CallAbsImm.into())
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

        for (i, arg) in args.iter().enumerate() {
            let arg_offset = l + i as i32; // Place i-th arg at `[fp_c + L + i]`.
            let instr = match arg {
                Value::Literal(Literal::Integer(imm)) => {
                    CasmInstruction::new(Opcode::StoreImm.into())
                        .with_off2(arg_offset)
                        .with_imm(*imm)
                        .with_comment(format!("Arg {i}: [fp + {arg_offset}] = {imm}"))
                }
                Value::Operand(arg_id) => {
                    let src_off = layout.get_offset(*arg_id)?;
                    CasmInstruction::new(Opcode::StoreDerefFp.into())
                        .with_off0(src_off)
                        .with_off2(arg_offset)
                        .with_comment(format!("Arg {i}: [fp + {arg_offset}] = [fp + {src_off}]"))
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

    /// Generate `return` instruction.
    pub fn return_value(&mut self, value: Option<Value>) -> CodegenResult<()> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        if let Some(return_val) = value {
            let k = layout.num_return_values();
            if k > 0 {
                // TODO: Support multiple return values. For now, assume k=1.
                // The first (and only) return value goes to `[fp - K - 2 + 0] = [fp - 3]`.
                let return_slot_offset = -3;

                // Check if the value is already in the return slot (optimization for direct returns)
                let needs_copy = match return_val {
                    Value::Operand(val_id) => {
                        let current_offset = layout.get_offset(val_id).unwrap_or(0);
                        current_offset != return_slot_offset
                    }
                    _ => true, // Literals always need to be stored
                };

                if needs_copy {
                    let instr = match return_val {
                        Value::Literal(Literal::Integer(imm)) => {
                            CasmInstruction::new(Opcode::StoreImm.into())
                                .with_off2(return_slot_offset)
                                .with_imm(imm)
                                .with_comment(format!("Return value: [fp - 3] = {imm}"))
                        }
                        Value::Operand(val_id) => {
                            let src_off = layout.get_offset(val_id)?;
                            CasmInstruction::new(Opcode::StoreDerefFp.into())
                                .with_off0(src_off)
                                .with_off2(return_slot_offset)
                                .with_comment(format!("Return value: [fp - 3] = [fp + {src_off}]"))
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
        }

        self.instructions
            .push(CasmInstruction::new(Opcode::Ret.into()).with_comment("return".to_string()));
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

        // let instr = CasmInstruction::new(opcodes::STORE_DOUBLE_DEREF_FP)
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
        let instr = CasmInstruction::new(Opcode::JmpAbsImm.into())
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
        let instr = CasmInstruction::new(Opcode::JnzFpImm.into())
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
    pub fn add_instruction(&mut self, instruction: CasmInstruction) {
        self.instructions.push(instruction);
    }

    /// Get the generated instructions
    pub fn instructions(&self) -> &[CasmInstruction] {
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

    /// Take ownership of the generated instructions
    pub fn into_instructions(self) -> Vec<CasmInstruction> {
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
                        let instr = CasmInstruction::new(Opcode::StoreImm.into())
                            .with_off2(dest_offset)
                            .with_imm(imm)
                            .with_comment(format!("Store immediate: [fp + {dest_offset}] = {imm}"));

                        self.instructions.push(instr);
                    }

                    Value::Operand(val_id) => {
                        let val_offset = layout.get_offset(val_id)?;

                        let instr = CasmInstruction::new(Opcode::StoreDerefFp.into())
                            .with_off0(val_offset)
                            .with_off2(dest_offset)
                            .with_comment(format!(
                                "Store: [fp + {dest_offset}] = [fp + {val_offset}]"
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
}

impl Default for CasmBuilder {
    fn default() -> Self {
        Self::new()
    }
}
