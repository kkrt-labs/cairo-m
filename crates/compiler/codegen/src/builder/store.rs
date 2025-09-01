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
    }

    /// Store a felt/boolean/pointer immediate and track the write.
    pub(super) fn store_immediate(&mut self, value: u32, offset: i32, comment: String) {
        let instr = InstructionBuilder::new(STORE_IMM)
            .with_operand(Operand::Literal(value as i32))
            .with_operand(Operand::Literal(offset))
            .with_comment(comment);
        self.emit_push(instr);
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
        src_off: i32,
        base_off: i32,
        imm: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_IMM)
            .with_operand(Operand::Literal(src_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(base_off))
            .with_comment(comment);
        self.emit_push(instr);
    }

    pub(crate) fn store_to_double_deref_fp_fp(
        &mut self,
        base_off: i32,
        imm: i32,
        src_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_TO_DOUBLE_DEREF_FP_FP)
            .with_operand(Operand::Literal(src_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(base_off))
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
    use crate::test_support::{exec, Mem};
    use cairo_m_common::instruction::{
        STORE_ADD_FP_IMM, STORE_DOUBLE_DEREF_FP, STORE_DOUBLE_DEREF_FP_FP, STORE_FP_IMM, STORE_IMM,
        STORE_TO_DOUBLE_DEREF_FP_FP, STORE_TO_DOUBLE_DEREF_FP_IMM, U32_STORE_ADD_FP_IMM,
        U32_STORE_IMM,
    };
    use cairo_m_compiler_mir::{Literal, Value, ValueId};
    use proptest::prelude::*;
    use proptest::strategy::{Just, Strategy};
    use stwo_prover::core::fields::m31::M31;

    // -------------------------
    // Test setup helpers
    // -------------------------

    fn mk_builder_with_value() -> (CasmBuilder, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        layout.allocate_value(a, 1).unwrap();
        (CasmBuilder::new(layout, 0), a)
    }

    // -------------------------
    // Basic store operation tests
    // -------------------------

    #[test]
    fn test_store_fp_plus_imm() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.store_fp_plus_imm(3, 7, "[fp + 7] = fp + 3".into());

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_FP_IMM);
        assert_eq!(b.instructions[0].op0(), Some(3));
        assert_eq!(b.instructions[0].op1(), Some(7));
    }

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
    fn test_copy_slots_zero_size() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);
        b.copy_slots(10, 20, 0, "Empty:");
        assert_eq!(
            b.instructions.len(),
            0,
            "Zero-size copy should generate no instructions"
        );
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

        // Test execution
        let mut mem = Mem::new(64);
        mem.set_u32(5, 0xDEAD_BEEF);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get_u32(12), 0xDEAD_BEEF);
    }

    #[test]
    fn test_store_from_double_deref_fp_imm() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.store_from_double_deref_fp_imm(2, 5, 8, "[[fp + 2] + 5] -> [fp + 8]".into());

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_DOUBLE_DEREF_FP);
        assert_eq!(b.instructions[0].op0(), Some(2));
        assert_eq!(b.instructions[0].op1(), Some(5));
        assert_eq!(b.instructions[0].op2(), Some(8));
    }

    #[test]
    fn test_store_from_double_deref_fp_fp() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.store_from_double_deref_fp_fp(2, 3, 8, "[[fp + 2] + [fp + 3]] -> [fp + 8]".into());

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_DOUBLE_DEREF_FP_FP);
        assert_eq!(b.instructions[0].op0(), Some(2));
        assert_eq!(b.instructions[0].op1(), Some(3));
        assert_eq!(b.instructions[0].op2(), Some(8));
    }

    #[test]
    fn test_store_to_double_deref_fp_imm() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.store_to_double_deref_fp_imm(8, 2, 5, "[fp + 8] -> [[fp + 2] + 5]".into());

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_TO_DOUBLE_DEREF_FP_IMM);
        assert_eq!(b.instructions[0].op0(), Some(8));
        assert_eq!(b.instructions[0].op1(), Some(5));
        assert_eq!(b.instructions[0].op2(), Some(2));
    }

    #[test]
    fn test_store_to_double_deref_fp_fp() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.store_to_double_deref_fp_fp(2, 3, 8, "[fp + 8] -> [[fp + 2] + [fp + 3]]".into());

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_TO_DOUBLE_DEREF_FP_FP);
        assert_eq!(b.instructions[0].op0(), Some(8));
        assert_eq!(b.instructions[0].op1(), Some(3));
        assert_eq!(b.instructions[0].op2(), Some(2));
    }

    // -------------------------
    // Copy value to offset tests
    // -------------------------

    #[test]
    fn test_copy_value_to_offset_literal_felt() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        let value = Value::Literal(Literal::Integer(42));
        b.copy_value_to_offset(&value, 5, 1).unwrap();

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_IMM);
        assert_eq!(b.instructions[0].op0(), Some(42));
        assert_eq!(b.instructions[0].op1(), Some(5));
    }

    #[test]
    fn test_copy_value_to_offset_literal_u32() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        let value = Value::Literal(Literal::Integer(0xABCD_1234));
        b.copy_value_to_offset(&value, 10, 2).unwrap();

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, U32_STORE_IMM);
        assert_eq!(b.instructions[0].op0(), Some(0x1234)); // Low
        assert_eq!(b.instructions[0].op1(), Some(0xABCD)); // High
        assert_eq!(b.instructions[0].op2(), Some(10));
    }

    #[test]
    fn test_copy_value_to_offset_boolean() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);

        // Test true
        b.copy_value_to_offset(&Value::Literal(Literal::Boolean(true)), 3, 1)
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_IMM);
        assert_eq!(b.instructions[0].op0(), Some(1));

        // Test false
        b.instructions.clear();
        b.copy_value_to_offset(&Value::Literal(Literal::Boolean(false)), 4, 1)
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_IMM);
        assert_eq!(b.instructions[0].op0(), Some(0));
    }

    #[test]
    fn test_copy_value_to_offset_unit() {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        let value = Value::Literal(Literal::Unit);
        b.copy_value_to_offset(&value, 5, 0).unwrap();

        // Unit type has size 0, should generate no instructions
        assert_eq!(b.instructions.len(), 0);
    }

    #[test]
    fn test_copy_value_to_offset_operand() {
        let (mut b, a) = mk_builder_with_value();
        let value = Value::Operand(a);
        b.copy_value_to_offset(&value, 10, 1).unwrap();

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, STORE_ADD_FP_IMM);
        assert_eq!(b.instructions[0].op0(), Some(0)); // Source offset
        assert_eq!(b.instructions[0].op1(), Some(0)); // Add 0
        assert_eq!(b.instructions[0].op2(), Some(10)); // Dest offset
    }

    /// Strategy for various immediate values to test
    fn immediate_value_strategy() -> impl Strategy<Value = u32> {
        prop_oneof![
            Just(0u32),     // Zero
            Just(1u32),     // One
            Just(u32::MAX), // Maximum
            Just(0xFFFF),   // 16-bit boundary
            Just(0x10000),  // Just over 16-bit
            any::<u32>(),   // Random values
        ]
    }

    proptest! {
        #[test]
        fn property_store_immediate_roundtrip(
            value in immediate_value_strategy(),
            offset in 0i32..50,
        ) {
            let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
            b.store_immediate(value, offset, format!("[fp + {offset}] = {value}"));

            let mut mem = Mem::new(64);
            exec(&mut mem, &b.instructions).unwrap();

            let stored = mem.get(offset).0;
            // Values stored are M31
            let expected = M31::from(value).0;
            prop_assert_eq!(stored, expected, "Value {} at offset {}", value, offset);
        }

        #[test]
        fn property_store_u32_immediate_roundtrip(
            value in any::<u32>(),
            offset in 0i32..50,
        ) {
            let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
            b.store_u32_immediate(value, offset, format!("u32 at {offset}"));

            let mut mem = Mem::new(64);
            exec(&mut mem, &b.instructions).unwrap();

            let stored = mem.get_u32(offset);
            prop_assert_eq!(stored, value, "U32 value {} at offset {}", value, offset);
        }

        #[test]
        fn property_copy_slots_preserves_values(
            src_offset in 0i32..20,
            dst_offset in 32i32..54,
            num_slots in 0usize..10,
        ) {
            let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
            let mut mem = Mem::new(64);

            // Initialize source values
            let values: Vec<u32> = (0..num_slots).map(|i| (i as u32 + 100) * 11).collect();
            for (i, &val) in values.iter().enumerate() {
                mem.set(src_offset + i as i32, M31::from(val));
            }

            // Copy slots
            b.copy_slots(src_offset, dst_offset, num_slots, "Copy:");
            exec(&mut mem, &b.instructions).unwrap();

            // Verify all values copied correctly
            for (i, &expected) in values.iter().enumerate() {
                let actual = mem.get(dst_offset + i as i32).0;
                prop_assert_eq!(actual, M31::from(expected).0,
                    "Slot {} mismatch", i);
            }
        }
    }
}
