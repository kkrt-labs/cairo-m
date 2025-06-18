//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.

use cairo_m_compiler_mir::{Literal, Value, ValueId};
use cairo_m_compiler_parser::parser::BinaryOp;

use crate::optimizations::{CopyPropagationPass, NeutralOperationsPass, OptimizationPipeline};
use crate::{CasmInstruction, CodegenError, CodegenResult, FunctionLayout, Label, Opcode, Operand};

/// Symbolic instructions can be either a label or a CASM instruction.
/// This allows for efficient label resolution later on
#[derive(Debug)]
pub enum SymbolicInstruction {
    Label(Label),
    Instruction(CasmInstruction),
}

/// Builder for generating CASM instructions
///
/// This struct manages the generation of CASM instructions, handling the
/// translation from MIR values to fp-relative memory addresses.
#[derive(Debug)]
pub struct CasmBuilder {
    /// Generated instructions
    symbolic_instructions: Vec<SymbolicInstruction>,
    /// Instructions with solved labels
    instructions: Vec<CasmInstruction>,
    /// Labels
    labels: Vec<Label>,
    /// Current function layout for offset lookups
    layout: Option<FunctionLayout>,
}

impl CasmBuilder {
    /// Create a new CASM builder
    pub const fn new() -> Self {
        Self {
            symbolic_instructions: Vec::new(),
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

    /// Run optimizations on the current function's instructions
    pub fn run_optimizations(&mut self) -> CodegenResult<()> {
        let mut pipeline = OptimizationPipeline::new();
        pipeline.add_pass(Box::new(NeutralOperationsPass::new()));
        pipeline.add_pass(Box::new(CopyPropagationPass::new()));
        pipeline.run(self)?;
        Ok(())
    }

    /// Add a label at the current position
    pub fn add_label(&mut self, label: Label) {
        self.symbolic_instructions
            .push(SymbolicInstruction::Label(label));
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
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        let dest_off = layout.allocate_local(dest, 1)?;

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
                // For equality, we need to compute (a - b) and check if it's zero
                // This is a simplification - real implementation might need more sophisticated handling
                self.generate_equality_check(dest_off, left, right)?;
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Binary operation {op:?} not implemented"
                )));
            }
        }

        Ok(())
    }

    /// Generate arithmetic operation (add, sub, mul, div)
    fn generate_arithmetic_op(
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

                self.symbolic_instructions
                    .push(SymbolicInstruction::Instruction(instr));
            }

            // Left is value, right is immediate: use fp_imm variant
            (Value::Operand(left_id), Value::Literal(Literal::Integer(imm))) => {
                let left_off = layout.get_offset(*left_id)?;

                let instr = CasmInstruction::new(fp_imm_opcode)
                    .with_off0(left_off)
                    .with_off2(dest_off)
                    .with_imm(*imm)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {left_off}] op {imm}"));

                self.symbolic_instructions
                    .push(SymbolicInstruction::Instruction(instr));
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
    ) -> CodegenResult<()> {
        // For now, implement equality as a subtraction and let the caller handle the comparison
        // A more sophisticated implementation would use conditional logic
        self.generate_arithmetic_op(
            Opcode::StoreSubFpFp.into(),
            Opcode::StoreSubFpImm.into(),
            dest_off,
            left,
            right,
        )?;

        // Add a comment to indicate this is an equality check
        if let Some(last_instr) = self.symbolic_instructions.last_mut() {
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

            if let SymbolicInstruction::Instruction(instr) = last_instr {
                instr.comment = Some(format!(
                    "Equality check: [fp + {dest_off}] = {left_str} - {right_str}"
                ));
            }
        }

        Ok(())
    }

    /// Generate assignment instruction
    ///
    /// Handles simple value assignments: dest = source
    pub fn assign(&mut self, dest: ValueId, source: Value) -> CodegenResult<()> {
        let layout = self
            .layout
            .as_mut()
            .ok_or_else(|| CodegenError::LayoutError("No layout set".to_string()))?;

        //TODO: use proper value size?
        let dest_off = layout.allocate_local(dest, 1)?;

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Store immediate value
                let instr = CasmInstruction::new(Opcode::StoreImm.into())
                    .with_off2(dest_off)
                    .with_imm(imm)
                    .with_comment(format!("[fp + {dest_off}] = {imm}"));

                self.symbolic_instructions
                    .push(SymbolicInstruction::Instruction(instr));
            }

            Value::Operand(src_id) => {
                // Copy from another value
                let src_off = layout.get_offset(src_id)?;

                let instr = CasmInstruction::new(Opcode::StoreDerefFp.into())
                    .with_off0(src_off)
                    .with_off2(dest_off)
                    .with_comment(format!("[fp + {dest_off}] = [fp + {src_off}]"));

                self.symbolic_instructions
                    .push(SymbolicInstruction::Instruction(instr));
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported assignment source".to_string(),
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
        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(instr));

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
        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(instr));
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
            self.symbolic_instructions
                .push(SymbolicInstruction::Instruction(instr));
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
                self.symbolic_instructions
                    .push(SymbolicInstruction::Instruction(instr));
            }
        }

        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(
                CasmInstruction::new(Opcode::Ret.into()).with_comment("return".to_string()),
            ));
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
        let instr = CasmInstruction::new(Opcode::JmpRelImm.into())
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("jump rel {target_label}"));

        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(instr));
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

        let instr = CasmInstruction::new(Opcode::JnzFpImm.into())
            .with_off0(cond_off)
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("if [fp + {cond_off}] != 0 jmp rel {target_label}"));

        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(instr));
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

    pub fn solve_labels(&mut self) -> CodegenResult<()> {
        use std::collections::HashMap;

        // First pass: collect label positions
        let mut label_positions: HashMap<String, usize> = HashMap::new();
        let mut instruction_index = 0;

        for symbolic_instr in &self.symbolic_instructions {
            match symbolic_instr {
                SymbolicInstruction::Label(label) => {
                    label_positions.insert(label.name.clone(), instruction_index);
                }
                SymbolicInstruction::Instruction(_) => {
                    instruction_index += 1;
                }
            }
        }

        // Second pass: resolve labels and generate final instructions
        self.instructions.clear();
        let mut current_index = 0;

        for symbolic_instr in &self.symbolic_instructions {
            match symbolic_instr {
                SymbolicInstruction::Label(label) => {
                    let mut new_label = Label::new(label.name.clone());
                    new_label.address = Some(current_index);
                    self.labels.push(new_label);
                }
                SymbolicInstruction::Instruction(instr) => {
                    let mut resolved_instr = instr.clone();

                    // Resolve label operands for local jumps only
                    if let Some(Operand::Label(label_name)) = &instr.operand {
                        // Only resolve local labels (jumps within the same function)
                        // Leave function calls unresolved as they will be handled by the linker
                        let should_resolve = match Opcode::from_u32(instr.opcode) {
                            Some(Opcode::JnzFpImm) | Some(Opcode::JmpRelImm) => {
                                // These are local control flow instructions
                                label_positions.contains_key(label_name)
                            }
                            Some(Opcode::CallAbsImm) => {
                                // Function calls - leave unresolved
                                false
                            }
                            _ => {
                                // Other instructions with labels - assume local for now
                                label_positions.contains_key(label_name)
                            }
                        };

                        if should_resolve {
                            let target_position =
                                label_positions.get(label_name).ok_or_else(|| {
                                    CodegenError::UnresolvedLabel(format!(
                                        "Label '{}' not found",
                                        label_name
                                    ))
                                })?;

                            // For relative jumps, calculate relative offset
                            let offset = match Opcode::from_u32(instr.opcode) {
                                Some(Opcode::JnzFpImm) | Some(Opcode::JmpRelImm) => {
                                    // Relative jump: target - current - 1 (PC advances after instruction)
                                    *target_position as i32 - current_index as i32 - 1
                                }
                                _ => {
                                    // For other local instructions, use absolute addressing
                                    *target_position as i32
                                }
                            };

                            resolved_instr.operand = Some(Operand::Literal(offset));
                        }
                        // If not resolving, keep the label as-is for later resolution by linker
                    }

                    self.instructions.push(resolved_instr);
                    current_index += 1;
                }
            }
        }

        Ok(())
    }

    /// Add a raw CASM instruction
    pub fn add_instruction(&mut self, instruction: CasmInstruction) {
        self.symbolic_instructions
            .push(SymbolicInstruction::Instruction(instruction));
    }

    /// Get the generated instructions
    pub fn instructions(&self) -> &[CasmInstruction] {
        &self.instructions
    }

    /// Get the symbolic instructions
    pub fn symbolic_instructions(&self) -> &[SymbolicInstruction] {
        &self.symbolic_instructions
    }

    /// Update the symbolic instructions
    pub fn set_symbolic_instructions(&mut self, instructions: Vec<SymbolicInstruction>) {
        self.symbolic_instructions = instructions;
    }

    /// Update the generated instructions
    pub fn set_instructions(&mut self, instructions: Vec<CasmInstruction>) {
        self.instructions = instructions;
    }

    /// Get the labels
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Take ownership of the labels
    pub fn into_labels(self) -> Vec<Label> {
        self.symbolic_instructions
            .iter()
            .filter_map(|instr| {
                if let SymbolicInstruction::Label(label) = instr {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect()
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

                        self.symbolic_instructions
                            .push(SymbolicInstruction::Instruction(instr));
                    }

                    Value::Operand(val_id) => {
                        let val_offset = layout.get_offset(val_id)?;

                        let instr = CasmInstruction::new(Opcode::StoreDerefFp.into())
                            .with_off0(val_offset)
                            .with_off2(dest_offset)
                            .with_comment(format!(
                                "Store: [fp + {dest_offset}] = [fp + {val_offset}]"
                            ));

                        self.symbolic_instructions
                            .push(SymbolicInstruction::Instruction(instr));
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
