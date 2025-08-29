//! u32 operations: arithmetic, comparisons, bitwise, immediate transforms.

use super::normalize::{normalize_u32_cmp_fp_fp, normalize_u32_cmp_fp_imm};
use super::opcodes::{u32_fp_fp, u32_fp_imm};
use crate::{CodegenError, CodegenResult, InstructionBuilder, Operand};
use cairo_m_compiler_mir::{BinaryOp, Literal, Value};

impl super::CasmBuilder {
    pub(super) fn u32_op(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        let is_cmp = matches!(
            op,
            BinaryOp::U32Eq
                | BinaryOp::U32Neq
                | BinaryOp::U32Greater
                | BinaryOp::U32GreaterEqual
                | BinaryOp::U32Less
                | BinaryOp::U32LessEqual
        );
        let result_size = if is_cmp { 1 } else { 2 };

        // Normalize commutative immediate-left cases to immediate-right
        let (left, right) = super::normalize::canonicalize_commutative_u32(op, left, right);

        match (&left, &right) {
            (Value::Operand(lid), Value::Operand(rid)) => {
                let mut lo = self.layout.get_offset(*lid)?;
                let mut ro = self.layout.get_offset(*rid)?;

                let norm = normalize_u32_cmp_fp_fp(op);
                if norm.swap {
                    std::mem::swap(&mut lo, &mut ro);
                }

                let is_cmp_op = matches!(
                    norm.op,
                    BinaryOp::U32Eq
                        | BinaryOp::U32Neq
                        | BinaryOp::U32Greater
                        | BinaryOp::U32GreaterEqual
                        | BinaryOp::U32Less
                        | BinaryOp::U32LessEqual
                );
                let comment = if is_cmp_op {
                    format!(
                        "[fp + {dest_off}] = u32([fp + {lo}], [fp + {}]) {op} u32([fp + {ro}], [fp + {}])",
                        lo + 1,
                        ro + 1,
                    )
                } else {
                    format!(
                        "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {lo}], [fp + {}]) {op} u32([fp + {ro}], [fp + {}])",
                        dest_off + 1,
                        lo + 1,
                        ro + 1,
                    )
                };
                let instr = InstructionBuilder::new(u32_fp_fp(norm.op)?)
                    .with_operand(Operand::Literal(lo))
                    .with_operand(Operand::Literal(ro))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(comment);
                self.emit_push(instr);
                self.emit_touch(dest_off, result_size);

                if norm.complement {
                    self.complement_felt_in_place(dest_off);
                }
            }

            (Value::Operand(lid), Value::Literal(Literal::Integer(imm))) => {
                let lo = self.layout.get_offset(*lid)?;

                let orig_imm = { *imm };

                // Handle easy boundaries first
                if matches!(op, BinaryOp::U32Greater) && orig_imm == 0xFFFF_FFFF {
                    self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
                    return Ok(());
                }
                if matches!(op, BinaryOp::U32LessEqual) && orig_imm == 0xFFFF_FFFF {
                    self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
                    return Ok(());
                }

                // Select immediate to encode and any complement note
                let mut complement_after = false;
                let mut extra_note: Option<String> = None;

                // For comparisons, normalize to canonical op and possibly biased imm
                let norm = normalize_u32_cmp_fp_imm(op, orig_imm);

                // For arithmetic sub with immediate we encode as add with two's complement
                let encoded_imm = match op {
                    BinaryOp::U32Sub => {
                        let neg = super::twos_complement_u32(orig_imm);
                        extra_note = Some(format!(
                            " (two's complement of {orig} -> {neg})",
                            orig = orig_imm,
                            neg = neg
                        ));
                        neg
                    }
                    _ => norm.biased_imm.unwrap_or(orig_imm),
                };

                // Track complement requirement from normalization
                if norm.complement {
                    complement_after = true;
                }

                let (low, high) = super::split_u32_value(encoded_imm);

                let is_cmp_op = matches!(
                    norm.op,
                    BinaryOp::U32Eq
                        | BinaryOp::U32Neq
                        | BinaryOp::U32Greater
                        | BinaryOp::U32GreaterEqual
                        | BinaryOp::U32Less
                        | BinaryOp::U32LessEqual
                );

                // Show the encoded immediate in hex (matches actual operands)
                let imm_hex = format!("{:#010x}", encoded_imm);
                // Append legacy-style bias/complement notes for select ops
                if extra_note.is_none() {
                    extra_note = match op {
                        BinaryOp::U32Greater => {
                            Some(format!(" (biased c' = {:#010x}; gt = 1 - lt)", encoded_imm))
                        }
                        BinaryOp::U32LessEqual => {
                            // le is compiled as lt with c' = c + 1, but keep comment minimal
                            Some(format!(" (biased c' = {:#010x})", encoded_imm))
                        }
                        _ => None,
                    };
                }

                let base = if is_cmp_op {
                    format!(
                        "[fp + {dest_off}] = u32([fp + {lo}], [fp + {}]) {op} u32({low}, {high})",
                        lo + 1
                    )
                } else {
                    format!(
                        "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {lo}], [fp + {}]) {op} u32({low}, {high})",
                        dest_off + 1,
                        lo + 1
                    )
                };
                let comment = format!(
                    "{base} /* imm = {imm_hex} */{}",
                    extra_note.unwrap_or_default()
                );

                let instr = InstructionBuilder::new(u32_fp_imm(norm.op)?)
                    .with_operand(Operand::Literal(lo))
                    .with_operand(Operand::Literal(low))
                    .with_operand(Operand::Literal(high))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(comment);
                self.emit_push(instr);
                self.emit_touch(dest_off, result_size);

                if complement_after {
                    self.complement_felt_in_place(dest_off);
                }
            }

            (Value::Literal(Literal::Integer(imm)), Value::Operand(rid)) => {
                let ro = self.layout.get_offset(*rid)?;
                match op {
                    BinaryOp::U32Add | BinaryOp::U32Mul => {
                        let (lo, hi) = super::split_u32_i32(*imm as i32);
                        let instr = InstructionBuilder::new(u32_fp_imm(op)?)
                            .with_operand(Operand::Literal(ro))
                            .with_operand(Operand::Literal(lo))
                            .with_operand(Operand::Literal(hi))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!("u32([fp + {dest_off}], [fp + {}]) = u32([fp + {ro}]) {op} u32({lo}, {hi})", dest_off + 1));
                        self.emit_push(instr);
                        self.emit_touch(dest_off, result_size);
                    }
                    BinaryOp::U32Sub | BinaryOp::U32Div => {
                        let tmp = self.layout.reserve_stack(2);
                        self.store_u32_immediate(
                            *imm,
                            tmp,
                            format!("[fp + {}], [fp + {}] = u32({imm})", tmp, tmp + 1),
                        );
                        let instr = InstructionBuilder::new(u32_fp_fp(op)?)
                            .with_operand(Operand::Literal(tmp))
                            .with_operand(Operand::Literal(ro))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!("u32([fp + {dest_off}], [fp + {}]) = u32([fp + {tmp}]) {op} u32([fp + {ro}])", dest_off + 1));
                        self.emit_push(instr);
                        self.emit_touch(dest_off, result_size);
                    }
                    BinaryOp::U32Eq
                    | BinaryOp::U32Neq
                    | BinaryOp::U32Greater
                    | BinaryOp::U32GreaterEqual
                    | BinaryOp::U32Less
                    | BinaryOp::U32LessEqual
                    | BinaryOp::U32BitwiseAnd
                    | BinaryOp::U32BitwiseOr
                    | BinaryOp::U32BitwiseXor => {
                        let tmp = self.layout.reserve_stack(2);
                        self.store_u32_immediate(
                            *imm,
                            tmp,
                            format!("[fp + {}], [fp + {}] = u32({imm})", tmp, tmp + 1),
                        );
                        let is_cmp = matches!(
                            op,
                            BinaryOp::U32Eq
                                | BinaryOp::U32Neq
                                | BinaryOp::U32Greater
                                | BinaryOp::U32GreaterEqual
                                | BinaryOp::U32Less
                                | BinaryOp::U32LessEqual
                        );
                        let instr = InstructionBuilder::new(u32_fp_fp(op)?)
                            .with_operand(Operand::Literal(tmp))
                            .with_operand(Operand::Literal(ro))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!("u32([fp + {dest_off}], [fp + {}]) = u32([fp + {tmp}]) {op} u32([fp + {ro}])", dest_off + 1));
                        self.emit_push(instr);
                        self.emit_touch(dest_off, result_size);
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported op: {op}"
                        )))
                    }
                }
            }

            (Value::Literal(Literal::Integer(li)), Value::Literal(Literal::Integer(ri))) => {
                // Constant fold arith and comparisons for u32
                let l = { *li };
                let r = { *ri };
                let res_u32 = match op {
                    BinaryOp::U32Add => l.wrapping_add(r),
                    BinaryOp::U32Sub => l.wrapping_sub(r),
                    BinaryOp::U32Mul => l.wrapping_mul(r),
                    BinaryOp::U32Div => {
                        if r == 0 {
                            return Err(CodegenError::InvalidMir("Division by zero".into()));
                        }
                        l.wrapping_div(r)
                    }
                    _ => 0,
                };
                if is_cmp {
                    let res = match op {
                        BinaryOp::U32Eq => (l == r) as u32,
                        BinaryOp::U32Neq => (l != r) as u32,
                        BinaryOp::U32Greater => (l > r) as u32,
                        BinaryOp::U32GreaterEqual => (l >= r) as u32,
                        BinaryOp::U32Less => (l < r) as u32,
                        BinaryOp::U32LessEqual => (l <= r) as u32,
                        _ => unreachable!(),
                    };
                    self.store_immediate(res, dest_off, format!("[fp + {dest_off}] = {res}"));
                    self.emit_touch(dest_off, 1);
                } else {
                    self.store_u32_immediate(
                        res_u32,
                        dest_off,
                        format!(
                            "u32([fp + {dest_off}], [fp + {}]) = {res_u32}",
                            dest_off + 1
                        ),
                    );
                }
            }

            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported u32 operands".into(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::instruction::{
        STORE_SUB_FP_FP, U32_STORE_ADD_FP_FP, U32_STORE_ADD_FP_IMM, U32_STORE_EQ_FP_IMM,
        U32_STORE_LT_FP_IMM,
    };
    use cairo_m_compiler_mir::{BinaryOp, Value, ValueId};

    fn mk_builder_two_ops() -> (CasmBuilder, ValueId, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        let b = ValueId::from_raw(2);
        layout.allocate_value(a, 2).unwrap();
        layout.allocate_value(b, 2).unwrap();
        (CasmBuilder::new(layout, 0), a, b)
    }

    #[test]
    fn test_u32_add_fp_fp() {
        let (mut b, a, c) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Add, 8, Value::operand(a), Value::operand(c))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, U32_STORE_ADD_FP_FP);
        assert_eq!(b.instructions[0].op0(), Some(0));
        assert_eq!(b.instructions[0].op1(), Some(2));
        assert_eq!(b.instructions[0].op2(), Some(8));
    }

    #[test]
    fn test_u32_sub_fp_imm_uses_add_with_twos_complement() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Sub, 5, Value::operand(a), Value::integer(7))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, U32_STORE_ADD_FP_IMM);
        assert_eq!(b.instructions[0].op0(), Some(0));
        // op1/op2 contain lo/hi parts of two's complement, non-zero expected
        assert!(b.instructions[0].op1().unwrap() != 0 || b.instructions[0].op2().unwrap() != 0);
        // dest off
        let last = &b.instructions[0].operands[3];
        if let Operand::Literal(v) = last {
            assert_eq!(*v, 5);
        } else {
            panic!()
        }
    }

    #[test]
    fn test_u32_eq_fp_imm() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Eq, 3, Value::operand(a), Value::integer(42))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, U32_STORE_EQ_FP_IMM);
        assert_eq!(b.instructions[0].op0(), Some(0));
        assert!(b.instructions[0].op2().is_some()); // hi part present
                                                    // dest
        let last = &b.instructions[0].operands[3];
        if let Operand::Literal(v) = last {
            assert_eq!(*v, 3);
        } else {
            panic!()
        }
    }

    #[test]
    fn test_u32_greater_fp_imm_complements() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(
            BinaryOp::U32Greater,
            4,
            Value::operand(a),
            Value::integer(10),
        )
        .unwrap();
        // Should emit compare (lt with bias) + complement (STORE_IMM, copy, STORE_SUB_FP_FP)
        assert_eq!(b.instructions.len(), 4);
        assert_eq!(b.instructions[0].opcode, U32_STORE_LT_FP_IMM);
        assert_eq!(b.instructions[1].opcode, cairo_m_common::instruction::STORE_IMM);
        assert_eq!(b.instructions[2].opcode, cairo_m_common::instruction::STORE_ADD_FP_IMM);
        assert_eq!(b.instructions[3].opcode, STORE_SUB_FP_FP);
    }

    #[test]
    fn test_u32_and_immediate_left_normalized() {
        let (mut b, a, _) = mk_builder_two_ops();
        // (imm & x) should normalize to (x & imm) and use FP_IMM
        b.u32_op(
            BinaryOp::U32BitwiseAnd,
            6,
            Value::integer(0xF0F0_F0F0),
            Value::operand(a),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, cairo_m_common::instruction::U32_STORE_AND_FP_IMM);
        assert_eq!(i.op0(), Some(0)); // src operand offset
        // Check encoded imm split
        assert_eq!(i.op1(), Some(0xF0F0_F0F0u32 as i32 & 0xFFFF));
        assert_eq!(i.op2(), Some(((0xF0F0_F0F0u32 >> 16) & 0xFFFF) as i32));
        // dest
        let last = &i.operands[3];
        if let Operand::Literal(v) = last {
            assert_eq!(*v, 6);
        } else {
            panic!("expected literal dest off")
        }
    }

    #[test]
    fn test_u32_or_immediate_left_normalized() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(
            BinaryOp::U32BitwiseOr,
            7,
            Value::integer(0x0000_FFFF),
            Value::operand(a),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, cairo_m_common::instruction::U32_STORE_OR_FP_IMM);
        assert_eq!(i.op0(), Some(0));
        assert_eq!(i.op2(), Some(0x0000));
        let last = &i.operands[3];
        if let Operand::Literal(v) = last {
            assert_eq!(*v, 7);
        } else {
            panic!("expected literal dest off")
        }
    }

    #[test]
    fn test_u32_eq_immediate_left_normalized() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Eq, 2, Value::integer(12345), Value::operand(a))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, U32_STORE_EQ_FP_IMM);
        assert_eq!(i.op0(), Some(0)); // src operand offset
        let last = &i.operands[3];
        if let Operand::Literal(v) = last {
            assert_eq!(*v, 2);
        } else {
            panic!("expected literal dest off")
        }
    }
}
