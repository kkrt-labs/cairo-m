//! # Main Code Generator
//!
//! This module orchestrates the entire MIR to CASM translation process.

use std::collections::HashMap;

use cairo_m_common::instruction::Instruction as CasmInstr;
use cairo_m_common::program::{AbiSlot, AbiType, EntrypointInfo};
use cairo_m_common::{Program, ProgramMetadata};
use cairo_m_compiler_mir::{
    BasicBlockId, BinaryOp, DataLayout, Instruction, InstructionKind, Literal, MirFunction,
    MirModule, MirType, Terminator, Value, ValueId,
};
use stwo_prover::core::fields::m31::M31;

use crate::mir_passes::legalize::legalize_module_for_vm;
use crate::passes;
use crate::{CasmBuilder, CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label};

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
        // Clone MIR and run target-specific legalization so builder can assume invariants.
        let mut legalized = module.clone();
        legalize_module_for_vm(&mut legalized);

        // Step 1: Calculate layouts for all functions (post-legalization)
        self.calculate_all_layouts(&legalized)?;

        // Step 2: Generate code for all functions (first pass)
        self.generate_all_functions(&legalized)?;

        // Step 3: Calculate memory layout for variable-sized instructions
        self.calculate_memory_layout()?;

        // Step 4: Resolve labels (second pass)
        self.resolve_labels()?;

        Ok(())
    }

    /// Compile the generated code into a CompiledProgram.
    pub(crate) fn compile(self) -> CodegenResult<Program> {
        let instructions = self
            .instructions
            .iter()
            .map(|instr| instr.build())
            .collect::<CodegenResult<_>>()?;

        Ok(Program {
            // TODO: Link source file / crates once supported
            metadata: ProgramMetadata {
                compiler_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                compiled_at: Some(chrono::Utc::now().to_rfc3339()),
                source_file: None,
                extra: HashMap::new(),
            },
            entrypoints: self.function_entrypoints,
            instructions,
        })
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
            let size_in_qm31s = cairo_m_common::Instruction::size_in_qm31s_for_opcode(
                instr_builder.inner_instr().opcode_value(),
            )
            .ok_or_else(|| {
                CodegenError::InvalidMir(format!(
                    "Unknown opcode: {}",
                    instr_builder.inner_instr().opcode_value()
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

        // Create entrypoint info for this function with type information
        let params: Vec<AbiSlot> = function
            .parameters
            .iter()
            .enumerate()
            .map(|(i, &value_id)| {
                let ty = &function.value_types[&value_id];
                Ok(AbiSlot {
                    name: format!("arg{}", i), // TODO: Get proper names from semantic info
                    ty: abi_type_from_mir(ty)?,
                })
            })
            .collect::<CodegenResult<_>>()?;

        let returns: Vec<AbiSlot> = function
            .return_values
            .iter()
            .enumerate()
            .map(|(i, &value_id)| {
                Ok(AbiSlot {
                    name: format!("ret{}", i),
                    ty: abi_type_from_mir(&function.value_types[&value_id])?,
                })
            })
            .collect::<CodegenResult<_>>()?;

        let entrypoint_info = EntrypointInfo {
            pc: self.instructions.len(),
            params,
            returns,
        };
        self.function_entrypoints
            .insert(function.name.clone(), entrypoint_info);

        builder.emit_add_label(func_label);

        // Generate code for all basic blocks
        self.generate_basic_blocks(function, module, &mut builder)?;

        self.label_counter += builder.label_counter();

        // Run post-builder passes (deduplication, peephole opts, etc.)
        passes::run_all(&mut builder)?;

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
            builder.emit_add_label(block_label);

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
            self.generate_terminator(
                &block.terminator,
                &function.name,
                function,
                builder,
                next_block_id,
            )?;
        }

        Ok(())
    }

    /// Helper function to find direct argument placement offset
    ///
    /// Checks if the next instruction is a call and if the given destination
    /// is used as an argument, returning the offset where it should be placed.
    fn find_direct_argument_placement_offset(
        &self,
        dest: ValueId,
        block_instructions: &[Instruction],
        instruction_index: usize,
        builder: &CasmBuilder,
    ) -> Option<i32> {
        if let Some(next_instruction) = block_instructions.get(instruction_index + 1) {
            if let InstructionKind::Call {
                args, signature, ..
            } = &next_instruction.kind
            {
                if let Some(arg_index) = args
                    .iter()
                    .position(|arg| matches!(arg, Value::Operand(id) if *id == dest))
                {
                    let l = builder.current_frame_usage();
                    let mut arg_offset = l;
                    for (i, param_type) in signature.param_types.iter().enumerate() {
                        if i == arg_index {
                            break;
                        }
                        // Arrays are passed as pointers (1 slot)
                        arg_offset += DataLayout::memory_size_of(param_type) as i32;
                    }
                    return Some(arg_offset);
                }
            }
        }
        None
    }

    /// Generate code for a single instruction
    #[allow(clippy::too_many_arguments)]
    fn generate_instruction(
        &self,
        instruction: &Instruction,
        function: &MirFunction,
        module: &MirModule,
        builder: &mut CasmBuilder,
        block_instructions: &[Instruction],
        instruction_index: usize,
        terminator: &Terminator,
    ) -> CodegenResult<()> {
        match &instruction.kind {
            InstructionKind::Assign { dest, source, ty } => {
                // Direct Argument Placement Optimization
                let mut target_offset = self.find_direct_argument_placement_offset(
                    *dest,
                    block_instructions,
                    instruction_index,
                    builder,
                );

                // Fallback to return-value optimization
                if target_offset.is_none() {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator, function);
                }

                // Use the unified type-aware assign method
                builder.assign(*dest, *source, ty, target_offset)?;
            }

            InstructionKind::AssertEq { left, right } => {
                // Only felt/bool single-slot values
                match (left, right) {
                    (Value::Operand(_), Value::Operand(_)) => {
                        return Err(CodegenError::InvalidMir(
                            "AssertEq on operands is not allowed".into(),
                        ));
                    }
                    (Value::Operand(lid), Value::Literal(Literal::Integer(imm))) => {
                        let lo = builder.layout_mut().get_offset(*lid)?;
                        builder.assert_eq_fp_imm(
                            lo,
                            *imm as i32,
                            format!("assert [fp + {lo}] == {imm}"),
                        );
                    }
                    (Value::Literal(Literal::Integer(imm)), Value::Operand(rid)) => {
                        let ro = builder.layout_mut().get_offset(*rid)?;
                        builder.assert_eq_fp_imm(
                            ro,
                            *imm as i32,
                            format!("assert [fp + {ro}] == {imm}"),
                        );
                    }
                    (Value::Operand(lid), Value::Literal(Literal::Boolean(b))) => {
                        let lo = builder.layout_mut().get_offset(*lid)?;
                        let imm = if *b { 1 } else { 0 };
                        builder.assert_eq_fp_imm(lo, imm, format!("assert [fp + {lo}] == {}", imm));
                    }
                    (Value::Literal(Literal::Boolean(b)), Value::Operand(rid)) => {
                        let ro = builder.layout_mut().get_offset(*rid)?;
                        let imm = if *b { 1 } else { 0 };
                        builder.assert_eq_fp_imm(ro, imm, format!("assert [fp + {ro}] == {}", imm));
                    }
                    (Value::Literal(left), Value::Literal(right)) => {
                        if left != right {
                            return Err(CodegenError::InvalidMir(
                                "An assert expression was evaluated to false".into(),
                            ));
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }

            InstructionKind::UnaryOp { op, dest, source } => {
                // Direct Argument Placement Optimization
                let mut target_offset = self.find_direct_argument_placement_offset(
                    *dest,
                    block_instructions,
                    instruction_index,
                    builder,
                );

                // Fallback to return-value optimization
                if target_offset.is_none() {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator, function);
                }

                builder.unary_op(*op, *dest, *source, target_offset)?;
            }

            InstructionKind::BinaryOp {
                op,
                dest,
                left,
                right,
            } => {
                // Post-legalization invariant: only U32Eq/U32Less remain among u32 comparisons.
                debug_assert!(
                    !matches!(
                        op,
                        BinaryOp::U32Neq
                            | BinaryOp::U32Greater
                            | BinaryOp::U32LessEqual
                            | BinaryOp::U32GreaterEqual
                    ),
                    "Illegalized u32 comparison {:?} should not reach codegen (expected legalizer to rewrite)",
                    op
                );
                // Direct Argument Placement Optimization
                let mut target_offset = self.find_direct_argument_placement_offset(
                    *dest,
                    block_instructions,
                    instruction_index,
                    builder,
                );

                // Fallback to return-value optimization if no other target was found
                // TODO: Currently disabled for U32 operations due to multi-slot handling
                if target_offset.is_none()
                    && !matches!(
                        op,
                        BinaryOp::U32Add | BinaryOp::U32Sub | BinaryOp::U32Mul | BinaryOp::U32Div
                    )
                {
                    target_offset = self.get_target_offset_for_dest(*dest, terminator, function);
                }

                builder.binary_op(*op, *dest, *left, *right, target_offset)?;
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
                builder.lower_call(&callee_function.name, args, signature, dests)?;
            }
            InstructionKind::Cast {
                dest,
                source,
                source_type,
                target_type,
            } => {
                builder.generate_cast(*dest, *source, source_type, target_type)?;
            }

            InstructionKind::Debug { .. } => {
                todo!("Debug is not implemented yet");
            }

            InstructionKind::Phi { .. } => {
                // Phi nodes are a compile-time construct for SSA form
                // They should be eliminated before code generation through
                // register allocation or SSA destruction pass
                return Err(CodegenError::InvalidMir(
                    "Phi instructions should be eliminated before code generation".to_string(),
                ));
            }

            InstructionKind::Nop => {
                // No operation - skip code generation
                // Nops are used as placeholders during transformation passes
            }

            // Value-based aggregate operations
            InstructionKind::MakeTuple { dest, elements } => {
                builder.make_tuple(*dest, elements, function)?;
            }

            InstructionKind::ExtractTupleElement {
                dest,
                tuple,
                index,
                element_ty,
            } => {
                builder.extract_tuple_element(*dest, *tuple, *index, element_ty, function)?;
            }

            InstructionKind::MakeStruct {
                dest,
                fields,
                struct_ty,
            } => {
                builder.make_struct(*dest, fields, struct_ty)?;
            }

            InstructionKind::ExtractStructField {
                dest,
                struct_val,
                field_name,
                field_ty,
            } => {
                builder.extract_struct_field(*dest, *struct_val, field_name, field_ty, function)?;
            }

            InstructionKind::InsertField {
                dest,
                struct_val,
                field_name,
                new_value,
                struct_ty,
            } => {
                builder.insert_struct_field(
                    *dest,
                    *struct_val,
                    field_name,
                    *new_value,
                    struct_ty,
                )?;
            }

            InstructionKind::InsertTuple {
                dest,
                tuple_val,
                index,
                new_value,
                tuple_ty,
            } => {
                builder.insert_tuple_element(*dest, *tuple_val, *index, *new_value, tuple_ty)?;
            }

            // Array operations - initially treat like tuples/structs (value-based)
            // TODO: Implement proper array materialization and pointer-based passing
            InstructionKind::MakeFixedArray {
                dest,
                elements,
                element_ty,
            } => {
                // For now, treat arrays like tuples - store elements sequentially
                // This will need to be updated for proper pointer-based arrays
                builder.make_fixed_array(*dest, elements, element_ty)?;
            }

            InstructionKind::ArrayIndex {
                dest,
                array,
                index,
                element_ty,
            } => {
                // Use unified array operation for loading
                use crate::builder::ArrayOperation;
                builder.array_operation(
                    *array,
                    *index,
                    element_ty,
                    ArrayOperation::Load { dest: *dest },
                    function,
                )?;
            }

            InstructionKind::ArrayInsert {
                dest,
                array_val,
                index,
                new_value,
                array_ty,
            } => {
                // Use unified array operation for storing
                use crate::builder::ArrayOperation;
                // Extract element type from array type
                let element_ty = match array_ty {
                    MirType::FixedArray { element_type, .. } => element_type.as_ref(),
                    _ => {
                        return Err(CodegenError::InvalidMir(
                            "ArrayInsert requires array type".to_string(),
                        ))
                    }
                };
                builder.array_operation(
                    *array_val,
                    *index,
                    element_ty,
                    ArrayOperation::Store {
                        dest: *dest,
                        value: *new_value,
                    },
                    function,
                )?;
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
        function: &MirFunction,
    ) -> Vec<Option<i32>> {
        match terminator {
            Terminator::Return { values } => {
                // Calculate the total number of return slots (accounting for multi-slot types)
                let return_types: Vec<MirType> = function
                    .return_values
                    .iter()
                    .map(|&ret_id| {
                        function
                            .value_types
                            .get(&ret_id)
                            .cloned()
                            .expect("Missing return type")
                    })
                    .collect();

                let k_slots: i32 = return_types
                    .iter()
                    .map(|ty| DataLayout::memory_size_of(ty) as i32)
                    .sum();

                // Calculate cumulative slot offsets for each return value
                let mut slot_offsets = Vec::new();
                let mut cumulative = 0;
                for ty in &return_types {
                    slot_offsets.push(cumulative);
                    cumulative += DataLayout::memory_size_of(ty) as i32;
                }

                dests
                    .iter()
                    .map(|dest| {
                        // Check if this dest is one of the values being returned
                        values
                            .iter()
                            .position(|v| matches!(v, Value::Operand(id) if *id == *dest))
                            .map(|index| {
                                // Return value i goes to [fp - k_slots - 2 + slot_offset[i]]
                                -(k_slots + 2) + slot_offsets[index]
                            })
                    })
                    .collect()
            }
            _ => vec![None; dests.len()], // No optimization for non-return terminators
        }
    }

    /// Get the target offset for a single destination ValueId if it will be immediately returned
    fn get_target_offset_for_dest(
        &self,
        dest: ValueId,
        terminator: &Terminator,
        function: &MirFunction,
    ) -> Option<i32> {
        self.get_target_offsets_for_dests(&[dest], terminator, function)
            .into_iter()
            .next()
            .flatten()
    }

    /// Generate code for a terminator with fall-through optimization
    fn generate_terminator(
        &self,
        terminator: &Terminator,
        function_name: &str,
        function: &MirFunction,
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
                builder.jump(&target_label);
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
                builder.jump(&else_label);
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

                match op {
                    BinaryOp::Eq => {
                        // For felt comparison, compute `a - b`. Result is zero if equal.
                        builder.compute_into_offset(
                            BinaryOp::Sub,
                            temp_slot_offset,
                            *left,
                            *right,
                        )?;
                        // Jump to else if non-zero (not equal)
                        builder.jnz_offset(temp_slot_offset, &else_label);
                        // Fallthrough to the `then` block if the `jnz` was not taken.
                        let then_is_next = next_block_id == Some(*then_target);
                        if !then_is_next {
                            builder.jump(&then_label);
                        }
                    }
                    BinaryOp::Neq => {
                        // For felt comparison, compute `a - b`. Result is non-zero if not equal.
                        builder.compute_into_offset(
                            BinaryOp::Sub,
                            temp_slot_offset,
                            *left,
                            *right,
                        )?;
                        // Jump to then if non-zero (not equal)
                        builder.jnz_offset(temp_slot_offset, &then_label);
                        // Fallthrough to the `else` block if the `jnz` was not taken.
                        let else_is_next = next_block_id == Some(*else_target);
                        if !else_is_next {
                            builder.jump(&else_label);
                        }
                    }
                    BinaryOp::U32Eq => {
                        // For U32 comparison, use U32Eq which returns a felt (1 if equal, 0 if not)
                        builder.compute_into_offset(
                            BinaryOp::U32Eq,
                            temp_slot_offset,
                            *left,
                            *right,
                        )?;
                        // Jump to then if non-zero (equal)
                        builder.jnz_offset(temp_slot_offset, &then_label);
                        // Fallthrough to the `else` block if the `jnz` was not taken.
                        let else_is_next = next_block_id == Some(*else_target);
                        if !else_is_next {
                            builder.jump(&else_label);
                        }
                    }
                    BinaryOp::U32Neq => {
                        // Builder only accepts U32Eq/U32Less for u32 comparisons.
                        // Implement `a != b` as:
                        //   tmp = (a == b)
                        //   if tmp != 0 -> else (equal)
                        //   else -> then (not equal)
                        builder.compute_into_offset(
                            BinaryOp::U32Eq,
                            temp_slot_offset,
                            *left,
                            *right,
                        )?;

                        // If equal (tmp != 0), jump to else
                        builder.jnz_offset(temp_slot_offset, &else_label);

                        // Otherwise, flow to then. If then isn't next, emit a jump.
                        let then_is_next = next_block_id == Some(*then_target);
                        if !then_is_next {
                            builder.jump(&then_label);
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
                // Get the return types from the function
                let return_types: Vec<MirType> = function
                    .return_values
                    .iter()
                    .map(|&ret_id| {
                        function
                            .value_types
                            .get(&ret_id)
                            .cloned()
                            .expect("Missing return type")
                    })
                    .collect();

                builder.return_values(values, &return_types)?;
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

        // Resolve label references in instructions (typed API)
        for (logical_pc, instruction) in self.instructions.iter_mut().enumerate() {
            // Only process instructions that carry a label placeholder
            let Some(label_name) = instruction.label.clone() else {
                continue;
            };

            let physical_pc = self.memory_layout.get(logical_pc).copied().ok_or_else(|| {
                CodegenError::UnresolvedLabel(format!("Invalid PC {} for instruction", logical_pc))
            })?;

            match instruction.inner_instr_mut() {
                CasmInstr::JmpAbsImm { target } => {
                    let &target_addr = label_map
                        .get(&label_name)
                        .ok_or_else(|| CodegenError::UnresolvedLabel(label_name.clone()))?;
                    *target = M31::from(target_addr as i32);
                    instruction.label = None;
                }
                CasmInstr::JnzFpImm { offset, .. } => {
                    let &target_addr = label_map
                        .get(&label_name)
                        .ok_or_else(|| CodegenError::UnresolvedLabel(label_name.clone()))?;
                    let rel = (target_addr as i32) - (physical_pc as i32);
                    *offset = M31::from(rel);
                    instruction.label = None;
                }
                CasmInstr::CallAbsImm { target, .. } => {
                    let &target_addr = label_map
                        .get(&label_name)
                        .ok_or_else(|| CodegenError::UnresolvedLabel(label_name.clone()))?;
                    *target = M31::from(target_addr as i32);
                    instruction.label = None;
                }
                _ => {
                    return Err(CodegenError::UnresolvedLabel(format!(
                        "Unexpected label for opcode {}: {}",
                        instruction.inner_instr().opcode_value(),
                        label_name
                    )));
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
            result.push_str(&format!("{:4}: {}\n", pc, instruction));
        }

        result
    }

    /// Get the generated instructions (for testing)
    pub fn instructions(&self) -> &[InstructionBuilder] {
        &self.instructions
    }
}

#[cfg(test)]
mod tests_asserts {
    use super::*;
    use cairo_m_compiler_mir::{
        Instruction, InstructionKind, MirModule, MirType, Terminator, Value,
    };

    #[test]
    fn codegen_emits_assert_opcodes_for_mir_asserts() {
        // Build a simple MIR function with felt and u32 asserts
        let mut module = MirModule::new();
        let mut func = MirFunction::new("main".to_string());

        let a = func.new_typed_value_id(MirType::Felt);
        let b = func.new_typed_value_id(MirType::Felt);
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction::assign(a, Value::integer(5), MirType::Felt));
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction::assign(b, Value::integer(6), MirType::Felt));
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction {
                kind: InstructionKind::AssertEq {
                    left: Value::operand(a),
                    right: Value::Literal(Literal::Integer(1)),
                },
                source_span: None,
                source_expr_id: None,
                comment: None,
            });

        let u1 = func.new_typed_value_id(MirType::U32);
        let u2 = func.new_typed_value_id(MirType::U32);
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction::assign(u1, Value::integer(3), MirType::U32));
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction::assign(u2, Value::integer(4), MirType::U32));
        // Compute boolean equality then assert == 1
        let eq_bool = func.new_typed_value_id(MirType::Bool);
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction::binary_op(
                cairo_m_compiler_mir::BinaryOp::U32Eq,
                eq_bool,
                Value::operand(u1),
                Value::operand(u2),
            ));
        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .push_instruction(Instruction {
                kind: InstructionKind::AssertEq {
                    left: Value::operand(eq_bool),
                    right: Value::integer(1),
                },
                source_span: None,
                source_expr_id: None,
                comment: None,
            });

        func.basic_blocks
            .get_mut(func.entry_block)
            .unwrap()
            .set_terminator(Terminator::return_void());

        module.add_function(func);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();
        let program = gen.compile().unwrap();

        let mut assert_fp_fp = 0;
        let mut assert_fp_imm = 0;
        for instr in &program.instructions {
            match instr {
                CasmInstr::AssertEqFpFp { .. } => assert_fp_fp += 1,
                CasmInstr::AssertEqFpImm { .. } => assert_fp_imm += 1,
                _ => {}
            }
        }

        assert_eq!(assert_fp_fp, 0);
        assert!(
            assert_fp_imm == 2,
            "expected 2 AssertEqFpImm, got {}",
            assert_fp_imm
        );
    }
}

/// Convert MIR types to ABI types for program metadata
fn abi_type_from_mir(ty: &MirType) -> CodegenResult<AbiType> {
    let out = match ty {
        MirType::Felt => AbiType::Felt,
        MirType::Bool => AbiType::Bool,
        MirType::U32 => AbiType::U32,
        MirType::Tuple(types) => {
            let elems: Vec<AbiType> = types
                .iter()
                .map(abi_type_from_mir)
                .collect::<CodegenResult<_>>()?;
            AbiType::Tuple(elems)
        }
        MirType::Struct { name, fields } => AbiType::Struct {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(fname, fty)| Ok((fname.clone(), abi_type_from_mir(fty)?)))
                .collect::<CodegenResult<_>>()?,
        },
        MirType::FixedArray { element_type, size } => AbiType::FixedSizeArray {
            element: Box::new(abi_type_from_mir(element_type)?),
            size: *size as u32,
        },
        MirType::Function { .. } => {
            return Err(CodegenError::InvalidMir(
                "Functions are not supported in entrypoint signatures".into(),
            ))
        }
        MirType::Unit => AbiType::Unit,
        MirType::Error | MirType::Unknown => {
            return Err(CodegenError::InvalidMir(
                "Unsupported ABI type in entrypoint signature".into(),
            ))
        }
    };
    Ok(out)
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
            .push(Instruction::assign(dest, Value::integer(42), MirType::Felt));

        // Return the value
        block.terminator = Terminator::return_value(Value::Operand(dest));

        function.basic_blocks.push(block);
        function.return_values.push(dest);
        module.functions.push(function);

        // Compile the module
        let mut generator = CodeGenerator::new();
        generator.generate_module(&module).unwrap();
        let compiled = generator.compile().unwrap();

        // Check the store immediate instruction (should be first)
        // With the direct return optimization, the immediate is stored directly
        // to the return slot at [fp - 3], which is offset -3
        let store_imm = &compiled.instructions[0];

        // Check for StoreImm opcode
        assert_eq!(
            store_imm,
            &CasmInstr::StoreImm {
                imm: M31::from(42),
                dst_off: M31::from(-3)
            }
        );
    }
}
