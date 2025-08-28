//! Store/copy helpers for felt, u32, and aggregates.
//!
//! Centralizes STORE opcodes: immediates, single-slot and multi-slot copies, and u32 copies.

use crate::{CodegenError, CodegenResult, InstructionBuilder, Operand};
use cairo_m_common::instruction::{
    STORE_DOUBLE_DEREF_FP, STORE_DOUBLE_DEREF_FP_FP, STORE_FP_IMM, STORE_IMM,
    STORE_TO_DOUBLE_DEREF_FP_FP, STORE_TO_DOUBLE_DEREF_FP_IMM, U32_STORE_ADD_FP_IMM, U32_STORE_IMM,
};
use cairo_m_compiler_mir::{Literal, Value};

impl super::CasmBuilder {
    /// Store copy of an M31 from src to dest with an exact comment string.
    pub(crate) fn store_copy_single(&mut self, src_off: i32, dest_off: i32, comment: String) {
        self.felt_add_fp_imm(src_off, 0, dest_off, comment);
    }

    /// Store `fp + base_off` into `[fp + dest_off]` with provided comment.
    pub(super) fn store_fp_plus_imm(&mut self, base_off: i32, dest_off: i32, comment: String) {
        let instr = InstructionBuilder::new(STORE_FP_IMM)
            .with_operand(Operand::Literal(base_off))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dest_off, 1);
    }

    /// Load from memory: `[[fp + base_off] + imm] -> [fp + dest_off]` (slot-sized), with comment.
    pub(super) fn store_from_double_deref_fp_imm(
        &mut self,
        base_off: i32,
        imm: i32,
        dest_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP)
            .with_operand(Operand::Literal(base_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dest_off, 1);
    }

    /// Load from memory: `[[fp + base_off] + [fp + idx_off]] -> [fp + dest_off]` (slot-sized), with comment.
    pub(super) fn store_from_double_deref_fp_fp(
        &mut self,
        base_off: i32,
        idx_off: i32,
        dest_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_DOUBLE_DEREF_FP_FP)
            .with_operand(Operand::Literal(base_off))
            .with_operand(Operand::Literal(idx_off))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dest_off, 1);
    }
    /// Copy a u32 value (2 slots) from `src_off` to `dest_off` using the dedicated opcode.
    pub(super) fn store_copy_u32(&mut self, src_off: i32, dest_off: i32, comment_prefix: &str) {
        let instr = InstructionBuilder::new(U32_STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(src_off))
            .with_operand(Operand::Literal(0))
            .with_operand(Operand::Literal(0))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(format!(
                "{comment_prefix}u32([fp + {dest_off}], [fp + {}]) = u32([fp + {src_off}], [fp + {}]) + u32(0, 0)",
                dest_off + 1,
                src_off + 1
            ));
        self.emit_push(instr);
        self.emit_touch(dest_off, 2);
    }

    /// Store a felt/boolean/pointer immediate and track the write.
    pub(super) fn store_immediate(&mut self, value: u32, offset: i32, comment: String) {
        let instr = InstructionBuilder::new(STORE_IMM)
            .with_operand(Operand::Literal(value as i32))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(offset, 1);
    }

    /// Store a u32 immediate split into two slots and track the write.
    pub(super) fn store_u32_immediate(&mut self, value: u32, offset: i32, comment: String) {
        let (lo, hi) = super::split_u32_value(value);
        let instr = InstructionBuilder::new(U32_STORE_IMM)
            .with_operand(Operand::Literal(lo))
            .with_operand(Operand::Literal(hi))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(offset, 2);
    }

    /// Copy `slots` consecutive words from `src_off` to `dest_off`.
    pub(super) fn copy_slots(
        &mut self,
        src_off: i32,
        dest_off: i32,
        slots: usize,
        comment_prefix: &str,
    ) {
        if slots == 0 {
            return;
        }
        for i in 0..slots {
            let s = src_off + i as i32;
            let d = dest_off + i as i32;
            self.store_copy_single(
                s,
                d,
                format!("{comment_prefix} slot {i}: [fp + {d}] = [fp + {s}] + 0"),
            );
        }
    }

    pub(crate) fn store_to_double_deref_fp_imm(
        &mut self,
        base_off: i32,
        imm: i32,
        dest_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
            .with_operand(Operand::Literal(base_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(comment);
        self.emit_push(instr);
    }

    pub(crate) fn store_to_double_deref_fp_fp(
        &mut self,
        base_off: i32,
        imm: i32,
        dest_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
            .with_operand(Operand::Literal(base_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dest_off))
            .with_comment(comment);
        self.emit_push(instr);
    }

    /// Helper method to copy a value to a specific offset
    pub(crate) fn copy_value_to_offset(
        &mut self,
        value: &Value,
        target_offset: i32,
        size: usize,
    ) -> CodegenResult<()> {
        // Nothing to copy for zero-sized values (e.g., unit)
        if size == 0 {
            return Ok(());
        }
        match value {
            Value::Literal(Literal::Integer(imm)) => {
                if size == 1 {
                    self.store_immediate(
                        *imm,
                        target_offset,
                        format!("[fp + {}] = {}", target_offset, imm),
                    );
                } else if size == 2 {
                    // Handle u32 literal
                    self.store_u32_immediate(
                        *imm,
                        target_offset,
                        format!(
                            "[fp + {}], [fp + {}] = u32({})",
                            target_offset,
                            target_offset + 1,
                            imm
                        ),
                    );
                } else {
                    return Err(CodegenError::UnsupportedInstruction(format!(
                        "Unsupported literal size: {}",
                        size
                    )));
                }
            }
            Value::Literal(Literal::Boolean(b)) => {
                // Booleans are single-slot values storing 0/1
                let imm = if *b { 1 } else { 0 };
                self.store_immediate(
                    imm,
                    target_offset,
                    format!("[fp + {}] = {}", target_offset, imm),
                );
            }
            Value::Literal(Literal::Unit) => {
                // Unit has size 0 by layout; nothing to store
            }
            Value::Operand(src_id) => {
                let src_offset = self.layout.get_offset(*src_id)?;
                // Copy each slot using the single-slot helper to keep exact comment format
                for i in 0..size {
                    let slot_src = src_offset + i as i32;
                    let slot_dst = target_offset + i as i32;
                    self.store_copy_single(
                        slot_src,
                        slot_dst,
                        format!("[fp + {}] = [fp + {}] + 0", slot_dst, slot_src),
                    );
                }
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(format!(
                    "Unsupported value type in aggregate: {:?}",
                    value
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::CasmBuilder;
    use crate::layout::FunctionLayout;
    use cairo_m_common::instruction::{
        STORE_ADD_FP_IMM, STORE_IMM, U32_STORE_ADD_FP_IMM, U32_STORE_IMM,
    };

    #[test]
    fn test_copy_slots_single() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        b.copy_slots(3, 7, 1, "T:");
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, STORE_ADD_FP_IMM);
        assert_eq!(i.op0(), Some(3));
        assert_eq!(i.op1(), Some(0));
        assert_eq!(i.op2(), Some(7));
    }

    #[test]
    fn test_copy_slots_multi() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        b.copy_slots(10, 20, 3, "C:");
        assert_eq!(b.instructions.len(), 3);
        for k in 0..3 {
            let i = &b.instructions[k];
            assert_eq!(i.opcode, STORE_ADD_FP_IMM);
            assert_eq!(i.op0(), Some(10 + k as i32));
            assert_eq!(i.op1(), Some(0));
            assert_eq!(i.op2(), Some(20 + k as i32));
        }
    }

    #[test]
    fn test_store_copy_u32() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        b.store_copy_u32(5, 12, "U32:");
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, U32_STORE_ADD_FP_IMM);
        assert_eq!(i.op0(), Some(5));
        // imm lo/hi zeros
        assert_eq!(i.op1(), Some(0));
        assert_eq!(i.op2(), Some(0));
        // dest offset
        assert_eq!(
            i.operands
                .get(3)
                .and_then(|o| if let Operand::Literal(v) = o {
                    Some(*v)
                } else {
                    None
                }),
            Some(12)
        );
    }

    #[test]
    fn test_store_immediates() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        b.store_immediate(7, 3, "X".into());
        b.store_u32_immediate(0x0102_0304, 10, "Y".into());
        assert_eq!(b.instructions[0].opcode, STORE_IMM);
        assert_eq!(b.instructions[0].op0(), Some(7));
        assert_eq!(b.instructions[0].op1(), Some(3));
        assert_eq!(b.instructions[1].opcode, U32_STORE_IMM);
        assert_eq!(b.instructions[1].op0(), Some(0x0304));
        assert_eq!(b.instructions[1].op1(), Some(0x0102));
        assert_eq!(b.instructions[1].op2(), Some(10));
    }
}
