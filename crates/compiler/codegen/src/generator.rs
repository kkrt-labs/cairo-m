//! # Main Code Generator
//!
//! This module orchestrates the entire MIR to CASM translation process.

use std::collections::HashMap;

use cairo_m_common::instruction::*;
use cairo_m_common::program::EntrypointInfo;
use cairo_m_common::{Program, ProgramMetadata};
use cairo_m_compiler_mir::{
    BasicBlockId, BinaryOp, Instruction, InstructionKind, Literal, MirFunction, MirModule,
    Terminator, Value, ValueId,
};

use crate::{
    CasmBuilder, CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label, Operand,
};

/// Main code generator that orchestrates MIR to CASM translation
#[derive(Debug)]
pub struct CodeGenerator {
    /// Generated instructions for all functions
    instructions: Vec<InstructionBuilder>,
    /// Labels that need resolution
    labels: Vec<Label>,
    /// Function name to entrypoint info mapping
    function_entrypoints: HashMap<String, EntrypointInfo>,
    /// Function layouts for frame size calculations
    function_layouts: HashMap<String, FunctionLayout>,
    /// Memory layout: maps logical instruction index to physical memory address
    memory_layout: Vec<u32>,

    label_counter: usize,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            function_entrypoints: HashMap::new(),
            function_layouts: HashMap::new(),
            memory_layout: Vec::new(),
            label_counter: 0,
        }
    }

    /// Generate CASM code for an entire MIR module
    pub fn generate_module(&mut self, module: &MirModule) -> CodegenResult<()> {
        // Step 1: Calculate layouts for all functions
        self.calculate_all_layouts(module)?;

        // Step 2: Generate code for all functions (first pass)
        self.generate_all_functions(module)?;

        // Step 3: Calculate memory layout for variable-sized instructions
        self.calculate_memory_layout()?;

        // Step 4: Resolve labels (second pass)
        self.resolve_labels()?;

        Ok(())
    }

    /// Compile the generated code into a CompiledProgram.
    pub fn compile(self) -> Program {
        let instructions = self
            .instructions
            .iter()
            .map(|instr| instr.build())
            .collect();

        Program {
            // TODO: Link source file / crates once supported
            metadata: ProgramMetadata {
                compiler_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                compiled_at: Some(chrono::Utc::now().to_rfc3339()),
                source_file: None,
                extra: HashMap::new(),
            },
            entrypoints: self.function_entrypoints,
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

    /// Calculate memory layout for variable-sized instructions
    fn calculate_memory_layout(&mut self) -> CodegenResult<()> {
        self.memory_layout.clear();
        let mut current_mem_pc = 0u32;

        for instr_builder in &self.instructions {
            self.memory_layout.push(current_mem_pc);

            // Get the size of this instruction based on its opcode
            let size_in_qm31s =
                cairo_m_common::Instruction::size_in_qm31s_for_opcode(instr_builder.opcode)
                    .ok_or_else(|| {
                        CodegenError::InvalidMir(format!(
                            "Unknown opcode: {}",
                            instr_builder.opcode
                        ))
                    })?;
            current_mem_pc += size_in_qm31s;
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
        let mut builder = CasmBuilder::new(layout, self.label_counter);

        // Add function label - but we'll fix the address later
        let func_label = Label::for_function(&function.name);

        // Create entrypoint info for this function
        // TODO: this is not using the name of the argument, rather a placeholder with arg_{index}.
        // Fix this with the proper argument names.
        // Note: We store the logical PC here, will convert to physical address later
        let entrypoint_info = EntrypointInfo {
            pc: self.instructions.len(),
            args: (0..function.parameters.len())
                .map(|i| format!("arg{}", i))
                .collect(),
            num_return_values: function.return_values.len(),
        };
        self.function_entrypoints
            .insert(function.name.clone(), entrypoint_info);

        builder.add_label(func_label);

        // Generate code for all basic blocks
        self.generate_basic_blocks(function, module, &mut builder)?;

        self.label_counter += builder.label_counter();

        // Remove duplicate offsets
        let _ = builder.resolve_duplicate_offsets();

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

            for (idx, instruction) in block.instructions.iter().enumerate() {
                self.generate_instruction(
                    instruction,
                    function,
                    module,
                    builder,
                    &block.instructions,
                    idx,
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
        block_instructions: &[Instruction],
        instruction_index: usize,
        terminator: &Terminator,
    ) -> CodegenResult<()> {
        match &instruction.kind {
            InstructionKind::Assign { dest, source } => {
                let mut target_offset = None;

                // Direct Argument Placement Optimization
                if let Some(next_instruction) = block_instructions.get(instruction_index + 1) {
                    if let InstructionKind::Call {
                        args, signature, ..
                    } = &next_instruction.kind
                    {
                        if let Some(arg_index) = args
                            .iter()
                            .position(|arg| matches!(arg, Value::Operand(id) if *id == *dest))
                        {
                            let l = builder.current_frame_usage();
                            let mut arg_offset = l;
                            for (i, param_type) in signature.param_types.iter().enumerate() {
                                if i == arg_index {
                                    break;
                                }
                                arg_offset += param_type.size_units() as i32;
                            }
                            target_offset = Some(arg_offset);
                        }
                    }
                }

                // Fallback to return-value optimization
                if target_offset.is_none() {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator);
                }

                builder.assign_with_target(*dest, *source, target_offset)?;
            }

            InstructionKind::UnaryOp {
                op,
                dest,
                source,
                in_place_target,
            } => {
                let mut target_offset = if let Some(target_addr_id) = in_place_target {
                    builder.layout_mut().get_offset(*target_addr_id).ok()
                } else {
                    None
                };

                // Direct Argument Placement Optimization
                if target_offset.is_none() {
                    if let Some(next_instruction) = block_instructions.get(instruction_index + 1) {
                        if let InstructionKind::Call {
                            args, signature, ..
                        } = &next_instruction.kind
                        {
                            if let Some(arg_index) = args
                                .iter()
                                .position(|arg| matches!(arg, Value::Operand(id) if *id == *dest))
                            {
                                let l = builder.current_frame_usage();
                                let mut arg_offset = l;
                                for (i, param_type) in signature.param_types.iter().enumerate() {
                                    if i == arg_index {
                                        break;
                                    }
                                    arg_offset += param_type.size_units() as i32;
                                }
                                target_offset = Some(arg_offset);
                            }
                        }
                    }
                }

                // Fallback to return-value optimization
                if target_offset.is_none() {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator);
                }

                builder.unary_op_with_target(*op, *dest, *source, target_offset)?;
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
                in_place_target,
            } => {
                // Check if this op can be performed in-place
                let mut target_offset = if let Some(target_addr_id) = in_place_target {
                    // The optimization applies. Get the offset for the target address.
                    builder.layout_mut().get_offset(*target_addr_id).ok()
                } else {
                    None
                };

                // NEW: Direct Argument Placement Optimization
                if target_offset.is_none() {
                    // Look ahead to see if `dest` is used as a call argument
                    if let Some(next_instruction) = block_instructions.get(instruction_index + 1) {
                        if let InstructionKind::Call {
                            args, signature, ..
                        } = &next_instruction.kind
                        {
                            // Check if dest is used as an argument
                            if let Some(arg_index) = args
                                .iter()
                                .position(|arg| matches!(arg, Value::Operand(id) if *id == *dest))
                            {
                                // Calculate where this argument needs to be placed
                                let l = builder.current_frame_usage();
                                let mut arg_offset = l;
                                for (i, param_type) in signature.param_types.iter().enumerate() {
                                    if i == arg_index {
                                        break;
                                    }
                                    arg_offset += param_type.size_units() as i32;
                                }
                                target_offset = Some(arg_offset);
                            }
                        }
                    }
                }

                // Fallback to return-value optimization if no other target was found
                if target_offset.is_none() {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator);
                }

                builder.binary_op_with_target(*op, *dest, *left, *right, target_offset)?;
            }

            InstructionKind::Call {
                dests,
                callee,
                args,
                signature,
            } => {
                // Look up the callee's actual function name from the module
                let callee_function = module.functions.get(*callee).ok_or_else(|| {
                    CodegenError::MissingTarget(format!("No function found for callee {callee:?}"))
                })?;

                let callee_name = &callee_function.name;
                let _num_returns = callee_function.return_values.len();

                if dests.len() == 1 {
                    // Single return value
                    builder.call(dests[0], callee_name, args, signature)?;
                } else {
                    // Multiple return values
                    builder.call_multiple(dests, callee_name, args, signature)?;
                }
            }

            InstructionKind::VoidCall {
                callee,
                args,
                signature,
            } => {
                // Look up the callee's actual function name from the module
                let callee_function = module.functions.get(*callee).ok_or_else(|| {
                    CodegenError::MissingTarget(format!("No function found for callee {callee:?}"))
                })?;
                let callee_name = &callee_function.name;

                builder.void_call(callee_name, args, signature)?;
            }

            InstructionKind::Load { dest, address } => {
                builder.load(*dest, *address)?;
            }

            InstructionKind::Store { address, value } => {
                // Check if this store's destination is used as a call argument
                if let Value::Operand(addr_id) = address {
                    if let Some(next_instruction) = block_instructions.get(instruction_index + 1) {
                        if let InstructionKind::Call {
                            args, signature, ..
                        } = &next_instruction.kind
                        {
                            // Check if the stored-to address is used as an argument
                            if let Some(arg_index) = args.iter().position(
                                |arg| matches!(arg, Value::Operand(id) if *id == *addr_id),
                            ) {
                                // Direct Argument Placement: store directly to argument position
                                let l = builder.current_frame_usage();
                                let mut arg_offset = l;
                                for (i, param_type) in signature.param_types.iter().enumerate() {
                                    if i == arg_index {
                                        break;
                                    }
                                    arg_offset += param_type.size_units() as i32;
                                }

                                // Store the value directly at the argument offset
                                match value {
                                    Value::Literal(Literal::Integer(imm)) => {
                                        builder.store_immediate_at(
                                            *imm,
                                            arg_offset,
                                            format!(
                                                "Direct arg placement: [fp + {}] = {}",
                                                arg_offset, imm
                                            ),
                                        )?;
                                        // Map the address ValueId to this offset
                                        builder.layout_mut().map_value(*addr_id, arg_offset);
                                    }
                                    Value::Operand(_src_id) => {
                                        // For operand sources, use regular store but at arg offset
                                        builder.store_at(*addr_id, arg_offset, *value)?;
                                    }
                                    _ => {
                                        // Fallback to regular store
                                        builder.store(*address, *value)?;
                                    }
                                }
                                return Ok(());
                            }
                        }
                    }
                }

                // Normal store
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

    /// Get the target offsets for destination ValueIds if they will be immediately returned
    /// Supports optimization for multiple return values
    fn get_target_offsets_for_dests(
        &self,
        dests: &[ValueId],
        terminator: &Terminator,
    ) -> Vec<Option<i32>> {
        match terminator {
            Terminator::Return { values } => {
                let k = values.len() as i32;
                dests
                    .iter()
                    .map(|dest| {
                        // Check if this dest is one of the values being returned
                        values
                            .iter()
                            .position(|v| matches!(v, Value::Operand(id) if *id == *dest))
                            .map(|index| {
                                // Return value i goes to [fp - K - 2 + i]
                                -(k + 2) + index as i32
                            })
                    })
                    .collect()
            }
            _ => vec![None; dests.len()], // No optimization for non-return terminators
        }
    }

    /// Get the target offset for a single destination ValueId if it will be immediately returned
    fn get_target_offset_for_dest(&self, dest: ValueId, terminator: &Terminator) -> Option<i32> {
        self.get_target_offsets_for_dests(&[dest], terminator)
            .into_iter()
            .next()
            .flatten()
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
                if let Some(next_id) = next_block_id {
                    if *target == next_id {
                        // Skip generating the jump - fall through to next block
                        return Ok(());
                    }
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
                let then_label = format!("{function_name}_{then_target:?}");
                let else_label = format!("{function_name}_{else_target:?}");

                // Because CASM has only JNZ, we need to jump to the then_label if the condition is true.
                // We can't optimize a fallthrough.
                builder.jnz(*condition, &then_label)?;
                builder.jump(&else_label)?;
            }

            Terminator::BranchCmp {
                op,
                left,
                right,
                then_target,
                else_target,
            } => {
                // This is the optimized path. We perform the comparison and branch directly
                // without materializing a boolean value into a named MIR variable.

                // Reserve a temporary slot on the stack for the comparison result.
                let temp_slot_offset = builder.layout_mut().reserve_stack(1);

                let then_label = format!("{function_name}_{then_target:?}");
                let else_label = format!("{function_name}_{else_target:?}");

                // For comparison, we compute `a - b`. The result is non-zero if they are not equal.
                builder.generate_arithmetic_op(BinaryOp::Sub, temp_slot_offset, *left, *right)?;

                match op {
                    BinaryOp::Eq => {
                        // `jnz` jumps if the result is non-zero.
                        // A non-zero result means `a != b`, so we should jump to the `else` block.
                        // Otherwise we can simply fallthrough.
                        builder.jnz_offset(temp_slot_offset, &else_label)?;

                        // Fallthrough to the `then` block if the `jnz` was not taken.
                        let then_is_next = next_block_id == Some(*then_target);
                        if !then_is_next {
                            builder.jump(&then_label)?;
                        }
                    }
                    BinaryOp::Neq => {
                        // `jnz` jumps if the result is non-zero.
                        // A non-zero result means `a != b`, so we should jump to the `then` block.
                        // Otherwise we can simply fallthrough.
                        builder.jnz_offset(temp_slot_offset, &then_label)?;

                        // Fallthrough to the `else` block if the `jnz` was not taken.
                        let else_is_next = next_block_id == Some(*else_target);
                        if !else_is_next {
                            builder.jump(&else_label)?;
                        }
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported comparison op in BranchCmp: {op:?}"
                        )));
                    }
                }
            }

            Terminator::Return { values } => {
                builder.return_values(values)?;
            }

            Terminator::Unreachable => {
                // Unreachable code - could add a debug trap or just ignore
            }
        }

        Ok(())
    }

    /// Resolve all label references (second pass)
    fn resolve_labels(&mut self) -> CodegenResult<()> {
        // Build a map of label names to their physical addresses
        let mut label_map = HashMap::new();

        // Update function entrypoints to use physical addresses and add to label map
        for (name, info) in &mut self.function_entrypoints {
            let logical_pc = info.pc;
            if let Some(&physical_pc) = self.memory_layout.get(logical_pc) {
                info.pc = physical_pc as usize;
                label_map.insert(name.clone(), physical_pc as usize);
            } else {
                return Err(CodegenError::UnresolvedLabel(format!(
                    "Function {} has invalid PC {}",
                    name, logical_pc
                )));
            }
        }

        // Add block labels with their physical addresses
        for label in &self.labels {
            if let Some(logical_addr) = label.address {
                if let Some(&physical_addr) = self.memory_layout.get(logical_addr) {
                    label_map.insert(label.name.clone(), physical_addr as usize);
                } else {
                    return Err(CodegenError::UnresolvedLabel(format!(
                        "Label {} has invalid address {}",
                        label.name, logical_addr
                    )));
                }
            }
        }

        // Resolve label references in instructions
        for (logical_pc, instruction) in self.instructions.iter_mut().enumerate() {
            // Check all operands for labels
            for operand in instruction.operands.iter_mut() {
                if let Operand::Label(label_name) = operand {
                    let physical_pc =
                        self.memory_layout.get(logical_pc).copied().ok_or_else(|| {
                            CodegenError::UnresolvedLabel(format!(
                                "Invalid PC {} for instruction",
                                logical_pc
                            ))
                        })?;

                    match instruction.opcode {
                        JMP_ABS_IMM => {
                            // Absolute jump - use direct physical address
                            if let Some(&target_addr) = label_map.get(label_name) {
                                *operand = Operand::Literal(target_addr as i32);
                            } else {
                                return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                            }
                        }

                        JNZ_FP_IMM => {
                            // Conditional jump - use relative offset in physical memory
                            if let Some(&target_addr) = label_map.get(label_name) {
                                let relative_offset = (target_addr as i32) - (physical_pc as i32);
                                *operand = Operand::Literal(relative_offset);
                            } else {
                                return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                            }
                        }

                        CALL_ABS_IMM => {
                            // Function call - use direct physical address
                            if let Some(&target_addr) = label_map.get(label_name) {
                                *operand = Operand::Literal(target_addr as i32);
                            } else {
                                return Err(CodegenError::UnresolvedLabel(label_name.clone()));
                            }
                        }

                        _ => {
                            // Other opcodes with label operands - this shouldn't happen with current implementation
                            return Err(CodegenError::UnresolvedLabel(format!(
                                "Unexpected label operand for opcode {}: {}",
                                instruction.opcode, label_name
                            )));
                        }
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

        // Add function labels from function_entrypoints
        for (name, info) in &self.function_entrypoints {
            pc_to_labels.entry(info.pc).or_default().push(name.clone());
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
                // note: TODO for rendering only. Move to a Display impl for InstructionBuilder?

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
    pub fn instructions(&self) -> &[InstructionBuilder] {
        &self.instructions
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use cairo_m_compiler_mir::{BasicBlock, MirFunction, MirModule, MirType, Terminator, Value};
    use stwo_prover::core::fields::m31::M31;

    use super::*;

    fn create_simple_function() -> MirFunction {
        let mut function = MirFunction::new("main".to_string());
        let value_id = function.new_typed_value_id(MirType::Felt);
        function.parameters.push(value_id);
        function.return_values.push(value_id);

        // Create a simple basic block that returns the parameter
        let mut block = BasicBlock::new();
        block.terminator = Terminator::return_value(Value::Operand(value_id));

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
        assert!(generator.function_entrypoints.contains_key("main"));
    }

    #[test]
    fn test_immediate_compilation() {
        // Create a simple module with a store immediate instruction
        let mut module = MirModule::new();
        let mut function = MirFunction::new("test".to_string());

        // Create a simple block that stores an immediate and returns
        let mut block = BasicBlock::new();

        // Store immediate 42 to local variable
        let dest = function.new_typed_value_id(MirType::Felt);
        block
            .instructions
            .push(Instruction::assign(dest, Value::integer(42)));

        // Return the value
        block.terminator = Terminator::return_value(Value::Operand(dest));

        function.basic_blocks.push(block);
        function.return_values.push(dest);
        module.functions.push(function);

        // Compile the module
        let mut generator = CodeGenerator::new();
        generator.generate_module(&module).unwrap();
        let compiled = generator.compile();

        // Check the store immediate instruction (should be first)
        // With the direct return optimization, the immediate is stored directly
        // to the return slot at [fp - 3], which is offset -3
        let store_imm = &compiled.instructions[0];

        // Check for StoreImm opcode
        assert_eq!(store_imm.opcode_value(), STORE_IMM);

        // Check operands - StoreImm has format: [imm, dst_off]
        let operands = store_imm.operands();
        assert_eq!(operands[0], M31::from(42)); // immediate value
        assert_eq!(operands[1], M31::from(-3)); // destination offset
    }
}
