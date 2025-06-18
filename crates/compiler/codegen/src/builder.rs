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
    /// Counter for generating unique labels
    label_counter: usize,
}

impl CasmBuilder {
    /// Create a new CASM builder
    pub fn new(label_counter: usize) -> Self {
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
            BinaryOp::Eq => self.generate_equals_op(dest_off, left, right)?,
            BinaryOp::Neq => self.generate_arithmetic_op(
                Opcode::StoreSubFpFp.into(),
                Opcode::StoreSubFpImm.into(),
                dest_off,
                left,
                right,
            )?,
            BinaryOp::And => {
                self.generate_arithmetic_op(
                    Opcode::StoreMulFpFp.into(),
                    Opcode::StoreMulFpImm.into(),
                    dest_off,
                    left,
                    right,
                )?;
            }
            BinaryOp::Or => {
                self.generate_or_op(dest_off, left, right)?;
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

    pub fn generate_equals_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        let layout = self.layout.as_mut().unwrap();

        // Step 1: Allocate a temporary local for the difference
        let temp_off = layout.allocate_local(ValueId::from_raw(u32::MAX as usize), 1)?;

        // Step 2: Compute left - right into the temporary
        self.binary_op(
            BinaryOp::Sub,
            ValueId::from_raw(u32::MAX as usize),
            left,
            right,
        )?;

        // Step 3: Generate unique labels for this equality check
        let label_id = self.label_counter;
        self.label_counter += 1;
        let not_zero_label = format!("not_zero_{}", label_id);
        let end_label = format!("end_{}", label_id);

        // Step 4: Check if temp == 0 (meaning left == right)
        // jnz jumps if non-zero, so if temp != 0, jump to set result to 0 (or 1 if not equal)
        let jnz_instr = CasmInstruction::new(Opcode::JnzFpImm.into())
            .with_off0(temp_off)
            .with_operand(Operand::Label(not_zero_label.clone()))
            .with_comment("if temp != 0, jump to not_zero".to_string());
        self.instructions.push(jnz_instr);

        // Step 5: If we reach here, temp == 0, so left == right, set result to 1
        let set_false_instr = CasmInstruction::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment(format!("Set [fp + {dest_off}] to 1"));
        self.instructions.push(set_false_instr);

        // Jump to end
        let jmp_end_instr = CasmInstruction::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment("jump to end".to_string());
        self.instructions.push(jmp_end_instr);

        // Step 6: not_equal label - set result to 0
        let not_equal_label_obj = Label::new(not_zero_label);
        self.add_label(not_equal_label_obj);

        let set_true_instr = CasmInstruction::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(0)
            .with_comment(format!("Set [fp + {dest_off}] to 0"));
        self.instructions.push(set_true_instr);

        // Step 7: end label
        let end_label_obj = Label::new(end_label);
        self.add_label(end_label_obj);

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

    /// Get the label counter
    pub fn label_counter(&self) -> usize {
        self.label_counter
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

    pub fn generate_or_op(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        let layout = self.layout.as_mut().unwrap();

        // Step 1: Allocate temporaries for the boolean conversions
        let left_bool_off = layout.allocate_local(ValueId::from_raw(u32::MAX as usize), 1)?;
        let right_bool_off =
            layout.allocate_local(ValueId::from_raw((u32::MAX - 1) as usize), 1)?;
        let sum_off = layout.allocate_local(ValueId::from_raw((u32::MAX - 2) as usize), 1)?;
        let one_off = layout.allocate_local(ValueId::from_raw((u32::MAX - 3) as usize), 1)?;

        // Get offsets for operands that are values
        let left_off = match left {
            Value::Operand(left_id) => Some(layout.get_offset(left_id)?),
            _ => None,
        };
        let right_off = match right {
            Value::Operand(right_id) => Some(layout.get_offset(right_id)?),
            _ => None,
        };

        // Generate unique labels
        let label_id = self.label_counter;
        self.label_counter += 1;
        let left_true_label = format!("left_true_{}", label_id);
        let left_end_label = format!("left_end_{}", label_id);
        let right_true_label = format!("right_true_{}", label_id);
        let right_end_label = format!("right_end_{}", label_id);
        let clamp_label = format!("clamp_{}", label_id);
        let end_label = format!("end_{}", label_id);

        // Step 2: Convert left operand to boolean (0 or 1)
        match left {
            Value::Operand(_) => {
                let left_off = left_off.unwrap();

                // Test if left is non-zero
                let jnz_left = CasmInstruction::new(Opcode::JnzFpImm.into())
                    .with_off0(left_off)
                    .with_operand(Operand::Label(left_true_label.clone()))
                    .with_comment("if left != 0, jump to set left_bool = 1".to_string());
                self.instructions.push(jnz_left);

                // Left is zero, set left_bool = 0
                let set_left_false = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(left_bool_off)
                    .with_imm(0)
                    .with_comment("left_bool = 0".to_string());
                self.instructions.push(set_left_false);

                // Jump to end of left processing
                let jmp_left_end = CasmInstruction::new(Opcode::JmpAbsImm.into())
                    .with_operand(Operand::Label(left_end_label.clone()))
                    .with_comment("jump to left_end".to_string());
                self.instructions.push(jmp_left_end);

                // left_true label: set left_bool = 1
                self.add_label(Label::new(left_true_label));
                let set_left_true = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(left_bool_off)
                    .with_imm(1)
                    .with_comment("left_bool = 1".to_string());
                self.instructions.push(set_left_true);

                // left_end label
                self.add_label(Label::new(left_end_label));
            }
            Value::Literal(Literal::Integer(imm)) => {
                // Left is immediate, convert directly
                let left_bool = if imm != 0 { 1 } else { 0 };
                let set_left = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(left_bool_off)
                    .with_imm(left_bool)
                    .with_comment(format!("left_bool = {}", left_bool));
                self.instructions.push(set_left);
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported left operand in OR".to_string(),
                ));
            }
        }

        // Step 3: Convert right operand to boolean (0 or 1)
        match right {
            Value::Operand(_) => {
                let right_off = right_off.unwrap();

                // Test if right is non-zero
                let jnz_right = CasmInstruction::new(Opcode::JnzFpImm.into())
                    .with_off0(right_off)
                    .with_operand(Operand::Label(right_true_label.clone()))
                    .with_comment("if right != 0, jump to set right_bool = 1".to_string());
                self.instructions.push(jnz_right);

                // Right is zero, set right_bool = 0
                let set_right_false = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(right_bool_off)
                    .with_imm(0)
                    .with_comment("right_bool = 0".to_string());
                self.instructions.push(set_right_false);

                // Jump to end of right processing
                let jmp_right_end = CasmInstruction::new(Opcode::JmpAbsImm.into())
                    .with_operand(Operand::Label(right_end_label.clone()))
                    .with_comment("jump to right_end".to_string());
                self.instructions.push(jmp_right_end);

                // right_true label: set right_bool = 1
                self.add_label(Label::new(right_true_label));
                let set_right_true = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(right_bool_off)
                    .with_imm(1)
                    .with_comment("right_bool = 1".to_string());
                self.instructions.push(set_right_true);

                // right_end label
                self.add_label(Label::new(right_end_label));
            }
            Value::Literal(Literal::Integer(imm)) => {
                // Right is immediate, convert directly
                let right_bool = if imm != 0 { 1 } else { 0 };
                let set_right = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(right_bool_off)
                    .with_imm(right_bool)
                    .with_comment(format!("right_bool = {}", right_bool));
                self.instructions.push(set_right);
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported right operand in OR".to_string(),
                ));
            }
        }

        // Step 4: Add the boolean values
        let add_instr = CasmInstruction::new(Opcode::StoreAddFpFp.into())
            .with_off0(left_bool_off)
            .with_off1(right_bool_off)
            .with_off2(sum_off)
            .with_comment("sum = left_bool + right_bool".to_string());
        self.instructions.push(add_instr);

        // Step 5: Check if sum > 1, if so clamp to 1
        // Load immediate 1 for comparison
        let load_one = CasmInstruction::new(Opcode::StoreImm.into())
            .with_off2(one_off)
            .with_imm(1)
            .with_comment("temp = 1".to_string());
        self.instructions.push(load_one);

        // Subtract 1 from sum to test if > 1
        let sub_instr = CasmInstruction::new(Opcode::StoreSubFpFp.into())
            .with_off0(sum_off)
            .with_off1(one_off)
            .with_off2(one_off) // Reuse the temp slot
            .with_comment("temp = sum - 1".to_string());
        self.instructions.push(sub_instr);

        // If temp > 0 (i.e., sum > 1), jump to clamp
        let jnz_clamp = CasmInstruction::new(Opcode::JnzFpImm.into())
            .with_off0(one_off)
            .with_operand(Operand::Label(clamp_label.clone()))
            .with_comment("if sum > 1, jump to clamp".to_string());
        self.instructions.push(jnz_clamp);

        // Sum <= 1, use sum as result
        let copy_sum = CasmInstruction::new(Opcode::StoreDerefFp.into())
            .with_off0(sum_off)
            .with_off2(dest_off)
            .with_comment("result = sum".to_string());
        self.instructions.push(copy_sum);

        // Jump to end
        let jmp_end = CasmInstruction::new(Opcode::JmpAbsImm.into())
            .with_operand(Operand::Label(end_label.clone()))
            .with_comment("jump to end".to_string());
        self.instructions.push(jmp_end);

        // clamp label: set result = 1
        self.add_label(Label::new(clamp_label));
        let clamp_result = CasmInstruction::new(Opcode::StoreImm.into())
            .with_off2(dest_off)
            .with_imm(1)
            .with_comment("result = 1 (clamped)".to_string());
        self.instructions.push(clamp_result);

        // end label
        self.add_label(Label::new(end_label));

        Ok(())
    }
}
