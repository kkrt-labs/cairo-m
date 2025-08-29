//! # CASM Instruction Builder
//!
//! This module provides utilities for building CASM instructions from MIR values
//! and function layouts.
//!
//! Invariants (post-legalization): The codegen pipeline now runs a target-specific
//! MIR legalization pass before using this builder. As a result, when invoked via
//! `CodeGenerator`, the builder may assume that u32 comparisons are restricted to
//! `U32Eq` and strict `U32Less`. Other u32 comparison forms are rewritten by the
//! legalizer using swaps and boolean `Not`. The builder still supports the full
//! set of u32 comparison ops when called directly (e.g., unit tests), but the
//! generator asserts that illegalized ops do not reach this stage.

use cairo_m_common::instruction::*;
use cairo_m_common::Instruction as CasmInstr;
use cairo_m_compiler_mir::{BinaryOp, DataLayout, Literal, MirType, Value, ValueId};
use cairo_m_compiler_parser::parser::UnaryOp;
use stwo_prover::core::fields::m31::M31;

use crate::{CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label, Operand};

// Centralized emission helpers for instruction/label/touch routing.
mod aggregates;
mod asserts;
pub(crate) mod calls;
mod ctrlflow;
mod emit;
mod felt;
mod normalize;
mod store;
mod u32_ops;

/// Helper to split a u32 value into low and high 16-bit parts
#[inline]
pub(super) const fn split_u32_value(value: u32) -> (i32, i32) {
    ((value & 0xFFFF) as i32, ((value >> 16) & 0xFFFF) as i32)
}

#[rustfmt::skip]
fn felt_fp_fp_rebuild(orig: &CasmInstr, src0: M31, src1: M31, dst: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::StoreAddFpFp { .. } => CasmInstr::StoreAddFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::StoreSubFpFp { .. } => CasmInstr::StoreSubFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::StoreMulFpFp { .. } => CasmInstr::StoreMulFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::StoreDivFpFp { .. } => CasmInstr::StoreDivFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        _ => return Err(CodegenError::UnsupportedInstruction("Expected felt fp+fp instruction".into())),
    })
}

#[rustfmt::skip]
fn felt_fp_imm_rebuild(orig: &CasmInstr, src: M31, imm: M31, dst: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::StoreAddFpImm { .. } => CasmInstr::StoreAddFpImm { src_off: src, imm, dst_off: dst },
        CasmInstr::StoreMulFpImm { .. } => CasmInstr::StoreMulFpImm { src_off: src, imm, dst_off: dst },
        _ => return Err(CodegenError::UnsupportedInstruction("Expected felt fp+imm instruction".into())),
    })
}

#[rustfmt::skip]
fn u32_fp_fp_rebuild(orig: &CasmInstr, src0: M31, src1: M31, dst: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::U32StoreAddFpFp { .. } => CasmInstr::U32StoreAddFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::U32StoreSubFpFp { .. } => CasmInstr::U32StoreSubFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::U32StoreMulFpFp { .. } => CasmInstr::U32StoreMulFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::U32StoreDivFpFp { .. } => CasmInstr::U32StoreDivFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::U32StoreEqFpFp { .. } => CasmInstr::U32StoreEqFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        CasmInstr::U32StoreLtFpFp { .. } => CasmInstr::U32StoreLtFpFp { src0_off: src0, src1_off: src1, dst_off: dst },
        _ => return Err(CodegenError::UnsupportedInstruction("Expected u32 fp+fp instruction".into())),
    })
}

#[rustfmt::skip]
fn u32_fp_imm_rebuild(orig: &CasmInstr, src: M31, imm_lo: M31, imm_hi: M31, dst: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::U32StoreAddFpImm { .. } => CasmInstr::U32StoreAddFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreMulFpImm { .. } => CasmInstr::U32StoreMulFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreDivFpImm { .. } => CasmInstr::U32StoreDivFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreEqFpImm { .. } => CasmInstr::U32StoreEqFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreLtFpImm { .. } => CasmInstr::U32StoreLtFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreAndFpImm { .. } => CasmInstr::U32StoreAndFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreOrFpImm { .. } => CasmInstr::U32StoreOrFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        CasmInstr::U32StoreXorFpImm { .. } => CasmInstr::U32StoreXorFpImm { src_off: src, imm_lo, imm_hi, dst_off: dst },
        _ => return Err(CodegenError::UnsupportedInstruction("Expected u32 fp+imm instruction".into())),
    })
}

/// Builder for generating CASM instructions
///
/// This struct manages the generation of CASM instructions, handling the
/// translation from MIR values to fp-relative memory addresses.
#[derive(Debug)]
pub struct CasmBuilder {
    /// Generated instructions
    pub(super) instructions: Vec<InstructionBuilder>,
    /// Labels that need to be resolved
    pub(super) labels: Vec<Label>,
    /// Current function layout for offset lookups
    layout: FunctionLayout,
    /// Counter for generating unique labels
    pub(super) label_counter: usize,
    /// Highest fp+ offset that has been written to (for optimization tracking)
    pub(super) max_written_offset: i32,
}

/// Represents the type of array operation to perform
pub enum ArrayOperation {
    /// Load an element from array into dest
    Load { dest: ValueId },
    /// Store a value into an array element, creating a new array in dest
    Store { dest: ValueId, value: Value },
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

    /// Get the current "live" frame usage based on what's actually been written
    pub const fn live_frame_usage(&self) -> i32 {
        self.max_written_offset + 1 // Convert from 0-based offset to size
    }

    /// Get the current pre-allocated frame usage
    pub const fn current_frame_usage(&self) -> i32 {
        self.layout.current_frame_usage()
    }

    // store_immediate and store_u32_immediate moved to builder/store.rs to group STORE opcodes.

    /// Generate type-aware assignment instruction
    ///
    /// Handles assignments for all types including aggregates (structs, tuples)
    pub(crate) fn assign(
        &mut self,
        dest: ValueId,
        source: Value,
        ty: &MirType,
        target_offset: Option<i32>,
    ) -> CodegenResult<()> {
        let size = DataLayout::memory_size_of(ty);

        // Determine destination offset
        let dest_off = if let Some(offset) = target_offset {
            // Use the provided target offset and map the ValueId to it
            self.layout.map_value(dest, offset);
            offset
        } else {
            // Get the pre-allocated offset from the layout, or allocate on demand
            match self.layout.get_offset(dest) {
                Ok(offset) => offset,
                Err(_) => {
                    // Value wasn't pre-allocated, allocate it now
                    let alloc_size = DataLayout::memory_size_of(ty);
                    self.layout.allocate_local(dest, alloc_size)?
                }
            }
        };

        match source {
            Value::Literal(Literal::Integer(imm)) => {
                // Handle immediate values based on size
                if size == 1 {
                    // Single slot value (felt, bool, pointer)
                    self.store_immediate(imm, dest_off, format!("[fp + {dest_off}] = {imm}"));
                } else if size == 2 && matches!(ty, MirType::U32) {
                    // U32 value
                    let value = imm;
                    self.store_u32_immediate(
                        value,
                        dest_off,
                        format!(
                            "u32([fp + {dest_off}], [fp + {}]) = u32({value})",
                            dest_off + 1
                        ),
                    );
                } else {
                    return Err(CodegenError::UnsupportedInstruction(format!(
                        "Cannot assign immediate to aggregate type of size {}",
                        size
                    )));
                }
            }

            Value::Literal(Literal::Boolean(b)) => {
                if size != 1 {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Boolean literal must be single-slot".to_string(),
                    ));
                }
                self.store_immediate(b as u32, dest_off, format!("[fp + {dest_off}] = {b}"));
            }

            Value::Operand(src_id) => {
                // Copy from another value
                let src_off = self.layout.get_offset(src_id)?;

                if size == 1 {
                    // Single slot copy
                    let comment_suffix = if matches!(ty, MirType::FixedArray { .. }) {
                        " (array pointer)"
                    } else {
                        ""
                    };
                    self.store_copy_single(
                        src_off,
                        dest_off,
                        format!("[fp + {dest_off}] = [fp + {src_off}] + 0{comment_suffix}"),
                    );
                } else if matches!(ty, MirType::U32) {
                    // U32 copy using dedicated instruction
                    self.store_copy_u32(src_off, dest_off, "");
                } else {
                    // Multi-slot copy for aggregates (structs, tuples, etc.)
                    // Copy each slot individually with legacy comments
                    self.copy_slots(src_off, dest_off, size, "");
                }
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported assignment source: {:?}",
                    source
                )));
            }
        }

        Ok(())
    }

    /// Generate unary operation instruction
    pub(crate) fn unary_op(
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
                self.felt_arith(
                    BinaryOp::Sub,
                    dest_off,
                    Value::Literal(Literal::Integer(0)),
                    source,
                )?;
            }
            UnaryOp::Not => {
                // the `!` operator is not supported on the `felt` type, but is on the `bool` type,
                // which is simply `== 0`
                self.bool_not(dest_off, source)?;
            }
        }
        Ok(())
    }

    /// Generate a binary operation instruction
    ///
    /// If target_offset is provided, writes directly to that location.
    /// Otherwise, allocates a new local variable.
    pub(crate) fn binary_op(
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
                self.felt_arith(op, dest_off, left, right)?;
            }
            BinaryOp::Eq => self.felt_eq(dest_off, left, right)?,
            BinaryOp::Neq => self.felt_neq(dest_off, left, right)?,
            BinaryOp::And => self.bool_and(dest_off, left, right)?,
            BinaryOp::Or => self.bool_or(dest_off, left, right)?,
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Felt comparisons beyond Eq/Neq are unsupported".into(),
                ));
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
            | BinaryOp::U32GreaterEqual
            | BinaryOp::U32BitwiseAnd
            | BinaryOp::U32BitwiseOr
            | BinaryOp::U32BitwiseXor => {
                self.u32_op(op, dest_off, left, right)?;
            }
        }

        Ok(())
    }

    /// Compute a binary operation directly into a raw `dest_off` without a `ValueId`.
    /// This is primarily used by branch generation to avoid materializing a SSA name.
    pub(crate) fn compute_into_offset(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                self.felt_arith(op, dest_off, left, right)
            }
            BinaryOp::Eq => self.felt_eq(dest_off, left, right),
            BinaryOp::Neq => self.felt_neq(dest_off, left, right),
            BinaryOp::And => self.bool_and(dest_off, left, right),
            BinaryOp::Or => self.bool_or(dest_off, left, right),
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                Err(CodegenError::UnsupportedInstruction(
                    "Felt comparisons beyond Eq/Neq are unsupported".into(),
                ))
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
            | BinaryOp::U32GreaterEqual
            | BinaryOp::U32BitwiseAnd
            | BinaryOp::U32BitwiseOr
            | BinaryOp::U32BitwiseXor => self.u32_op(op, dest_off, left, right),
        }
    }

    /// Get the generated instructions
    pub(crate) fn instructions(&self) -> &[InstructionBuilder] {
        &self.instructions
    }

    /// Get the labels
    pub(crate) fn labels(&self) -> &[Label] {
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

    // TODO: This should be modeled as a MIR-like pre-pass.
    /// Removes any occurrences of instructions where two or more offsets are the same.
    ///
    /// - Rationale: the prover cannot handle multiple accesses to the same memory
    ///   location within one instruction (read-after-write hazards). We conservatively
    ///   expand such instructions using temporaries.
    pub(crate) fn resolve_duplicate_offsets(&mut self) -> CodegenResult<()> {
        let mut new_instructions = Vec::new();
        // Track how instruction indices change: original_index -> new_index_range
        let mut index_mapping: Vec<Option<std::ops::Range<usize>>> = Vec::new();

        // Reserve space for temporary variables only when needed
        // Keep separate temps for felt (1-slot) and U32 (2-slot) operations to avoid size conflicts
        let mut felt_temp1: Option<i32> = None;
        let mut felt_temp2: Option<i32> = None;
        let mut u32_temp1: Option<i32> = None;
        let mut u32_temp2: Option<i32> = None;

        for instr in self.instructions.iter() {
            let current_new_index = new_instructions.len();

            let replacement_instructions = match instr.opcode {
                STORE_ADD_FP_FP | STORE_SUB_FP_FP | STORE_MUL_FP_FP | STORE_DIV_FP_FP => {
                    // Reserve temp variables on demand for felt operations (need 2 single-slot temps)
                    if felt_temp1.is_none() || felt_temp2.is_none() {
                        felt_temp1 = Some(self.layout.reserve_stack(1));
                        felt_temp2 = Some(self.layout.reserve_stack(1));
                    }
                    Self::handle_fp_fp_duplicates(instr, felt_temp1.unwrap(), felt_temp2.unwrap())
                }
                STORE_ADD_FP_IMM | STORE_MUL_FP_IMM => {
                    // Reserve temp variable on demand for felt operations with immediate (need 1 single-slot temp)
                    if felt_temp1.is_none() {
                        felt_temp1 = Some(self.layout.reserve_stack(1));
                    }
                    Self::handle_fp_imm_duplicates(instr, felt_temp1.unwrap())
                }
                // U32 arithmetic operations with FP operands
                U32_STORE_ADD_FP_FP | U32_STORE_SUB_FP_FP | U32_STORE_MUL_FP_FP
                | U32_STORE_DIV_FP_FP => {
                    // Reserve temp variables on demand for U32 operations (need 2 slots each)
                    if u32_temp1.is_none() || u32_temp2.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                        u32_temp2 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_fp_duplicates(instr, u32_temp1.unwrap(), u32_temp2.unwrap())
                }
                // U32 arithmetic operations with immediate operands
                U32_STORE_ADD_FP_IMM | U32_STORE_MUL_FP_IMM | U32_STORE_DIV_FP_IMM => {
                    // Reserve temp variable on demand for U32 operations (need 2 slots)
                    if u32_temp1.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_imm_duplicates(instr, u32_temp1.unwrap())
                }
                // U32 comparison operations with FP operands (result is felt, not u32)
                U32_STORE_EQ_FP_FP | U32_STORE_LT_FP_FP => {
                    // Reserve temp variables on demand for U32 comparisons (need 2 slots each for operands)
                    if u32_temp1.is_none() || u32_temp2.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                        u32_temp2 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_fp_duplicates(instr, u32_temp1.unwrap(), u32_temp2.unwrap())
                }
                // U32 comparison operations with immediate operands
                U32_STORE_EQ_FP_IMM | U32_STORE_LT_FP_IMM => {
                    // Reserve temp variable on demand for U32 comparisons (need 2 slots)
                    if u32_temp1.is_none() {
                        u32_temp1 = Some(self.layout.reserve_stack(2));
                    }
                    Self::handle_u32_fp_imm_duplicates(instr, u32_temp1.unwrap())
                }
                _ => {
                    // Keep instruction as-is
                    Ok(vec![instr.clone()])
                }
            }?;

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
    // TODO: this should be done in a system similar to MIR's `passes`.
    fn handle_fp_fp_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
        temp_var_offset2: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        let typed_instr = instr.get_typed_instruction().unwrap();
        #[rustfmt::skip]
        let (off0, off1, off2) = match typed_instr {
            CasmInstr::StoreAddFpFp { src0_off, src1_off, dst_off, } => (src0_off, src1_off, dst_off),
            CasmInstr::StoreSubFpFp { src0_off, src1_off, dst_off, } => (src0_off, src1_off, dst_off),
            CasmInstr::StoreMulFpFp { src0_off, src1_off, dst_off, } => (src0_off, src1_off, dst_off),
            CasmInstr::StoreDivFpFp { src0_off, src1_off, dst_off, } => (src0_off, src1_off, dst_off),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Could not handle fp_fp duplicate".to_string(),
                ))
            }
        };

        if off0 == off1 && off1 == off2 {
            // The three offsets are the same, store off0 and off1 in temp vars and replace with 3 instructions
            Ok(vec![
                InstructionBuilder::from_instr(
                    CasmInstr::StoreAddFpImm {
                        src_off: *off0,
                        imm: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                ),
                InstructionBuilder::from_instr(
                    CasmInstr::StoreAddFpImm {
                        src_off: *off1,
                        imm: M31::from(0),
                        dst_off: M31::from(temp_var_offset2),
                    },
                    Some(format!("[fp + {temp_var_offset2}] = [fp + {off1}] + 0")),
                ),
                InstructionBuilder::from_instr(
                    felt_fp_fp_rebuild(
                        typed_instr,
                        M31::from(temp_var_offset),
                        M31::from(temp_var_offset2),
                        *off2,
                    )?,
                    Some(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {temp_var_offset2}]"
                    )),
                ),
            ])
        } else if off0 == off1 || off0 == off2 {
            // off0 is a duplicate, store off0 in a temp var and replace with 2 instructions
            Ok(vec![
                InstructionBuilder::from_instr(
                    CasmInstr::StoreAddFpImm {
                        src_off: *off0,
                        imm: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!("[fp + {temp_var_offset}] = [fp + {off0}] + 0")),
                ),
                InstructionBuilder::from_instr(
                    felt_fp_fp_rebuild(typed_instr, M31::from(temp_var_offset), *off1, *off2)?,
                    Some(format!(
                        "[fp + {off2}] = [fp + {temp_var_offset}] op [fp + {off1}]"
                    )),
                ),
            ])
        } else if off1 == off2 {
            // off1 is a duplicate, store off1 in a temp var and replace with 2 instructions
            Ok(vec![
                InstructionBuilder::from_instr(
                    CasmInstr::StoreAddFpImm {
                        src_off: *off1,
                        imm: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!("[fp + {temp_var_offset}] = [fp + {off1}] + 0")),
                ),
                InstructionBuilder::from_instr(
                    felt_fp_fp_rebuild(typed_instr, *off0, M31::from(temp_var_offset), *off2)?,
                    Some(format!(
                        "[fp + {off2}] = [fp + {off0}] op [fp + {temp_var_offset}]"
                    )),
                ),
            ])
        } else {
            // No duplicates, keep as-is
            Ok(vec![instr.clone()])
        }
    }

    /// Handles duplicate offsets in fp+immediate binary operations.
    /// Expands in-place operations using a temporary variable when source equals destination.
    // TODO: this should be done in a system similar to MIR's `passes`.
    fn handle_fp_imm_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        let typed_instr = instr.get_typed_instruction().unwrap();
        #[rustfmt::skip]
        let (src_off, imm, dst_off) = match typed_instr {
            CasmInstr::StoreAddFpImm { src_off, imm, dst_off, } => (*src_off, *imm, *dst_off),
            CasmInstr::StoreMulFpImm { src_off, imm, dst_off, } => (*src_off, *imm, *dst_off),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Could not handle fp_imm duplicate".to_string(),
                ))
            }
        };
        if src_off == dst_off {
            // src_off is a duplicate, store src_off in a temp var and replace with 2 instructions
            Ok(vec![
                InstructionBuilder::from_instr(
                    CasmInstr::StoreAddFpImm {
                        src_off,
                        imm: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!("[fp + {temp_var_offset}] = [fp + {src_off}] + 0")),
                ),
                InstructionBuilder::from_instr(
                    felt_fp_imm_rebuild(typed_instr, M31::from(temp_var_offset), imm, dst_off)?,
                    Some(format!(
                        "[fp + {dst_off}] = [fp + {temp_var_offset}] op {imm}"
                    )),
                ),
            ])
        } else {
            // No duplicates, keep as-is
            Ok(vec![instr.clone()])
        }
    }

    /// Handles duplicate offsets in U32 fp+fp operations.
    /// Similar to handle_fp_fp_duplicates but needs to handle 2-slot U32 values.
    /// For U32 comparisons, only the destination is 1 slot (felt result).
    // TODO: this should be done in a system similar to MIR's `passes`.
    fn handle_u32_fp_fp_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
        temp_var_offset2: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        #[rustfmt::skip]
        let (src0_off, src1_off, dst_off) = match instr.get_typed_instruction().unwrap() {
            CasmInstr::U32StoreAddFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            CasmInstr::U32StoreSubFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            CasmInstr::U32StoreMulFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            CasmInstr::U32StoreDivFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            CasmInstr::U32StoreEqFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            CasmInstr::U32StoreLtFpFp { src0_off, src1_off, dst_off, } => (*src0_off, *src1_off, *dst_off),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Expected u32 fp+fp instruction".into(),
                ))
            }
        };
        // Use typed instruction for rebuild
        let typed_instr = instr.get_typed_instruction().unwrap();
        // Check if this is a comparison (result is felt) or arithmetic (result is u32)
        let is_comparison = matches!(
            typed_instr,
            CasmInstr::U32StoreEqFpFp { .. } | CasmInstr::U32StoreLtFpFp { .. }
        );

        // For U32 values, we need to check overlaps considering 2-slot values
        // src0 uses [src0_off, src0_off+1], src1 uses [src1_off, src1_off+1]
        // dst uses [dst_off] for comparisons, [dst_off, dst_off+1] for arithmetic

        let src0_overlaps_src1 = src0_off == src1_off
            || src0_off == src1_off + M31::from(1)
            || src0_off + M31::from(1) == src1_off
            || src0_off + M31::from(1) == src1_off + M31::from(1);

        let src0_overlaps_dst = if is_comparison {
            src0_off == dst_off || src0_off + M31::from(1) == dst_off
        } else {
            src0_off == dst_off
                || src0_off == dst_off + M31::from(1)
                || src0_off + M31::from(1) == dst_off
                || src0_off + M31::from(1) == dst_off + M31::from(1)
        };

        let src1_overlaps_dst = if is_comparison {
            src1_off == dst_off || src1_off + M31::from(1) == dst_off
        } else {
            src1_off == dst_off
                || src1_off == dst_off + M31::from(1)
                || src1_off + M31::from(1) == dst_off
                || src1_off + M31::from(1) == dst_off + M31::from(1)
        };

        let res = if src0_overlaps_src1 && src0_overlaps_dst {
            // All three overlap, need to copy both sources to temp locations
            // We need 4 temp slots total (2 for each U32)
            vec![
                // Copy src0 to temp
                InstructionBuilder::from_instr(
                    CasmInstr::U32StoreAddFpImm {
                        src_off: src0_off,
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!(
                        "u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1,
                        src0_off + M31::from(1)
                    )),
                ),
                // Copy src1 to temp
                InstructionBuilder::from_instr(
                    CasmInstr::U32StoreAddFpImm {
                        src_off: src1_off,
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: M31::from(temp_var_offset2),
                    },
                    Some(format!(
                        "u32([fp + {temp_var_offset2}], [fp + {}]) = u32([fp + {src1_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset2 + 1,
                        src1_off + M31::from(1)
                    )),
                ),
                // Perform operation with temp locations
                InstructionBuilder::from_instr(
                    u32_fp_fp_rebuild(
                        typed_instr,
                        M31::from(temp_var_offset),
                        M31::from(temp_var_offset2),
                        dst_off,
                    )?,
                    Some(if is_comparison {
                        format!(
                            "[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {temp_var_offset2}], [fp + {}])",
                            temp_var_offset + 1,
                            temp_var_offset2 + 1
                        )
                    } else {
                        format!(
                            "u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {temp_var_offset2}], [fp + {}])",
                            dst_off + M31::from(1),
                            temp_var_offset,
                            temp_var_offset2
                        )
                    }),
                ),
            ]
        } else if src0_overlaps_dst {
            // src0 overlaps with dst, copy src0 to temp
            vec![
                InstructionBuilder::from_instr(
                    CasmInstr::U32StoreAddFpImm {
                        src_off: src0_off,
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!(
                        "u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1,
                        src0_off + M31::from(1)
                    )),
                ),
                InstructionBuilder::from_instr(
                    u32_fp_fp_rebuild(
                        typed_instr,
                        M31::from(temp_var_offset),
                        src1_off,
                        dst_off,
                    )?,
                    Some(if is_comparison {
                        format!(
                            "[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {src1_off}], [fp + {}])",
                            temp_var_offset + 1,
                            src1_off + M31::from(1)
                        )
                    } else {
                        format!(
                            "u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32([fp + {src1_off}], [fp + {}])",
                            dst_off + M31::from(1),
                            temp_var_offset + 1,
                            src1_off + M31::from(1)
                        )
                    }),
                ),
            ]
        } else if src1_overlaps_dst {
            // src1 overlaps with dst, copy src1 to temp
            vec![
                InstructionBuilder::from_instr(
                    CasmInstr::U32StoreAddFpImm {
                        src_off: src1_off,
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!(
                        "u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src1_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1,
                        src1_off + M31::from(1)
                    )),
                ),
                InstructionBuilder::from_instr(
                    u32_fp_fp_rebuild(
                        typed_instr,
                        src0_off,
                        M31::from(temp_var_offset),
                        dst_off,
                    )?,
                    Some(if is_comparison {
                        format!(
                            "[fp + {dst_off}] = u32([fp + {src0_off}], [fp + {}]) op u32([fp + {temp_var_offset}], [fp + {}])",
                            src0_off + M31::from(1),
                            temp_var_offset+1,
                        )
                    } else {
                        format!(
                            "u32([fp + {dst_off}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) op u32([fp + {temp_var_offset}], [fp + {}])",
                            dst_off + M31::from(1),
                            src0_off + M31::from(1),
                            temp_var_offset,
                        )
                    }),
                ),
            ]
        } else {
            // No overlaps, keep as-is
            vec![instr.clone()]
        };
        Ok(res)
    }

    /// Handles duplicate offsets in U32 fp+immediate operations.
    /// Similar to handle_fp_imm_duplicates but needs to handle 2-slot U32 values.
    // TODO: this should be done in a system similar to MIR's `passes`.
    fn handle_u32_fp_imm_duplicates(
        instr: &InstructionBuilder,
        temp_var_offset: i32,
    ) -> CodegenResult<Vec<InstructionBuilder>> {
        // Extract operands (legacy integers for overlap detection)
        let src_off = instr.op0().unwrap();
        let dst_off = if let Some(Operand::Literal(off)) = instr.operands.get(3) {
            *off
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 fp-imm instruction missing destination offset".to_string(),
            ));
        };

        // Get the immediate values (should be at positions 1 and 2)
        let imm_lo = if let Some(Operand::Literal(imm)) = instr.operands.get(1) {
            *imm
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 Store immediate low operand must be a literal".to_string(),
            ));
        };

        let imm_hi = if let Some(Operand::Literal(imm)) = instr.operands.get(2) {
            *imm
        } else {
            return Err(CodegenError::UnsupportedInstruction(
                "U32 Store immediate high operand must be a literal".to_string(),
            ));
        };

        // Use typed instruction for rebuild
        let typed_instr = instr.get_typed_instruction().unwrap();
        // Check if this is a comparison (result is felt) or arithmetic (result is u32)
        let is_comparison = matches!(
            typed_instr,
            CasmInstr::U32StoreEqFpImm { .. } | CasmInstr::U32StoreLtFpImm { .. }
        );

        // Check for problematic overlap between source and destination
        // For fp+imm operations, in-place operations (src_off == dst_off) are fine!
        // We only need to handle partial overlaps where the source and destination
        // overlap but are not identical.
        let has_overlap = if is_comparison {
            // For comparisons, dst is only 1 slot
            // Problematic if high word of source overlaps with dest
            src_off + 1 == dst_off
        } else {
            // For arithmetic, dst is 2 slots
            // In-place operation (src_off == dst_off) is fine
            // Problematic only if there's a partial overlap
            (src_off == dst_off + 1) || (src_off + 1 == dst_off)
        };

        if has_overlap {
            // Source overlaps with destination, copy source to temp
            Ok(vec![
                InstructionBuilder::from_instr(
                    CasmInstr::U32StoreAddFpImm {
                        src_off: M31::from(src_off),
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: M31::from(temp_var_offset),
                    },
                    Some(format!(
                        "u32([fp + {temp_var_offset}], [fp + {}]) = u32([fp + {src_off}], [fp + {}]) + u32(0, 0)",
                        temp_var_offset + 1,
                        src_off + 1
                    )),
                ),
                InstructionBuilder::from_instr(
                    u32_fp_imm_rebuild(
                        typed_instr,
                        M31::from(temp_var_offset),
                        M31::from(imm_lo),
                        M31::from(imm_hi),
                        M31::from(dst_off),
                    )?,
                    Some(if is_comparison {
                        format!(
                            "[fp + {dst_off}] = u32([fp + {temp_var_offset}], [fp + {}]) op u32({imm_lo}, {imm_hi})",
                            temp_var_offset + 1
                        )
                    } else {
                        format!(
                            "u32([fp + {dst_off}], [fp + {}]) = u32([fp + {temp_var_offset}], [fp + {}]) op u32({imm_lo}, {imm_hi})",
                            dst_off + 1,
                            temp_var_offset + 1
                        )
                    }),
                ),
            ])
        } else {
            // No overlap, keep as-is
            Ok(vec![instr.clone()])
        }
    }

    // ===== Casting Operations =====

    /// Generates code for type casting operations
    pub(crate) fn generate_cast(
        &mut self,
        dest: ValueId,
        source: Value,
        source_type: &MirType,
        target_type: &MirType,
    ) -> CodegenResult<()> {
        match (source_type, target_type) {
            (MirType::U32, MirType::Felt) => {
                // An M31 felt fits values in [0, P-1] with P = 2^31 - 1.
                // Let u32 = hi * 2^16 + lo with 16-bit limbs.
                // Rule:
                //  - If hi == (2^15 - 1), require lo != (2^16 - 1)
                //  - Else, require hi < (2^15 - 1)

                const U32_HI_BOUND_EXCLUSIVE: i32 = 2i32.pow(15); // 32768 = 2^15
                const U32_HI_BOUND_CHECK: i32 = U32_HI_BOUND_EXCLUSIVE - 1; // 32767 = 2^15 - 1
                const U16_MAX_PLUS_ONE: i32 = 2i32.pow(16); // 65536 = 2^16
                const U16_MAX: i32 = U16_MAX_PLUS_ONE - 1; // 65535 = 2^16 - 1

                let src_off = match source {
                    Value::Operand(id) => self.layout.get_offset(id)?,
                    _ => {
                        return Err(CodegenError::InvalidMir(
                            "Cast source must be an operand".to_string(),
                        ))
                    }
                };

                let dest_off = self.layout.allocate_local(dest, 1)?;

                // Compute hi < 32767 (fast path)
                let hi_lt_32767 = self.layout.reserve_stack(1);
                self.felt_lower_than_fp_imm(
                    src_off + 1,
                    U32_HI_BOUND_CHECK,
                    hi_lt_32767,
                    format!(
                        "[fp + {hi_lt_32767}] = [fp + {}] < {U32_HI_BOUND_CHECK} // hi < 2^15 - 1",
                        src_off + 1
                    ),
                );

                // If hi < 32767, we're good
                let ok_label = self.emit_new_label_name("u32_cast_ok");
                self.jnz_offset(hi_lt_32767, &ok_label);

                // Else: hi >= 32767. Valid only if hi == 32767 and lo < 65535.
                // Compute hi == 32767 using (hi < 32768) - (hi < 32767)
                let hi_lt_32768 = self.layout.reserve_stack(1);
                self.felt_lower_than_fp_imm(
                    src_off + 1,
                    U32_HI_BOUND_EXCLUSIVE,
                    hi_lt_32768,
                    format!(
                        "[fp + {hi_lt_32768}] = [fp + {}] < {U32_HI_BOUND_EXCLUSIVE} // hi < 2^15",
                        src_off + 1
                    ),
                );

                let hi_eq_32767 = self.layout.reserve_stack(1);
                self.felt_sub_fp_fp(hi_lt_32768, hi_lt_32767, hi_eq_32767, format!("[fp + {hi_eq_32767}] = [fp + {hi_lt_32768}] - [fp + {hi_lt_32767}] // hi == 32767"));

                // lo < 65535
                let lo_lt_65535 = self.layout.reserve_stack(1);
                self.felt_lower_than_fp_imm(
                    src_off,
                    U16_MAX,
                    lo_lt_65535,
                    format!(
                        "[fp + {lo_lt_65535}] = [fp + {}] < {U16_MAX} // lo < 2^16 - 1",
                        src_off
                    ),
                );

                // conj = (hi == 32767) * (lo < 65535)
                let conj = self.layout.reserve_stack(1);
                self.felt_mul_fp_fp(hi_eq_32767, lo_lt_65535, conj, format!("[fp + {conj}] = [fp + {hi_eq_32767}] * [fp + {lo_lt_65535}] // hi==32767 && lo<65535"));

                // Require conj == 1
                self.assert_eq_fp_imm(conj, 1, "assert(hi == 32767 && lo < 65535)".to_string());

                // Success path label
                self.emit_add_label(Label::new(ok_label));

                // Convert to felt: lo + hi * 2^16
                let temp_hi_shifted = self.layout.reserve_stack(1);
                self.felt_mul_fp_imm(
                    src_off + 1,
                    U16_MAX_PLUS_ONE,
                    temp_hi_shifted,
                    format!(
                        "[fp + {temp_hi_shifted}] = [fp + {}] * {U16_MAX_PLUS_ONE} // hi * 2^16",
                        src_off + 1
                    ),
                );
                self.felt_add_fp_fp(src_off, temp_hi_shifted, dest_off, format!("[fp + {dest_off}] = [fp + {src_off}] + [fp + {temp_hi_shifted}] // Cast u32->felt"));

                Ok(())
            }
            _ => Err(CodegenError::UnsupportedInstruction(format!(
                "Unsupported cast from {} to {}",
                source_type, target_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cairo_m_compiler_mir::instruction::CalleeSignature;
    use cairo_m_compiler_mir::MirType;

    #[test]
    fn test_handle_fp_fp_duplicates_all_same() {
        // Create instruction: [fp + 5] = [fp + 5] + [fp + 5]
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.felt_add_fp_fp(5, 5, 5, "test".to_string());

        let temp1 = 10;
        let temp2 = 11;
        let result =
            CasmBuilder::handle_fp_fp_duplicates(&builder.instructions[0], temp1, temp2).unwrap();
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
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.felt_add_fp_fp(5, 3, 5, "test".to_string());

        let temp1 = 10;
        let temp2 = 11;
        let result =
            CasmBuilder::handle_fp_fp_duplicates(&builder.instructions[0], temp1, temp2).unwrap();
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that first operand is copied to temp
        assert_eq!(result[0].op2(), Some(temp1));
        assert_eq!(result[0].op0(), Some(5));
        assert_eq!(result[1].op0(), Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_in_place() {
        // Create instruction: [fp + 5] = [fp + 5] + 42
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.felt_add_fp_imm(5, 42, 5, "test".to_string());

        let temp1 = 10;
        let result =
            CasmBuilder::handle_fp_imm_duplicates(&builder.instructions[0], temp1).unwrap();
        assert_eq!(result.len(), 2, "Should expand to 2 instructions");

        // Check that source is copied to temp first
        assert_eq!(result[0].op2(), Some(temp1));
        assert_eq!(result[0].op0(), Some(5));
        assert_eq!(result[1].op0(), Some(temp1));
    }

    #[test]
    fn test_handle_fp_imm_duplicates_no_conflict() {
        // Create instruction: [fp + 7] = [fp + 5] + 42 (no conflict)
        let mut builder = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        builder.felt_add_fp_imm(5, 42, 7, "test".to_string());

        let temp1 = 10;
        let result =
            CasmBuilder::handle_fp_imm_duplicates(&builder.instructions[0], temp1).unwrap();
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
