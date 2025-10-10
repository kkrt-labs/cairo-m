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

use cairo_m_compiler_mir::{BinaryOp, DataLayout, Literal, MirType, Value, ValueId};
use cairo_m_compiler_parser::parser::UnaryOp;
use stwo_prover::core::fields::m31::M31;

use crate::{CodegenError, CodegenResult, FunctionLayout, InstructionBuilder, Label};

// Centralized emission helpers for instruction/label/touch routing.
mod aggregates;
mod asserts;
pub(crate) mod calls;
mod ctrlflow;
mod emit;
mod felt;
pub(crate) mod normalize;
mod store;
mod u32_ops;

/// Helper to split a u32 value into low and high 16-bit parts
#[inline]
pub(super) const fn split_u32_value(value: u32) -> (i32, i32) {
    ((value & 0xFFFF) as i32, ((value >> 16) & 0xFFFF) as i32)
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
    pub layout: FunctionLayout,
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
    pub fn assign(
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
    pub fn unary_op(
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

    /// Load a value from memory through a pointer
    ///
    /// Loads slots from [[ptr_base] + 0..size] into dest. Size is inferred from dest's layout.
    pub fn load_from_memory(
        &mut self,
        dest: ValueId,
        ptr_base: ValueId,
        size: usize,
    ) -> CodegenResult<()> {
        let base_off = self.layout.get_offset(ptr_base)?;
        let dest_off = self.layout.allocate_local(dest, size)?;

        for slot in 0..size {
            self.store_from_double_deref_fp_imm(
                base_off,
                slot as i32,
                dest_off + slot as i32,
                format!(
                    "[fp + {}] = [[fp + {}] + {}] (load limb {}/{})",
                    dest_off + slot as i32,
                    base_off,
                    slot,
                    slot + 1,
                    size
                ),
            );
        }

        Ok(())
    }

    /// Store a value to memory through a pointer
    ///
    /// Stores slots to [[ptr_base] + 0..size] from value. Size is inferred from value's layout.
    pub fn store_to_memory(&mut self, ptr_base: ValueId, value: ValueId) -> CodegenResult<()> {
        let base_off = self.layout.get_offset(ptr_base)?;
        let value_off = self.layout.get_offset(value)?;

        // Get size from value's layout
        let size = self.layout.get_value_size(value);

        for slot in 0..size {
            self.store_to_double_deref_fp_imm(
                value_off + slot as i32,
                base_off,
                slot as i32,
                format!(
                    "[[fp + {}] + {}] = [fp + {}] (store limb {}/{})",
                    base_off,
                    slot,
                    value_off + slot as i32,
                    slot + 1,
                    size
                ),
            );
        }

        Ok(())
    }

    /// Generate a binary operation instruction
    ///
    /// If target_offset is provided, writes directly to that location.
    /// Otherwise, allocates a new local variable.
    pub fn binary_op(
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
            | BinaryOp::U32Rem
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
            | BinaryOp::U32Rem
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

    /// Get a mutable view of the generated instructions (for post passes)
    pub(crate) const fn instructions_mut(&mut self) -> &mut Vec<InstructionBuilder> {
        &mut self.instructions
    }

    #[cfg(test)]
    /// Get the labels
    pub(crate) fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Get a mutable reference to the labels (for post passes adjusting addresses)
    pub(crate) const fn labels_mut(&mut self) -> &mut Vec<Label> {
        &mut self.labels
    }

    /// Get a mutable reference to the layout
    pub const fn layout_mut(&mut self) -> &mut FunctionLayout {
        &mut self.layout
    }

    /// Get the label counter
    pub const fn label_counter(&self) -> usize {
        self.label_counter
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

                let dest_off = self.layout.allocate_local(dest, 1)?;
                let src_off = match source {
                    Value::Operand(id) => self.layout.get_offset(id)?,
                    Value::Literal(Literal::Integer(imm)) => {
                        let m31_imm = M31::from(imm);
                        if m31_imm.0 == imm {
                            self.store_immediate(
                                imm,
                                dest_off,
                                format!("[fp + {dest_off}] = {imm}"),
                            );
                            return Ok(());
                        } else {
                            return Err(CodegenError::InvalidMir(
                                "Cast source is a literal that does not fit in an M31".to_string(),
                            ));
                        }
                    }
                    _ => {
                        return Err(CodegenError::InvalidMir(
                            "Cast source must be an operand".to_string(),
                        ))
                    }
                };

                // Compute hi < 32767 (fast path)
                let hi_lt_32767 = self.layout.reserve_stack(1);
                let imm = U32_HI_BOUND_CHECK - 1;
                self.felt_le_fp_imm(
                    src_off + 1,
                    imm,
                    hi_lt_32767,
                    format!(
                        "[fp + {hi_lt_32767}] = [fp + {}] <= {imm} // hi < 2^15 - 1",
                        src_off + 1
                    ),
                );

                // If hi < 32767, we're good
                let ok_label = self.emit_new_label_name("u32_cast_ok");
                self.jnz_offset(hi_lt_32767, &ok_label);

                // Else: hi >= 32767. Valid only if hi == 32767 and lo < 65535.
                // Compute hi == 32767 using (hi < 32768) - (hi < 32767)
                let hi_lt_32768 = self.layout.reserve_stack(1);
                let imm = U32_HI_BOUND_EXCLUSIVE - 1;
                self.felt_le_fp_imm(
                    src_off + 1,
                    imm,
                    hi_lt_32768,
                    format!(
                        "[fp + {hi_lt_32768}] = [fp + {}] <= {imm} // hi < 2^15",
                        src_off + 1
                    ),
                );

                let hi_eq_32767 = self.layout.reserve_stack(1);
                self.felt_sub_fp_fp(hi_lt_32768, hi_lt_32767, hi_eq_32767, format!("[fp + {hi_eq_32767}] = [fp + {hi_lt_32768}] - [fp + {hi_lt_32767}] // hi == 32767"));

                // lo < 65535
                let lo_lt_65535 = self.layout.reserve_stack(1);
                let imm = U16_MAX - 1;
                self.felt_le_fp_imm(
                    src_off,
                    imm,
                    lo_lt_65535,
                    format!(
                        "[fp + {lo_lt_65535}] = [fp + {}] <= {imm} // lo < 2^16 - 1",
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
