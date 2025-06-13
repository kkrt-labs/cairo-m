//! # Main Code Generator
//!
//! This module orchestrates the entire MIR to CASM translation process.

use crate::{
    opcodes, CasmBuilder, CasmInstruction, CodegenError, CodegenResult, FunctionLayout, Label,
};
use cairo_m_compiler_mir::{Instruction, InstructionKind, MirFunction, MirModule, Terminator};
use std::collections::HashMap;
use stwo_prover::core::fields::m31::M31;

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
    pub fn generate_module(&mut self, module: &MirModule) -> CodegenResult<String> {
        // Step 1: Calculate layouts for all functions
        self.calculate_all_layouts(module)?;

        // Step 2: Generate code for all functions (first pass)
        self.generate_all_functions(module)?;

        // Step 3: Resolve labels (second pass)
        self.resolve_labels()?;

        // Step 4: Convert to final assembly string
        // TODO: this is only for snapshot testing / debugging purposes. in prod environment,
        // generate a sequence of instructions in JSON format.
        Ok(self.instructions_to_asm())
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
        for (block_id, block) in function.basic_blocks.iter_enumerated() {
            // Add block label
            let block_label = Label::for_block(&function.name, block_id);
            builder.add_label(block_label);

            for instruction in &block.instructions {
                self.generate_instruction(instruction, function, module, builder)?;
            }

            // Generate terminator
            self.generate_terminator(&block.terminator, &function.name, builder)?;
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
    ) -> CodegenResult<()> {
        match &instruction.kind {
            InstructionKind::Assign { dest, source } => {
                builder.assign(*dest, *source)?;
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
            } => {
                builder.binary_op(*op, *dest, *left, *right)?;
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

    /// Generate code for a terminator
    fn generate_terminator(
        &self,
        terminator: &Terminator,
        function_name: &str,
        builder: &mut CasmBuilder,
    ) -> CodegenResult<()> {
        match terminator {
            Terminator::Jump { target } => {
                let target_label = format!("{function_name}_{target:?}");
                builder.jump(&target_label)?;
            }

            Terminator::If {
                condition,
                then_target,
                else_target,
            } => {
                let then_label = format!("{function_name}_{then_target:?}");
                let else_label = format!("{function_name}_{else_target:?}");

                // The `condition` from an `Eq` operation is `a - b`.
                // It is non-zero if `a != b`.
                // Our MIR `if` checks for equality, so it branches to `then` if `a - b` is zero.
                // CASM's `jnz` jumps if the value is non-zero.
                // Therefore, if the condition value is non-zero (i.e., `a != b`), we jump to the `else` block.
                builder.jnz(*condition, &else_label)?;

                // If the condition was zero (`a == b`), we fall through to the `then` block's code.
                // We add an unconditional jump to the `then` block label. While this might seem
                // redundant if the `then` block immediately follows, it's safer and handles
                // arbitrary block ordering. The next block to be generated is not necessarily the `then` block.
                builder.jump(&then_label)?;
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

        // Resolve jump targets in instructions
        for (pc, instruction) in self.instructions.iter_mut().enumerate() {
            match instruction.opcode {
                opcodes::JMP_ABS_IMM => {
                    // Find the target label from the comment
                    // TODO: don't use comments to find the labels, as it's not a reliable format!
                    if let Some(comment) = &instruction.comment
                        && let Some(target) = comment.strip_prefix("jump abs ")
                        && let Some(&target_addr) = label_map.get(target)
                    {
                        instruction.imm = Some(M31::from(target_addr as u32));
                    }
                }

                opcodes::JNZ_FP_IMM => {
                    // Find the target label from the comment
                    if let Some(comment) = &instruction.comment {
                        // Look for the label after "jmp rel "
                        if let Some(idx) = comment.find("jmp rel ") {
                            let target = comment[(idx + 8)..].trim();
                            if let Some(&target_addr) = label_map.get(target) {
                                // Calculate relative offset
                                let relative_offset = (target_addr as i32) - (pc as i32);
                                instruction.imm = Some(M31::from(relative_offset));
                            }
                        }
                    }
                }

                opcodes::CALL_ABS_IMM => {
                    // Find the target function from the comment
                    if let Some(comment) = &instruction.comment
                        && let Some(target) = comment.strip_prefix("call ")
                    {
                        if let Some(&target_addr) = label_map.get(target) {
                            instruction.imm = Some(M31::from(target_addr as u32));
                        } else if target == "main" {
                            // Special case for main function
                            if let Some(&main_addr) = label_map.get("main") {
                                instruction.imm = Some(M31::from(main_addr as u32));
                            }
                        }
                    }
                }

                _ => {} // Other instructions don't need label resolution
            }
        }

        Ok(())
    }

    /// Convert instructions to final assembly string
    /// TODO: use a proper ASM representation.
    fn instructions_to_asm(&self) -> String {
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
    use super::*;
    use cairo_m_compiler_mir::{BasicBlock, MirFunction, MirModule, Terminator, Value, ValueId};

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
        let result = generator.generate_module(&module).unwrap();

        // Should generate some instructions
        assert!(!result.is_empty());

        // Should contain a main function
        assert!(generator.function_addresses.contains_key("main"));
    }
}
