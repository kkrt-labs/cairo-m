//! # Main Code Generator
//!
//! This module orchestrates the entire MIR to CASM translation process.

use std::collections::HashMap;

use cairo_m_compiler_mir::{
    BasicBlockId, Instruction, InstructionKind, MirFunction, MirModule, Terminator, Value, ValueId,
};
use stwo_prover::core::fields::m31::M31;

use crate::compiled_program::{CompiledInstruction, CompiledProgram, ProgramMetadata};
use crate::{
    CasmBuilder, CasmInstruction, CodegenError, CodegenResult, FunctionLayout, Label, Opcode,
    Operand,
};

/// Main code generator that orchestrates MIR to CASM translation
#[derive(Debug)]
pub struct CodeGenerator {
    /// Generated instructions for all functions
    instructions: Vec<CasmInstruction>,
    /// Labels that need resolution
    labels: Vec<Label>,
    /// Function name to starting PC mapping
    function_addresses: HashMap<String, usize>,
    /// Function layouts for frame size calculations
    function_layouts: HashMap<String, FunctionLayout>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            function_addresses: HashMap::new(),
            function_layouts: HashMap::new(),
        }
    }

    /// Generate CASM code for an entire MIR module
    pub fn generate_module(&mut self, module: &MirModule) -> CodegenResult<()> {
        // Step 1: Calculate layouts for all functions
        self.calculate_all_layouts(module)?;

        // Step 2: Generate code for all functions (first pass)
        self.generate_all_functions(module)?;

        // Step 3: Resolve labels (second pass)
        self.resolve_labels()?;

        Ok(())
    }

    /// Compile the generated code into a CompiledProgram.
    pub fn compile(&self) -> CompiledProgram {
        // Convert CasmInstructions to CompiledInstructions
        let instructions = self
            .instructions
            .iter()
            .map(|instr| {
                // Convert the instruction to operands array
                let operands: Vec<M31> = vec![
                    instr.op0().unwrap_or(0).into(),
                    instr.op1().unwrap_or(0).into(),
                    instr.op2().unwrap_or(0).into(),
                ];

                CompiledInstruction {
                    opcode: instr.opcode,
                    operands,
                }
            })
            .collect();

        CompiledProgram {
            metadata: ProgramMetadata {
                compiler_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                compiled_at: Some(chrono::Utc::now().to_rfc3339()),
                source_file: None,
                extra: HashMap::new(),
            },
            entrypoints: self.function_addresses.clone(),
            instructions,
        }
    }

    /// Calculate layouts for all functions in the module
    fn calculate_all_layouts(&mut self, module: &MirModule) -> CodegenResult<()> {
        for function in module.functions.iter() {
            let layout = FunctionLayout::new(function)?;
            self.function_layouts.insert(function.name.clone(), layout);
        }
        Ok(())
    }

    /// Generate code for all functions
    fn generate_all_functions(&mut self, module: &MirModule) -> CodegenResult<()> {
        for function in module.functions.iter() {
            self.generate_function(function, module)?;
        }
        Ok(())
    }

    /// Generate code for a single function
    fn generate_function(
        &mut self,
        function: &MirFunction,
        module: &MirModule,
    ) -> CodegenResult<()> {
        // Get the layout for this function
        let layout = self
            .function_layouts
            .get(&function.name)
            .ok_or_else(|| {
                CodegenError::LayoutError(format!("No layout found for function {}", function.name))
            })?
            .clone();

        // Create a builder for this function
        let mut builder = CasmBuilder::new().with_layout(layout);

        // Add function label - but we'll fix the address later
        let func_label = Label::for_function(&function.name);
        self.function_addresses
            .insert(function.name.clone(), self.instructions.len());
        builder.add_label(func_label);

        // Generate code for all basic blocks
        self.generate_basic_blocks(function, module, &mut builder)?;

        // Fix label addresses to be relative to the global instruction stream
        let instruction_offset = self.instructions.len();
        let mut corrected_labels = builder.labels().to_vec();
        for label in &mut corrected_labels {
            if let Some(local_addr) = label.address {
                label.address = Some(local_addr + instruction_offset);
            }
        }
        // Append generated instructions and corrected labels
        self.instructions
            .extend(builder.instructions().iter().cloned());
        self.labels.extend(corrected_labels);

        Ok(())
    }

    /// Generate code for all basic blocks in a function
    fn generate_basic_blocks(
        &self,
        function: &MirFunction,
        module: &MirModule,
        builder: &mut CasmBuilder,
    ) -> CodegenResult<()> {
        // Process blocks in order
        let block_count = function.basic_blocks.len();
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Add block label
            let block_label = Label::for_block(&function.name, block_id);
            builder.add_label(block_label);

            for instruction in &block.instructions {
                self.generate_instruction(
                    instruction,
                    function,
                    module,
                    builder,
                    &block.terminator,
                )?;
            }

            // Determine the next block in sequence (if any)
            let next_block_id = if block_id.index() + 1 < block_count {
                Some(BasicBlockId::from_raw(block_id.index() + 1))
            } else {
                None
            };

            // Generate terminator with fall-through optimization
            self.generate_terminator(&block.terminator, &function.name, builder, next_block_id)?;
        }

        Ok(())
    }

    /// Generate code for a single instruction
    fn generate_instruction(
        &self,
        instruction: &Instruction,
        _function: &MirFunction,
        module: &MirModule,
        builder: &mut CasmBuilder,
        terminator: &Terminator,
    ) -> CodegenResult<()> {
        match &instruction.kind {
            InstructionKind::Assign { dest, source } => {
                // Check if this assignment result will be immediately returned
                let target_offset = self.get_target_offset_for_dest(*dest, terminator);
                builder.assign_with_target(*dest, *source, target_offset)?;
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
            } => {
                // Check if this binary operation result will be immediately returned
                let target_offset = self.get_target_offset_for_dest(*dest, terminator);
                builder.binary_op_with_target(*op, *dest, *left, *right, target_offset)?;
            }

            InstructionKind::Call { dest, callee, args } => {
                // Look up the callee's actual function name from the module
                let callee_function = module.functions.get(*callee).ok_or_else(|| {
                    CodegenError::MissingTarget(format!("No function found for callee {callee:?}"))
                })?;

                let callee_name = &callee_function.name;

                // For now, assume 1 return value for non-void calls
                // TODO: This should be determined from function signature / callee_function.return_value (when it has support for multiple?)
                let num_returns = 1;

                builder.call(*dest, callee_name, args, num_returns)?;
            }

            InstructionKind::VoidCall { callee, args } => {
                // Look up the callee's actual function name from the module
                let callee_function = module.functions.get(*callee).ok_or_else(|| {
                    CodegenError::MissingTarget(format!("No function found for callee {callee:?}"))
                })?;
                let callee_name = &callee_function.name;

                // Void calls have 0 return values
                let num_returns = 0;
                builder.void_call(callee_name, args, num_returns)?;
            }

            InstructionKind::Load { dest, address } => {
                builder.load(*dest, *address)?;
            }

            InstructionKind::Store { address, value } => {
                builder.store(*address, *value)?;
            }

            InstructionKind::StackAlloc { dest, size } => {
                // Allocate the requested stack space dynamically
                builder.allocate_stack(*dest, *size)?;
            }

            InstructionKind::AddressOf { .. } => {
                todo!("AddressOf is not implemented yet");
            }

            InstructionKind::GetElementPtr { dest, base, offset } => {
                builder.get_element_ptr(*dest, *base, *offset)?;
            }

            InstructionKind::Cast { .. } => {
                todo!("Cast is not implemented yet");
            }

            InstructionKind::Debug { .. } => {
                todo!("Debug is not implemented yet");
            }
        }

        Ok(())
    }

    /// Get the target offset for a destination ValueId if it will be immediately returned
    fn get_target_offset_for_dest(&self, dest: ValueId, terminator: &Terminator) -> Option<i32> {
        match terminator {
            Terminator::Return {
                value: Some(Value::Operand(return_dest)),
            } if *return_dest == dest => {
                // This destination will be immediately returned, use the return slot
                Some(-3) // Return slot is at [fp - 3]
            }
            _ => None, // Use normal allocation
        }
    }

    /// Generate code for a terminator with fall-through optimization
    fn generate_terminator(
        &self,
        terminator: &Terminator,
        function_name: &str,
        builder: &mut CasmBuilder,
        next_block_id: Option<BasicBlockId>,
    ) -> CodegenResult<()> {
        match terminator {
            Terminator::Jump { target } => {
                // Check if this jump is to the immediately following block
                if let Some(next_id) = next_block_id
                    && *target == next_id
                {
                    // Skip generating the jump - fall through to next block
                    return Ok(());
                }
                let target_label = format!("{function_name}_{target:?}");
                builder.jump(&target_label)?;
            }

            Terminator::If {
                condition,
                then_target,
                else_target,
            } => {
                // Check if we can optimize the control flow
                let then_is_next = next_block_id == Some(*then_target);

                let then_label = format!("{function_name}_{then_target:?}");
                let else_label = format!("{function_name}_{else_target:?}");

                // The `condition` from an `Eq` operation is `a - b`.
                // It is non-zero if `a != b`.
                // Our MIR `if` checks for equality, so it branches to `then` if `a - b` is zero.
                // CASM's `jnz` jumps if the value is non-zero.
                // Therefore, if the condition value is non-zero (i.e., `a != b`), we jump to the `else` block.
                // Always jnz to the else label
                builder.jnz(*condition, &else_label)?;

                if !then_is_next {
                    builder.jump(&then_label)?;
                }
            }

            Terminator::Return { value } => {
                builder.return_value(*value)?;
            }

            Terminator::Unreachable => {
                // Unreachable code - could add a debug trap or just ignore
            }
        }

        Ok(())
    }

    /// Resolve all label references (second pass)
    fn resolve_labels(&mut self) -> CodegenResult<()> {
        // Build a map of label names to their addresses
        let mut label_map = HashMap::new();

        // Add function addresses
        for (name, addr) in &self.function_addresses {
            label_map.insert(name.clone(), *addr);
        }

        // Add block labels
        for label in &self.labels {
            if let Some(addr) = label.address {
                label_map.insert(label.name.clone(), addr);
            }
        }

        // Resolve label references in instructions
        for (pc, instruction) in self.instructions.iter_mut().enumerate() {
            if let Some(Operand::Label(label_name)) = &instruction.operand {
                match Opcode::from_u32(instruction.opcode) {
                    Some(Opcode::JmpAbsImm) => {
                        // Absolute jump - use direct address
                        if let Some(&target_addr) = label_map.get(label_name) {
                            instruction.operand = Some(Operand::Literal(target_addr as i32));
                        } else {
                            return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                        }
                    }

                    Some(Opcode::JnzFpImm) => {
                        // Conditional jump - use relative offset
                        if let Some(&target_addr) = label_map.get(label_name) {
                            let relative_offset = (target_addr as i32) - (pc as i32);
                            instruction.operand = Some(Operand::Literal(relative_offset));
                        } else {
                            return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                        }
                    }

                    Some(Opcode::CallAbsImm) => {
                        // Function call - use direct address
                        if let Some(&target_addr) = label_map.get(label_name) {
                            instruction.operand = Some(Operand::Literal(target_addr as i32));
                        } else {
                            return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                        }
                    }

                    _ => {
                        // Other opcodes with label operands - this shouldn't happen with current implementation
                        let opcode_name = Opcode::from_u32(instruction.opcode)
                            .map(|op| format!("{op:?}"))
                            .unwrap_or_else(|| format!("Unknown({})", instruction.opcode));
                        return Err(CodegenError::UnresolvedLabel(format!(
                            "Unexpected label operand for opcode {opcode_name}: {label_name}"
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate a debug representation of instructions with resolved labels
    /// This is primarily used for snapshot testing
    pub fn debug_instructions(&self) -> String {
        let mut result = String::new();

        // Build a map from address (PC) to label names
        let mut pc_to_labels: std::collections::HashMap<usize, Vec<String>> =
            std::collections::HashMap::new();

        // Add function labels from function_addresses
        for (name, addr) in &self.function_addresses {
            pc_to_labels.entry(*addr).or_default().push(name.clone());
        }

        // Add block labels
        for label in &self.labels {
            if let Some(addr) = label.address {
                pc_to_labels
                    .entry(addr)
                    .or_default()
                    .push(label.name.clone());
            }
        }

        for (pc, instruction) in self.instructions.iter().enumerate() {
            if let Some(labels) = pc_to_labels.get(&pc) {
                // Deduplicate labels properly:
                // 1. If there's a function label and a corresponding block_0 label, only show the function label
                // 2. Otherwise, show all unique labels
                // note: for rendering only. Move to a Display impl for CasmInstruction?

                let mut labels_to_show = Vec::new();
                let mut function_labels = Vec::new();
                let mut block_labels = Vec::new();

                // Separate function labels from block labels
                for label in labels {
                    if label.contains('_') {
                        block_labels.push(label);
                    } else {
                        function_labels.push(label);
                    }
                }

                // Add function labels (should be at most one per address)
                if let Some(func_label) = function_labels.first() {
                    labels_to_show.push((*func_label).clone());

                    // Only add block_0 labels if they don't correspond to this function
                    for block_label in &block_labels {
                        let expected_block_0_label = format!("{func_label}_0");
                        if *block_label != &expected_block_0_label {
                            labels_to_show.push((*block_label).clone());
                        }
                    }
                } else {
                    // No function label, add all block labels
                    for block_label in &block_labels {
                        labels_to_show.push((*block_label).clone());
                    }
                }

                for label in labels_to_show {
                    result.push_str(&format!("{label}:\n"));
                }
            }
            result.push_str(&format!("{:4}: {}\n", pc, instruction.to_asm()));
        }

        result
    }

    /// Get the generated instructions (for testing)
    pub fn instructions(&self) -> &[CasmInstruction] {
        &self.instructions
    }

    /// Get function addresses (for testing)
    pub const fn function_addresses(&self) -> &HashMap<String, usize> {
        &self.function_addresses
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use cairo_m_compiler_mir::{BasicBlock, MirFunction, MirModule, Terminator, Value, ValueId};

    use super::*;

    fn create_simple_function() -> MirFunction {
        let mut function = MirFunction::new("main".to_string());
        let value_id = function.new_value_id();
        function.parameters.push(value_id);

        // Create a simple basic block that returns the parameter
        let mut block = BasicBlock::new();
        block.terminator = Terminator::Return {
            value: Some(Value::Operand(ValueId::from_raw(0))),
        };

        function.basic_blocks.push(block);
        function
    }

    #[test]
    fn test_simple_function_generation() {
        let function = create_simple_function();
        let mut module = MirModule::new();
        module.functions.push(function);

        let mut generator = CodeGenerator::new();
        generator.generate_module(&module).unwrap();

        // Should generate some instructions
        assert!(!generator.instructions.is_empty());

        // Should contain a main function
        assert!(generator.function_addresses.contains_key("main"));
    }

    #[test]
    fn test_immediate_compilation() {
        // Create a simple module with a store immediate instruction
        let mut module = MirModule::new();
        let mut function = MirFunction::new("test".to_string());

        // Create a simple block that stores an immediate and returns
        let mut block = BasicBlock::new();

        // Store immediate 42 to local variable
        let dest = function.new_value_id();
        block
            .instructions
            .push(Instruction::assign(dest, Value::integer(42)));

        // Return the value
        block.terminator = Terminator::Return {
            value: Some(Value::Operand(dest)),
        };

        function.basic_blocks.push(block);
        module.functions.push(function);

        // Compile the module
        let mut generator = CodeGenerator::new();
        generator.generate_module(&module).unwrap();
        let compiled = generator.compile();

        // Check the store immediate instruction (should be first)
        // With the direct return optimization, the immediate is stored directly
        // to the return slot at [fp - 3], which is offset -3
        let store_imm = &compiled.instructions[0];
        assert_eq!(store_imm.opcode, 6); // StoreImm opcode
        assert_eq!(
            store_imm.operands,
            vec![M31::from(42), M31::from(0), M31::from(-3)]
        );
    }
}
