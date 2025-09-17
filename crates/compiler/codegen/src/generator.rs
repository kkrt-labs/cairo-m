//! # Main Code Generator
//!
//! This module orchestrates the entire MIR to CASM translation process.

use std::collections::HashMap;

use cairo_m_common::instruction::Instruction as CasmInstr;
use cairo_m_common::program::{AbiSlot, AbiType, EntrypointInfo};
use cairo_m_common::{Program, ProgramData, ProgramMetadata};
use cairo_m_compiler_mir::{
    BasicBlockId, BinaryOp, DataLayout, Instruction, InstructionKind, Literal, MirFunction,
    MirModule, MirType, Projection, Terminator, Value, ValueId,
};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::mir_passes::legalize::legalize_module_for_vm;
use crate::passes;
use crate::{CasmBuilder, CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label};

// Mirror runner's memory model: MAX_ADDRESS = 2^28 - 1
const MAX_ADDRESS: i32 = (1 << 28) - 1;

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

    /// Read-only data blobs to append after code
    rodata_blobs: Vec<Vec<QM31>>,
    /// For deduplication: key -> blob index
    rodata_dedup: std::collections::HashMap<Vec<u32>, usize>,
    /// Map of rodata label -> blob index
    rodata_label_to_blob: std::collections::HashMap<String, usize>,
    /// Map of blob index -> rodata label (one label per blob)
    rodata_blob_to_label: std::collections::HashMap<usize, String>,
    /// Mutable data blobs appended after rodata (no dedup)
    data_blobs: Vec<Vec<QM31>>,
    /// Label -> mutable data blob index
    data_label_to_blob: std::collections::HashMap<String, usize>,
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
            rodata_blobs: Vec::new(),
            rodata_dedup: std::collections::HashMap::new(),
            rodata_label_to_blob: std::collections::HashMap::new(),
            rodata_blob_to_label: std::collections::HashMap::new(),
            data_blobs: Vec::new(),
            data_label_to_blob: std::collections::HashMap::new(),
        }
    }

    /// Ensure a single mutable data cell exists for the heap cursor.
    /// Returns the label name to use for addressing it.
    fn ensure_heap_cursor_label(&mut self) -> String {
        let label = "HEAP_CURSOR".to_string();
        if !self.data_label_to_blob.contains_key(&label) {
            // One QM31 cell initialized to zero
            let q = QM31::from_m31_array([M31::from(0), 0.into(), 0.into(), 0.into()]);
            let idx = self.data_blobs.len();
            self.data_blobs.push(vec![q]);
            self.data_label_to_blob.insert(label.clone(), idx);
        }
        label
    }

    /// Lower a HeapAllocCells MIR instruction into CASM using a bump allocator over a global cell.
    fn lower_heap_alloc_cells(
        &mut self,
        dest: ValueId,
        cells: &Value,
        builder: &mut CasmBuilder,
    ) -> CodegenResult<()> {
        // 1) Materialize address of HEAP_CURSOR in a temp via StoreImm with label
        let hp_label = self.ensure_heap_cursor_label();
        let hp_addr_off = builder.layout_mut().reserve_stack(1);
        let ib = InstructionBuilder::from(CasmInstr::StoreImm {
            imm: M31::from(0),
            dst_off: M31::from(hp_addr_off),
        })
        .with_comment(format!("[fp + {hp_addr_off}] = <{hp_label}>"))
        .with_label(hp_label);
        builder.emit_push(ib);

        // 2) Load current cursor: cur = [[fp + hp_addr_off] + 0]
        let cur_off = builder.layout_mut().reserve_stack(1);
        builder.store_from_double_deref_fp_imm(
            hp_addr_off,
            0,
            cur_off,
            format!("[fp + {cur_off}] = [[fp + {hp_addr_off}] + 0] (load heap cursor)"),
        );

        // 3) Materialize size (cells) into a slot
        let size_off = match cells {
            Value::Operand(id) => builder.layout_mut().get_offset(*id)?,
            Value::Literal(Literal::Integer(n)) => {
                let off = builder.layout_mut().reserve_stack(1);
                builder.store_immediate(*n, off, format!("[fp + {off}] = {n} (cells)"));
                off
            }
            Value::Literal(Literal::Boolean(b)) => {
                let off = builder.layout_mut().reserve_stack(1);
                let v = if *b { 1 } else { 0 };
                builder.store_immediate(v, off, format!("[fp + {off}] = {v} (cells)"));
                off
            }
            _ => {
                return Err(CodegenError::InvalidMir(
                    "HeapAllocCells: cells must be a felt operand or literal".into(),
                ))
            }
        };

        // 4) sum = cur + size - 1
        let sum_off = builder.layout_mut().reserve_stack(1);
        builder.felt_add_fp_fp(
            cur_off,
            size_off,
            sum_off,
            format!("[fp + {sum_off}] = [fp + {cur_off}] + [fp + {size_off}] (cur+size)"),
        );
        builder.felt_sub_fp_imm(
            sum_off,
            1,
            sum_off,
            format!("[fp + {sum_off}] = [fp + {sum_off}] - 1"),
        );

        // 5) base = MAX_ADDRESS - sum
        let max_off = builder.layout_mut().reserve_stack(1);
        builder.store_immediate(
            MAX_ADDRESS as u32,
            max_off,
            format!("[fp + {max_off}] = MAX_ADDRESS ({MAX_ADDRESS})"),
        );
        let base_off = builder.layout_mut().reserve_stack(1);
        builder.felt_sub_fp_fp(
            max_off,
            sum_off,
            base_off,
            format!("[fp + {base_off}] = [fp + {max_off}] - [fp + {sum_off}] (base)"),
        );

        // 6) new_cur = cur + size ; store to HEAP_CURSOR
        let new_cur_off = builder.layout_mut().reserve_stack(1);
        builder.felt_add_fp_fp(
            cur_off,
            size_off,
            new_cur_off,
            format!("[fp + {new_cur_off}] = [fp + {cur_off}] + [fp + {size_off}] (advance cursor)"),
        );
        builder.store_to_double_deref_fp_imm(
            new_cur_off,
            hp_addr_off,
            0,
            format!("[[fp + {hp_addr_off}] + 0] = [fp + {new_cur_off}] (store heap cursor)"),
        );

        // 7) Write result pointer to destination
        let dest_off = builder.layout_mut().allocate_local(dest, 1)?;
        builder.store_copy_single(
            base_off,
            dest_off,
            format!("[fp + {dest_off}] = [fp + {base_off}] (heap ptr)"),
        );

        Ok(())
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
        let instructions: Vec<cairo_m_common::Instruction> = self
            .instructions
            .iter()
            .map(|instr| instr.build())
            .collect::<CodegenResult<_>>()?;

        // Build linear program data: instructions then rodata
        let mut data: Vec<ProgramData> = instructions
            .into_iter()
            .map(ProgramData::Instruction)
            .collect();
        for blob in &self.rodata_blobs {
            for &q in blob.iter() {
                data.push(ProgramData::Value(q));
            }
        }
        // Append mutable data blobs after rodata
        for blob in &self.data_blobs {
            for &q in blob.iter() {
                data.push(ProgramData::Value(q));
            }
        }

        Ok(Program {
            // TODO: Link source file / crates once supported
            metadata: ProgramMetadata {
                compiler_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                compiled_at: Some(chrono::Utc::now().to_rfc3339()),
                source_file: None,
                extra: HashMap::new(),
            },
            entrypoints: self.function_entrypoints,
            data,
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

    /// Linearize a literal array into a rodata blob of QM31 values
    fn linearize_rodata_blob(elements: &[Value], element_ty: &MirType) -> CodegenResult<Vec<QM31>> {
        use cairo_m_compiler_mir::value::Literal as Lit;
        let mut out = Vec::new();
        match element_ty {
            MirType::Felt | MirType::Bool => {
                for v in elements {
                    let m = match v {
                        Value::Literal(Lit::Integer(n)) => *n,
                        Value::Literal(Lit::Boolean(b)) => {
                            if *b {
                                1
                            } else {
                                0
                            }
                        }
                        _ => {
                            return Err(CodegenError::InvalidMir(
                                "Non-literal element in rodata array".to_string(),
                            ))
                        }
                    };
                    out.push(QM31::from_m31_array([
                        M31::from(m),
                        0.into(),
                        0.into(),
                        0.into(),
                    ]));
                }
            }
            MirType::U32 => {
                for v in elements {
                    let Lit::Integer(n) = v.as_literal().ok_or_else(|| {
                        CodegenError::InvalidMir("Non-literal u32 element".into())
                    })?
                    else {
                        return Err(CodegenError::InvalidMir(
                            "Non-integer element for u32 array".to_string(),
                        ));
                    };
                    let lo = n & 0xFFFF;
                    let hi = (n >> 16) & 0xFFFF;
                    out.push(QM31::from_m31_array([
                        M31::from(lo),
                        0.into(),
                        0.into(),
                        0.into(),
                    ]));
                    out.push(QM31::from_m31_array([
                        M31::from(hi),
                        0.into(),
                        0.into(),
                        0.into(),
                    ]));
                }
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Const arrays only supported for felt, bool, u32".to_string(),
                ))
            }
        }
        Ok(out)
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
        &mut self,
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
                match &instruction.kind {
                    InstructionKind::MakeFixedArray {
                        dest,
                        elements,
                        element_ty,
                        is_const,
                    } => {
                        let all_literals = elements.iter().all(|v| matches!(v, Value::Literal(_)));
                        let is_scalar_elem =
                            matches!(element_ty, MirType::Felt | MirType::Bool | MirType::U32);
                        if *is_const && all_literals && is_scalar_elem {
                            // Register (or dedup) rodata blob
                            let blob = Self::linearize_rodata_blob(elements, element_ty)?;
                            // Build dedup key as flattened u32 limbs
                            let mut key: Vec<u32> = Vec::with_capacity(blob.len() * 4);
                            for q in &blob {
                                let arr = q.to_m31_array();
                                key.push(arr[0].0);
                                key.push(arr[1].0);
                                key.push(arr[2].0);
                                key.push(arr[3].0);
                            }
                            let blob_index = if let Some(&idx) = self.rodata_dedup.get(&key) {
                                idx
                            } else {
                                let idx = self.rodata_blobs.len();
                                self.rodata_blobs.push(blob);
                                self.rodata_dedup.insert(key, idx);
                                idx
                            };
                            // Reserve dest slot for array pointer and emit placeholder StoreImm 0 with label
                            let dest_off = builder.layout_mut().allocate_local(*dest, 1)?;
                            // Reuse a single label per unique blob
                            let ro_label =
                                if let Some(lbl) = self.rodata_blob_to_label.get(&blob_index) {
                                    lbl.clone()
                                } else {
                                    let lbl = format!("RODATA_{}", self.label_counter);
                                    self.label_counter += 1;
                                    self.rodata_blob_to_label.insert(blob_index, lbl.clone());
                                    self.rodata_label_to_blob.insert(lbl.clone(), blob_index);
                                    lbl
                                };
                            let ib = InstructionBuilder::from(CasmInstr::StoreImm {
                                imm: M31::from(0),
                                dst_off: M31::from(dest_off),
                            })
                            .with_comment(format!("[fp + {dest_off}] = <{}>", ro_label))
                            .with_label(ro_label.clone());
                            builder.emit_push(ib);
                        } else {
                            // Fallback to stack materialization
                            builder.make_fixed_array(*dest, elements, element_ty)?;
                        }
                    }
                    InstructionKind::HeapAllocCells { dest, cells } => {
                        self.lower_heap_alloc_cells(*dest, cells, builder)?;
                    }
                    _ => {
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
                }
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
                    (Value::Literal(left_literal), Value::Literal(right_literal)) => {
                        let right_imm = right_literal
                            .as_integer()
                            .expect("Right literal is not an integer");
                        let left_imm = left_literal
                            .as_integer()
                            .expect("Left literal is not an integer");
                        let is_eq = left_imm == right_imm;
                        let store_eq_dst = builder.layout_mut().reserve_stack(1);
                        builder.store_immediate(
                            is_eq as u32,
                            store_eq_dst,
                            format!("[fp + {store_eq_dst}] == {is_eq}"),
                        );
                        builder.assert_eq_fp_imm(
                            store_eq_dst,
                            1,
                            format!("assert [fp + {store_eq_dst}] == {is_eq}"),
                        );
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
                        BinaryOp::U32Add
                            | BinaryOp::U32Sub
                            | BinaryOp::U32Mul
                            | BinaryOp::U32Div
                            | BinaryOp::U32Rem
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

            // Array creation handled at the basic-block level for rodata lowering
            InstructionKind::MakeFixedArray {
                dest,
                elements,
                element_ty,
                ..
            } => {
                builder.make_fixed_array(*dest, elements, element_ty)?;
            }

            // New memory ops over places
            InstructionKind::Load { dest, place, ty } => {
                // Generalized place projection handling: Index + Field + Tuple
                let base_offset = builder.layout.get_offset(place.base)?;

                let mut curr_ty =
                    function
                        .value_types
                        .get(&place.base)
                        .cloned()
                        .ok_or_else(|| {
                            CodegenError::InvalidMir("Missing type for place base".into())
                        })?;

                let mut imm_off: i32 = 0;
                let mut dyn_off: Option<i32> = None;

                for proj in &place.projections {
                    match proj {
                        Projection::Index(idx) => {
                            let elem_ty = match &curr_ty {
                                MirType::FixedArray { element_type, .. } => *element_type.clone(),
                                MirType::Pointer { element } => *element.clone(),
                                other => {
                                    return Err(CodegenError::InvalidMir(format!(
                                        "Index projection on non-array/pointer type: {:?}",
                                        other
                                    )));
                                }
                            };
                            let stride = DataLayout::memory_size_of(&elem_ty) as i32;
                            match idx {
                                Value::Literal(Literal::Integer(i)) => {
                                    imm_off += (*i as i32) * stride;
                                }
                                Value::Operand(idx_id) => {
                                    let idx_layout = builder
                                        .layout
                                        .value_layouts
                                        .get(idx_id)
                                        .cloned()
                                        .ok_or_else(|| {
                                            CodegenError::InvalidMir(
                                                "Missing layout for index".to_string(),
                                            )
                                        })?;
                                    let idx_off = match idx_layout {
                                        crate::layout::ValueLayout::Slot { offset } => offset,
                                        _ => {
                                            return Err(CodegenError::InternalError(
                                                "Invalid index value layout".to_string(),
                                            ))
                                        }
                                    };
                                    if let Some(idx_ty) = function.value_types.get(idx_id) {
                                        if !matches!(idx_ty, MirType::Felt) {
                                            return Err(CodegenError::InvalidMir(format!(
                                                "Array index must be a felt; got {:?}",
                                                idx_ty
                                            )));
                                        }
                                    }
                                    let scaled = if stride != 1 {
                                        let tmp = builder.layout.reserve_stack(1);
                                        builder.felt_mul_fp_imm(
                                            idx_off,
                                            stride,
                                            tmp,
                                            format!(
                                                "[fp + {}] = [fp + {}] * {} (scale index)",
                                                tmp, idx_off, stride
                                            ),
                                        );
                                        tmp
                                    } else {
                                        idx_off
                                    };
                                    dyn_off = match dyn_off {
                                        None => Some(scaled),
                                        Some(ex) => {
                                            let sum = builder.layout.reserve_stack(1);
                                            builder.felt_add_fp_fp(
                                                ex,
                                                scaled,
                                                sum,
                                                format!(
                                                    "[fp + {}] = [fp + {}] + [fp + {}]",
                                                    sum, ex, scaled
                                                ),
                                            );
                                            Some(sum)
                                        }
                                    };
                                }
                                _ => {
                                    return Err(CodegenError::InvalidMir(
                                        "Invalid index value for load".to_string(),
                                    ))
                                }
                            }
                            curr_ty = elem_ty;
                        }
                        Projection::Field(name) => {
                            let off = DataLayout::field_offset(&curr_ty, name).ok_or_else(|| {
                                CodegenError::InvalidMir(format!(
                                    "Unknown field {} on type {:?}",
                                    name, curr_ty
                                ))
                            })? as i32;
                            imm_off += off;
                            curr_ty = curr_ty
                                .field_type(name)
                                .cloned()
                                .ok_or_else(|| CodegenError::InvalidMir("Bad field type".into()))?;
                        }
                        Projection::Tuple(index) => {
                            let off =
                                DataLayout::tuple_offset(&curr_ty, *index).ok_or_else(|| {
                                    CodegenError::InvalidMir(format!(
                                        "Tuple index {} out of bounds for {:?}",
                                        index, curr_ty
                                    ))
                                })? as i32;
                            imm_off += off;
                            curr_ty =
                                curr_ty.tuple_element_type(*index).cloned().ok_or_else(|| {
                                    CodegenError::InvalidMir("Bad tuple element type".into())
                                })?;
                        }
                    }
                }

                let elem_slots = DataLayout::memory_size_of(ty);
                let dest_off = builder.layout.allocate_local(*dest, elem_slots)?;
                match dyn_off {
                    None => {
                        for s in 0..elem_slots {
                            builder.store_from_double_deref_fp_imm(
                                base_offset,
                                imm_off + s as i32,
                                dest_off + s as i32,
                                format!(
                                    "[fp + {}] = [[fp + {}] + {}] (load slot {})",
                                    dest_off + s as i32,
                                    base_offset,
                                    imm_off + s as i32,
                                    s
                                ),
                            );
                        }
                    }
                    Some(off) => {
                        for s in 0..elem_slots {
                            let slot_off = if imm_off != 0 || s != 0 {
                                let tmp = builder.layout.reserve_stack(1);
                                builder.felt_add_fp_imm(
                                    off,
                                    imm_off + s as i32,
                                    tmp,
                                    format!(
                                        "[fp + {}] = [fp + {}] + {} (load slot {})",
                                        tmp,
                                        off,
                                        imm_off + s as i32,
                                        s
                                    ),
                                );
                                tmp
                            } else {
                                off
                            };
                            builder.store_from_double_deref_fp_fp(
                                base_offset,
                                slot_off,
                                dest_off + s as i32,
                                format!(
                                    "[fp + {}] = [[fp + {}] + [fp + {}]] (load slot {})",
                                    dest_off + s as i32,
                                    base_offset,
                                    slot_off,
                                    s
                                ),
                            );
                        }
                    }
                }
            }

            InstructionKind::Store { place, value, ty } => {
                let base_offset = builder.layout.get_offset(place.base)?;
                // Generalized projection chain
                let mut curr_ty =
                    function
                        .value_types
                        .get(&place.base)
                        .cloned()
                        .ok_or_else(|| {
                            CodegenError::InvalidMir("Missing type for place base".into())
                        })?;

                let mut imm_off: i32 = 0;
                let mut dyn_off: Option<i32> = None;

                for proj in &place.projections {
                    match proj {
                        Projection::Index(idx) => {
                            let elem_ty = match &curr_ty {
                                MirType::FixedArray { element_type, .. } => *element_type.clone(),
                                MirType::Pointer { element } => *element.clone(),
                                other => {
                                    return Err(CodegenError::InvalidMir(format!(
                                        "Index projection on non-array/pointer type: {:?}",
                                        other
                                    )));
                                }
                            };
                            let stride = DataLayout::memory_size_of(&elem_ty) as i32;
                            match idx {
                                Value::Literal(Literal::Integer(i)) => {
                                    imm_off += (*i as i32) * stride;
                                }
                                Value::Operand(idx_id) => {
                                    let idx_layout = builder
                                        .layout
                                        .value_layouts
                                        .get(idx_id)
                                        .cloned()
                                        .ok_or_else(|| {
                                            CodegenError::InvalidMir(
                                                "Missing layout for index".to_string(),
                                            )
                                        })?;
                                    let idx_off = match idx_layout {
                                        crate::layout::ValueLayout::Slot { offset } => offset,
                                        _ => {
                                            return Err(CodegenError::InternalError(
                                                "Invalid index value layout".to_string(),
                                            ))
                                        }
                                    };
                                    if let Some(idx_ty) = function.value_types.get(idx_id) {
                                        if !matches!(idx_ty, MirType::Felt) {
                                            return Err(CodegenError::InvalidMir(format!(
                                                "Array index must be a felt; got {:?}",
                                                idx_ty
                                            )));
                                        }
                                    }
                                    let scaled = if stride != 1 {
                                        let tmp = builder.layout.reserve_stack(1);
                                        builder.felt_mul_fp_imm(
                                            idx_off,
                                            stride,
                                            tmp,
                                            format!(
                                                "[fp + {}] = [fp + {}] * {} (scale index)",
                                                tmp, idx_off, stride
                                            ),
                                        );
                                        tmp
                                    } else {
                                        idx_off
                                    };
                                    dyn_off = match dyn_off {
                                        None => Some(scaled),
                                        Some(ex) => {
                                            let sum = builder.layout.reserve_stack(1);
                                            builder.felt_add_fp_fp(
                                                ex,
                                                scaled,
                                                sum,
                                                "combine offsets".into(),
                                            );
                                            Some(sum)
                                        }
                                    };
                                }
                                _ => {
                                    return Err(CodegenError::InvalidMir(
                                        "Invalid index value for store".to_string(),
                                    ))
                                }
                            }
                            curr_ty = elem_ty;
                        }
                        Projection::Field(name) => {
                            let off = DataLayout::field_offset(&curr_ty, name).ok_or_else(|| {
                                CodegenError::InvalidMir(format!(
                                    "Unknown field {} on type {:?}",
                                    name, curr_ty
                                ))
                            })? as i32;
                            imm_off += off;
                            curr_ty = curr_ty
                                .field_type(name)
                                .cloned()
                                .ok_or_else(|| CodegenError::InvalidMir("Bad field type".into()))?;
                        }
                        Projection::Tuple(index) => {
                            let off =
                                DataLayout::tuple_offset(&curr_ty, *index).ok_or_else(|| {
                                    CodegenError::InvalidMir(format!(
                                        "Tuple index {} out of bounds for {:?}",
                                        index, curr_ty
                                    ))
                                })? as i32;
                            imm_off += off;
                            curr_ty =
                                curr_ty.tuple_element_type(*index).cloned().ok_or_else(|| {
                                    CodegenError::InvalidMir("Bad tuple element type".into())
                                })?;
                        }
                    }
                }

                let elem_size = DataLayout::memory_size_of(ty);
                match value {
                    Value::Literal(Literal::Integer(v)) => {
                        if elem_size == 1 {
                            let tmp = builder.layout.reserve_stack(1);
                            builder.store_immediate(*v, tmp, format!("[fp + {}] = {}", tmp, *v));
                            if let Some(off) = dyn_off {
                                builder.store_to_double_deref_fp_fp(
                                    base_offset,
                                    off,
                                    tmp,
                                    format!(
                                        "[[fp + {}] + [fp + {}]] = [fp + {}]",
                                        base_offset, off, tmp
                                    ),
                                );
                            } else {
                                builder.store_to_double_deref_fp_imm(
                                    tmp,
                                    base_offset,
                                    imm_off,
                                    format!(
                                        "[[fp + {}] + {}] = [fp + {}]",
                                        base_offset, imm_off, tmp
                                    ),
                                );
                            }
                        } else if matches!(ty, MirType::U32) && elem_size == 2 {
                            let tmp = builder.layout.reserve_stack(2);
                            builder.store_u32_immediate(
                                *v,
                                tmp,
                                format!("[fp + {}], [fp + {}] = u32({v})", tmp, tmp + 1),
                            );
                            for s in 0..2 {
                                if let Some(off) = dyn_off {
                                    let slot_off = if s == 0 && imm_off == 0 {
                                        off
                                    } else {
                                        let t = builder.layout.reserve_stack(1);
                                        builder.felt_add_fp_imm(
                                            off,
                                            imm_off + s,
                                            t,
                                            format!(
                                                "[fp + {}] = [fp + {}] + {} (u32 slot {})",
                                                t,
                                                off,
                                                imm_off + s,
                                                s
                                            ),
                                        );
                                        t
                                    };
                                    builder.store_to_double_deref_fp_fp(
                                        base_offset,
                                        slot_off,
                                        tmp + s,
                                        format!(
                                            "[[fp + {}] + [fp + {}]] = [fp + {}] (u32 slot {})",
                                            base_offset,
                                            slot_off,
                                            tmp + s,
                                            s
                                        ),
                                    );
                                } else {
                                    builder.store_to_double_deref_fp_imm(
                                        tmp + s,
                                        base_offset,
                                        imm_off + s,
                                        format!(
                                            "[[fp + {}] + {}] = [fp + {}] (u32 slot {})",
                                            base_offset,
                                            imm_off + s,
                                            tmp + s,
                                            s
                                        ),
                                    );
                                }
                            }
                        } else {
                            return Err(CodegenError::UnsupportedInstruction(
                                "Storing immediate into multi-slot element is unsupported"
                                    .to_string(),
                            ));
                        }
                    }
                    Value::Literal(Literal::Boolean(b)) => {
                        if elem_size != 1 {
                            return Err(CodegenError::InvalidMir(
                                "Boolean literal store into multi-slot element".to_string(),
                            ));
                        }
                        let val = if *b { 1 } else { 0 };
                        let tmp = builder.layout.reserve_stack(1);
                        builder.store_immediate(val, tmp, format!("[fp + {}] = {}", tmp, val));
                        if let Some(off) = dyn_off {
                            builder.store_to_double_deref_fp_fp(
                                base_offset,
                                off,
                                tmp,
                                format!(
                                    "[[fp + {}] + [fp + {}]] = [fp + {}]",
                                    base_offset, off, tmp
                                ),
                            );
                        } else {
                            builder.store_to_double_deref_fp_imm(
                                tmp,
                                base_offset,
                                imm_off,
                                format!("[[fp + {}] + {}] = [fp + {}]", base_offset, imm_off, tmp),
                            );
                        }
                    }
                    Value::Operand(src_id) => {
                        let src_off = builder.layout.get_offset(*src_id)?;
                        if let Some(off) = dyn_off {
                            for s in 0..elem_size {
                                let slot_off = if imm_off != 0 || s != 0 {
                                    let t = builder.layout.reserve_stack(1);
                                    builder.felt_add_fp_imm(
                                        off,
                                        imm_off + s as i32,
                                        t,
                                        format!(
                                            "[fp + {}] = [fp + {}] + {} (slot {})",
                                            t,
                                            off,
                                            imm_off + s as i32,
                                            s
                                        ),
                                    );
                                    t
                                } else {
                                    off
                                };
                                builder.store_to_double_deref_fp_fp(
                                    base_offset,
                                    slot_off,
                                    src_off + s as i32,
                                    format!(
                                        "[[fp + {}] + [fp + {}]] = [fp + {}] (slot {})",
                                        base_offset,
                                        slot_off,
                                        src_off + s as i32,
                                        s
                                    ),
                                );
                            }
                        } else {
                            for s in 0..elem_size {
                                builder.store_to_double_deref_fp_imm(
                                    src_off + s as i32,
                                    base_offset,
                                    imm_off + s as i32,
                                    format!(
                                        "[[fp + {}] + {}] = [fp + {}] (slot {})",
                                        base_offset,
                                        imm_off + s as i32,
                                        src_off + s as i32,
                                        s
                                    ),
                                );
                            }
                        }
                    }
                    _ => {
                        return Err(CodegenError::InvalidMir(
                            "Invalid value for store".to_string(),
                        ))
                    }
                }
            }
            InstructionKind::HeapAllocCells { .. } => {
                // Handled at the basic-block level to enable label and data layout decisions.
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

        // Before resolving instruction labels, add rodata and data labels to the map
        // Compute total code length in QM31 words
        let mut code_len_qm31: u32 = 0;
        for instr_builder in &self.instructions {
            let opcode = instr_builder.inner_instr().opcode_value();
            let sz = cairo_m_common::Instruction::size_in_qm31s_for_opcode(opcode)
                .ok_or_else(|| CodegenError::InvalidMir(format!("Unknown opcode: {}", opcode)))?;
            code_len_qm31 += sz;
        }

        // Assign addresses to rodata blobs in insertion order
        let mut ro_offsets: Vec<u32> = Vec::with_capacity(self.rodata_blobs.len());
        let mut ro_running: u32 = 0;
        for blob in &self.rodata_blobs {
            ro_offsets.push(code_len_qm31 + ro_running);
            ro_running += blob.len() as u32;
        }

        // Sanity check: program size bound (2^30)
        let limit: u32 = 1 << 30;
        let ro_total = code_len_qm31 + ro_running;
        if ro_total >= limit {
            return Err(CodegenError::InternalError(format!(
                "Program (code + rodata) too large: {} >= {}",
                ro_total, limit
            )));
        }
        // Add rodata labels
        for (lbl, &blob_idx) in &self.rodata_label_to_blob {
            let addr = *ro_offsets
                .get(blob_idx)
                .ok_or_else(|| CodegenError::InternalError("Invalid rodata blob index".into()))?;
            label_map.insert(lbl.clone(), addr as usize);
        }

        // Now add mutable data labels after rodata
        let data_base = ro_total;
        let mut data_offsets: Vec<u32> = Vec::with_capacity(self.data_blobs.len());
        let mut d_running: u32 = 0;
        for blob in &self.data_blobs {
            data_offsets.push(data_base + d_running);
            d_running += blob.len() as u32;
        }
        let total = data_base + d_running;
        if total >= limit {
            return Err(CodegenError::InternalError(format!(
                "Program (code + rodata + data) too large: {} >= {}",
                total, limit
            )));
        }
        for (lbl, &blob_idx) in &self.data_label_to_blob {
            let addr = *data_offsets
                .get(blob_idx)
                .ok_or_else(|| CodegenError::InternalError("Invalid data blob index".into()))?;
            label_map.insert(lbl.clone(), addr as usize);
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
                CasmInstr::JmpRelImm { offset } => {
                    let &target_addr = label_map
                        .get(&label_name)
                        .ok_or_else(|| CodegenError::UnresolvedLabel(label_name.clone()))?;
                    let rel = (target_addr as i32) - (physical_pc as i32);
                    *offset = M31::from(rel);
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
                CasmInstr::StoreImm { imm, .. } => {
                    let &target_addr = label_map
                        .get(&label_name)
                        .ok_or_else(|| CodegenError::UnresolvedLabel(label_name.clone()))?;
                    *imm = M31::from(target_addr as i32);
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

        // Append rodata/data views (if any) after instructions
        // Compute code length in QM31 units for absolute base
        let mut code_len_qm31: u32 = 0;
        for ib in &self.instructions {
            if let Some(sz) = cairo_m_common::Instruction::size_in_qm31s_for_opcode(
                ib.inner_instr().opcode_value(),
            ) {
                code_len_qm31 += sz;
            }
        }

        // rodata section (immutable)
        if !self.rodata_blobs.is_empty() {
            result.push_str(&format!("---- rodata (base {}) ----\n", code_len_qm31));
            let mut addr = code_len_qm31 as usize;
            for (blob_idx, blob) in self.rodata_blobs.iter().enumerate() {
                result.push_str(&format!("; blob {} ({} words)\n", blob_idx, blob.len()));
                for q in blob {
                    let arr = q.to_m31_array();
                    let parts = [arr[0].0, arr[1].0, arr[2].0, arr[3].0];
                    result.push_str(&format!(
                        "{:4}: {} {} {} {}\n",
                        addr, parts[0], parts[1], parts[2], parts[3]
                    ));
                    addr += 1;
                }
            }
        }

        // data section (mutable), appended after rodata
        if !self.data_blobs.is_empty() {
            let rodata_len: u32 = self.rodata_blobs.iter().map(|b| b.len() as u32).sum();
            let data_base = code_len_qm31 + rodata_len;
            result.push_str(&format!("---- data (base {}) ----\n", data_base));
            let mut addr = data_base as usize;
            for (blob_idx, blob) in self.data_blobs.iter().enumerate() {
                result.push_str(&format!("; blob {} ({} words)\n", blob_idx, blob.len()));
                for q in blob {
                    let arr = q.to_m31_array();
                    let parts = [arr[0].0, arr[1].0, arr[2].0, arr[3].0];
                    result.push_str(&format!(
                        "{:4}: {} {} {} {}\n",
                        addr, parts[0], parts[1], parts[2], parts[3]
                    ));
                    addr += 1;
                }
            }
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

        let mut assert_fp_imm = 0;
        for item in &program.data {
            if let ProgramData::Instruction(instr) = item {
                if matches!(instr, CasmInstr::AssertEqFpImm { .. }) {
                    assert_fp_imm += 1;
                }
            }
        }

        assert!(
            assert_fp_imm == 2,
            "expected 2 AssertEqFpImm, got {}",
            assert_fp_imm
        );
    }
}

#[cfg(test)]
mod tests_heap_alloc {
    use super::*;
    use cairo_m_compiler_mir::{BasicBlock, MirFunction, MirModule, MirType, Terminator, Value};
    use stwo_prover::core::fields::m31::M31;

    // Helper: compute code length in QM31 units and index of first Value
    fn program_code_len_and_first_value_idx(program: &Program) -> (u32, Option<usize>) {
        let mut code_len_qm31: u32 = 0;
        let mut first_value_idx: Option<usize> = None;
        for (idx, item) in program.data.iter().enumerate() {
            match item {
                ProgramData::Instruction(instr) => {
                    code_len_qm31 += instr.size_in_qm31s();
                }
                ProgramData::Value(_) => {
                    first_value_idx = Some(idx);
                    break;
                }
            }
        }
        (code_len_qm31, first_value_idx)
    }

    #[test]
    fn heap_alloc_cells_emits_heap_cursor_and_addresses_it_via_label() {
        // Build MIR: fn main() { let p: felt = heapalloccells 3; }
        let mut module = MirModule::new();
        let mut f = MirFunction::new("main".to_string());

        // Destination pointer value
        let ptr_id = f.new_typed_value_id(MirType::Felt);
        let mut block = BasicBlock::new();
        block.push_instruction(cairo_m_compiler_mir::Instruction::heap_alloc_cells(
            ptr_id,
            Value::integer(3),
        ));
        block.terminator = Terminator::return_void();
        f.basic_blocks.push(block);
        module.add_function(f);

        // Generate CASM and compile
        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();
        let program = gen.compile().unwrap();

        // Expect at least one Value (our HEAP_CURSOR cell) appended after code
        let (code_len_qm31, first_value_idx) = program_code_len_and_first_value_idx(&program);
        let first_value_idx = first_value_idx.expect("Program should contain data value(s)");

        // Validate the first data value is the zero-initialized heap cursor
        match &program.data[first_value_idx] {
            ProgramData::Value(q) => {
                let arr = q.to_m31_array();
                assert_eq!(arr[0].0, 0, "heap cursor init must be zero");
                assert_eq!(arr[1].0, 0);
                assert_eq!(arr[2].0, 0);
                assert_eq!(arr[3].0, 0);
            }
            _ => panic!("Expected first data item to be a value"),
        }

        // The first instruction must include a StoreImm that materializes the absolute address
        // of the HEAP_CURSOR cell. That immediate equals the absolute base of the first Value,
        // which is code_len_qm31 in QM31 units.
        let heap_addr_m31 = M31::from(code_len_qm31 as i32);

        // Scan for a StoreImm imm == heap_addr_m31
        let mut found = false;
        for item in &program.data {
            if let ProgramData::Instruction(cairo_m_common::Instruction::StoreImm { imm, .. }) =
                item
            {
                if imm.0 == heap_addr_m31.0 {
                    found = true;
                    break;
                }
            }
        }
        assert!(
            found,
            "Expected a StoreImm that materializes the HEAP_CURSOR absolute address ({heap_addr_m31:?})"
        );
    }

    #[test]
    fn heap_alloc_cells_includes_double_deref_ops() {
        // Build MIR: fn main() { let n: felt = 5; let p: felt = heapalloccells n; }
        let mut module = MirModule::new();
        let mut f = MirFunction::new("main".to_string());

        let n_id = f.new_typed_value_id(MirType::Felt);
        let ptr_id = f.new_typed_value_id(MirType::Felt);

        let mut block = BasicBlock::new();
        block.push_instruction(cairo_m_compiler_mir::Instruction::assign(
            n_id,
            Value::integer(5),
            MirType::Felt,
        ));
        block.push_instruction(cairo_m_compiler_mir::Instruction::heap_alloc_cells(
            ptr_id,
            Value::operand(n_id),
        ));
        block.terminator = Terminator::return_void();
        f.basic_blocks.push(block);
        module.add_function(f);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();
        let program = gen.compile().unwrap();

        let mut saw_load_cursor = false;
        let mut saw_store_cursor = false;
        for item in &program.data {
            if let ProgramData::Instruction(instr) = item {
                match instr {
                    CasmInstr::StoreDoubleDerefFp { .. } => saw_load_cursor = true,
                    CasmInstr::StoreToDoubleDerefFpImm { .. } => saw_store_cursor = true,
                    _ => {}
                }
            }
        }
        assert!(
            saw_load_cursor,
            "expected StoreDoubleDerefFp for loading heap cursor"
        );
        assert!(
            saw_store_cursor,
            "expected StoreToDoubleDerefFpImm for storing heap cursor"
        );
    }
}

#[cfg(test)]
mod tests_rodata {
    use super::*;
    use cairo_m_compiler_mir::{
        BasicBlock, MirFunction, MirModule, MirType, Place, Terminator, Value,
    };
    use stwo_prover::core::fields::m31::M31;
    use stwo_prover::core::fields::qm31::QM31;

    // Focused test: literal u32 array lowered to rodata and dynamic index loads from rodata base
    #[test]
    fn rodata_u32_array_dynamic_index() {
        // Build MIR: fn main(i: felt) -> u32 { let arr: [u32;3] = [1,2,4]; return arr[i]; }
        let mut module = MirModule::new();
        let mut f = MirFunction::new("main".to_string());

        // Parameter i: felt
        let i_id = f.new_typed_value_id(MirType::Felt);
        f.parameters.push(i_id);

        // Create array value
        let arr_id = f.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });
        let mut block = BasicBlock::new();
        block.push_instruction(cairo_m_compiler_mir::Instruction {
            kind: cairo_m_compiler_mir::InstructionKind::MakeFixedArray {
                dest: arr_id,
                elements: vec![Value::integer(1), Value::integer(2), Value::integer(4)],
                element_ty: MirType::U32,
                is_const: true,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        });

        // Dynamic index load into u32
        let res_id = f.new_typed_value_id(MirType::U32);
        let place = Place::new(arr_id).with_index(Value::operand(i_id));
        block.push_instruction(cairo_m_compiler_mir::Instruction {
            kind: cairo_m_compiler_mir::InstructionKind::Load {
                dest: res_id,
                place,
                ty: MirType::U32,
            },
            source_span: None,
            source_expr_id: None,
            comment: None,
        });

        // Return the loaded u32
        block.terminator = Terminator::return_value(Value::operand(res_id));
        f.return_values.push(res_id);
        f.basic_blocks.push(block);

        module.add_function(f);

        // Generate CASM and compile to Program
        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();
        let program = gen.compile().unwrap();

        // Compute rodata base by summing instruction sizes in QM31 units
        let mut code_len_qm31: u32 = 0;
        let mut first_value_idx: Option<usize> = None;
        for (idx, item) in program.data.iter().enumerate() {
            match item {
                ProgramData::Instruction(instr) => {
                    code_len_qm31 += instr.size_in_qm31s();
                }
                ProgramData::Value(_) => {
                    first_value_idx = Some(idx);
                    break;
                }
            }
        }
        let ro_base = code_len_qm31;
        assert!(
            first_value_idx.is_some(),
            "Program should contain rodata values"
        );

        // Assert that some StoreImm immediate equals rodata base (array pointer setup)
        let mut saw_ptr = false;
        for item in &program.data {
            if let ProgramData::Instruction(CasmInstr::StoreImm { imm, .. }) = item {
                if imm.0 == ro_base {
                    saw_ptr = true;
                    break;
                }
            }
        }
        assert!(
            saw_ptr,
            "Expected a StoreImm with imm == rodata base {}",
            ro_base
        );

        // Validate rodata blob contents for [1u32, 2u32, 4u32]
        // For u32: two words per element (lo then hi limb), each as QM31([limb, 0, 0, 0])
        let expected_limbs = [1u32, 0, 2, 0, 4, 0];
        // Collect the raw QM31 rodata values after the first Value item
        let mut ro_values: Vec<QM31> = Vec::new();
        for item in &program.data[first_value_idx.unwrap()..] {
            if let ProgramData::Value(q) = item {
                ro_values.push(*q);
            }
        }
        // Compare prefix of rodata sequence equal to expected limbs x QM31
        assert!(
            ro_values.len() >= expected_limbs.len(),
            "Rodata too short: have {} values, need {}",
            ro_values.len(),
            expected_limbs.len()
        );
        for (i, limb) in expected_limbs.iter().enumerate() {
            let arr = ro_values[i].to_m31_array();
            assert_eq!(arr[0], M31::from(*limb), "rodata limb {} mismatch", i);
            assert_eq!(arr[1], M31::from(0));
            assert_eq!(arr[2], M31::from(0));
            assert_eq!(arr[3], M31::from(0));
        }
    }

    // Ensure identical literal arrays are deduplicated into one rodata blob
    #[test]
    fn rodata_deduplication_for_identical_arrays() {
        let mut module = MirModule::new();
        let mut f = MirFunction::new("main".to_string());

        // Parameter i: felt
        let i_id = f.new_typed_value_id(MirType::Felt);
        f.parameters.push(i_id);

        // Three arrays with identical contents
        let arr1 = f.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });
        let arr2 = f.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });
        let arr3 = f.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });

        let mut block = BasicBlock::new();
        for &arr in &[arr1, arr2, arr3] {
            block.push_instruction(cairo_m_compiler_mir::Instruction::make_const_fixed_array(
                arr,
                vec![Value::integer(1), Value::integer(2), Value::integer(4)],
                MirType::U32,
            ));
        }

        // Use dynamic index to force load path (not immediate fold)
        let d1 = f.new_typed_value_id(MirType::U32);
        let d2 = f.new_typed_value_id(MirType::U32);
        let d3 = f.new_typed_value_id(MirType::U32);
        for (arr, dest) in [arr1, arr2, arr3].iter().copied().zip([d1, d2, d3]) {
            let place = Place::new(arr).with_index(Value::operand(i_id));
            block.push_instruction(cairo_m_compiler_mir::Instruction::load(
                dest,
                place,
                MirType::U32,
            ));
        }

        block.terminator = Terminator::return_value(Value::operand(d1));
        f.return_values.push(d1);
        f.basic_blocks.push(block);
        module.add_function(f);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();

        // Expect exactly one rodata blob due to deduplication
        assert_eq!(
            gen.rodata_blobs.len(),
            1,
            "Expected rodata deduplication to a single blob"
        );
    }

    // Ensure arrays that escape via call are not rodata-optimized
    #[test]
    fn rodata_not_emitted_for_escaping_arrays() {
        // module with callee that stores into array
        let mut module = MirModule::new();

        // callee: fn store_it(arr: [u32;3], idx: felt) -> u32 { arr[idx] = 0; return 0u32 }
        let mut callee = MirFunction::new("store_it".to_string());
        let arr_param = callee.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });
        let idx_param = callee.new_typed_value_id(MirType::Felt);
        callee.parameters.push(arr_param);
        callee.parameters.push(idx_param);
        let mut cblock = BasicBlock::new();
        // store arr[idx] = 0
        let zero_u32 = Value::integer(0);
        let store_place = Place::new(arr_param).with_index(Value::operand(idx_param));
        cblock.push_instruction(cairo_m_compiler_mir::Instruction::store(
            store_place,
            zero_u32,
            MirType::U32,
        ));
        // return 0u32
        let ret0 = callee.new_typed_value_id(MirType::U32);
        cblock.push_instruction(cairo_m_compiler_mir::Instruction::assign(
            ret0,
            Value::integer(0),
            MirType::U32,
        ));
        cblock.terminator = Terminator::return_value(Value::operand(ret0));
        callee.return_values.push(ret0);
        callee.basic_blocks.push(cblock);
        let callee_id = module.add_function(callee);

        // caller: creates literal array and passes to callee
        let mut caller = MirFunction::new("main".to_string());
        let idx = caller.new_typed_value_id(MirType::Felt);
        caller.parameters.push(idx);
        let arr = caller.new_typed_value_id(MirType::FixedArray {
            element_type: Box::new(MirType::U32),
            size: 3,
        });
        let mut b = BasicBlock::new();
        b.push_instruction(cairo_m_compiler_mir::Instruction::make_fixed_array(
            arr,
            vec![Value::integer(1), Value::integer(2), Value::integer(3)],
            MirType::U32,
        ));
        // call store_it(arr, idx)
        let ret = caller.new_typed_value_id(MirType::U32);
        let sig = cairo_m_compiler_mir::instruction::CalleeSignature {
            param_types: vec![
                MirType::FixedArray {
                    element_type: Box::new(MirType::U32),
                    size: 3,
                },
                MirType::Felt,
            ],
            return_types: vec![MirType::U32],
        };
        b.push_instruction(cairo_m_compiler_mir::Instruction::call(
            vec![ret],
            callee_id,
            vec![Value::operand(arr), Value::operand(idx)],
            sig,
        ));
        b.terminator = Terminator::return_value(Value::operand(ret));
        caller.return_values.push(ret);
        caller.basic_blocks.push(b);
        module.add_function(caller);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();
        // No rodata should be emitted because the only candidate array escapes via call
        assert!(
            gen.rodata_blobs.is_empty(),
            "Escaping arrays must not be placed in rodata"
        );
    }

    // Constant-index folding is handled by MIR; no codegen folding test here.
}

/// Convert MIR types to ABI types for program metadata
fn abi_type_from_mir(ty: &MirType) -> CodegenResult<AbiType> {
    let out = match ty {
        MirType::Felt => AbiType::Felt,
        MirType::Bool => AbiType::Bool,
        MirType::U32 => AbiType::U32,
        MirType::Pointer { element } => AbiType::Pointer {
            element: Box::new(abi_type_from_mir(element)?),
            len: None,
        },
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
    use cairo_m_common::instruction::{
        STORE_DOUBLE_DEREF_FP, STORE_DOUBLE_DEREF_FP_FP, STORE_MUL_FP_IMM,
        STORE_TO_DOUBLE_DEREF_FP_IMM,
    };
    use cairo_m_compiler_mir::{
        BasicBlock, Instruction, MirFunction, MirModule, MirType, Place, Terminator, Value,
    };
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
        match &compiled.data[0] {
            ProgramData::Instruction(CasmInstr::StoreImm { imm, dst_off }) => {
                assert_eq!(*imm, M31::from(42));
                assert_eq!(*dst_off, M31::from(-3));
            }
            other => panic!("Expected first data to be StoreImm, got: {:?}", other),
        }
    }

    fn count_opcode(gen: &CodeGenerator, opcode: u32) -> usize {
        gen.instructions()
            .iter()
            .filter(|instr| instr.inner_instr().opcode_value() == opcode)
            .count()
    }

    #[test]
    fn load_static_index_emits_double_deref() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("load_static".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::Felt),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let dest = function.new_typed_value_id(MirType::Felt);

        function.parameters.push(arr);
        function.return_values.push(dest);

        let place = Place::new(arr).with_index(Value::integer(2));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::load(dest, place, MirType::Felt));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::return_value(Value::operand(dest));

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();

        assert!(
            count_opcode(&gen, STORE_DOUBLE_DEREF_FP) >= 1,
            "expected static load to use STORE_DOUBLE_DEREF_FP"
        );
    }

    #[test]
    fn store_static_index_emits_double_deref() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("store_static".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::Felt),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let value = function.new_typed_value_id(MirType::Felt);

        function.parameters.push(arr);
        function.parameters.push(value);

        let place = Place::new(arr).with_index(Value::integer(1));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::store(
                place,
                Value::operand(value),
                MirType::Felt,
            ));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::Return { values: vec![] };

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();

        assert!(
            count_opcode(&gen, STORE_TO_DOUBLE_DEREF_FP_IMM) >= 1,
            "expected static store to use STORE_TO_DOUBLE_DEREF_FP_IMM"
        );
    }

    #[test]
    fn load_dynamic_index_uses_fp_fp() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("load_dynamic".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::Felt),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let idx = function.new_typed_value_id(MirType::Felt);
        let dest = function.new_typed_value_id(MirType::Felt);

        function.parameters.push(arr);
        function.parameters.push(idx);
        function.return_values.push(dest);

        let place = Place::new(arr).with_index(Value::operand(idx));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::load(dest, place, MirType::Felt));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::return_value(Value::operand(dest));

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();

        assert!(
            count_opcode(&gen, STORE_DOUBLE_DEREF_FP_FP) >= 1,
            "expected dynamic load to use STORE_DOUBLE_DEREF_FP_FP"
        );
    }

    #[test]
    fn load_dynamic_u32_scales_index() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("load_dynamic_u32".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::U32),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let idx = function.new_typed_value_id(MirType::Felt);
        let dest = function.new_typed_value_id(MirType::U32);

        function.parameters.push(arr);
        function.parameters.push(idx);
        function.return_values.push(dest);

        let place = Place::new(arr).with_index(Value::operand(idx));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::load(dest, place, MirType::U32));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::return_value(Value::operand(dest));

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        gen.generate_module(&module).unwrap();

        assert!(
            count_opcode(&gen, STORE_MUL_FP_IMM) >= 1,
            "expected u32 dynamic load to scale index"
        );
        assert!(
            count_opcode(&gen, STORE_DOUBLE_DEREF_FP_FP) >= 2,
            "expected u32 load to fetch two slots"
        );
    }

    #[test]
    fn store_dynamic_u32_scales_index() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("store_dynamic_u32".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::U32),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let idx = function.new_typed_value_id(MirType::Felt);
        let value = function.new_typed_value_id(MirType::U32);

        function.parameters.push(arr);
        function.parameters.push(idx);
        function.parameters.push(value);

        let place = Place::new(arr).with_index(Value::operand(idx));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::store(
                place,
                Value::operand(value),
                MirType::U32,
            ));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::Return { values: vec![] };

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        let result = gen.generate_module(&module);
        assert!(result.is_ok(), "codegen failed: {:?}", result);
        let gen = gen;

        assert!(
            count_opcode(&gen, STORE_MUL_FP_IMM) >= 1,
            "expected u32 store with dynamic index to scale the offset"
        );
    }

    #[test]
    fn load_rejects_non_felt_index() {
        let mut module = MirModule::new();
        let mut function = MirFunction::new("load_bad_index".to_string());

        let arr_ty = MirType::Pointer {
            element: Box::new(MirType::Felt),
        };
        let arr = function.new_typed_value_id(arr_ty);
        let idx = function.new_typed_value_id(MirType::Bool);
        let dest = function.new_typed_value_id(MirType::Felt);

        function.parameters.push(arr);
        function.parameters.push(idx);
        function.return_values.push(dest);

        let place = Place::new(arr).with_index(Value::operand(idx));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .push_instruction(Instruction::load(dest, place, MirType::Felt));
        function
            .get_basic_block_mut(function.entry_block)
            .unwrap()
            .terminator = Terminator::return_value(Value::operand(dest));

        module.add_function(function);

        let mut gen = CodeGenerator::new();
        let err = gen.generate_module(&module).unwrap_err();
        assert!(
            matches!(err, CodegenError::InvalidMir(ref msg) if msg.contains("Array index must be a felt")),
            "expected non-felt index to be rejected, got {:?}",
            err
        );
    }
}
