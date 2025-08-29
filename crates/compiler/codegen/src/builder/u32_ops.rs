//! u32 operations: arithmetic, comparisons, bitwise, immediate transforms.

use super::normalize::{normalize_u32_cmp_fp_fp, normalize_u32_cmp_fp_imm};
use super::opcodes::{u32_fp_fp, u32_fp_imm};
use crate::{CodegenError, CodegenResult, InstructionBuilder, Operand};
use cairo_m_compiler_mir::{BinaryOp, Literal, Value};

fn is_u32_cmp_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::U32Eq
            | BinaryOp::U32Neq
            | BinaryOp::U32Greater
            | BinaryOp::U32GreaterEqual
            | BinaryOp::U32Less
            | BinaryOp::U32LessEqual
    )
}

impl super::CasmBuilder {
    pub(super) fn u32_op(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        let is_cmp_op = is_u32_cmp_op(op);
        let result_size = if is_cmp_op { 1 } else { 2 };

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
                        let neg = twos_complement_u32(orig_imm);
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

                if matches!(op, BinaryOp::U32Div) && *imm == 0 {
                    return Err(CodegenError::InvalidMir("Division by zero".into()));
                }

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
                        let (lo, hi) = split_u32_i32(*imm as i32);
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
                        let is_cmp = is_u32_cmp_op(op);
                        if is_cmp {
                            let norm = normalize_u32_cmp_fp_fp(op);
                            let (mut left_off, mut right_off) = (tmp, ro);
                            if norm.swap {
                                std::mem::swap(&mut left_off, &mut right_off);
                            }
                            let instr = InstructionBuilder::new(u32_fp_fp(norm.op)?)
                                .with_operand(Operand::Literal(left_off))
                                .with_operand(Operand::Literal(right_off))
                                .with_operand(Operand::Literal(dest_off))
                                .with_comment(format!(
                                    "[fp + {dest_off}] = u32([fp + {left_off}], [fp + {}]) {op} u32([fp + {right_off}], [fp + {}])",
                                    left_off + 1,
                                    right_off + 1
                                ));
                            self.emit_push(instr);
                            self.emit_touch(dest_off, 1);
                            if norm.complement {
                                self.complement_felt_in_place(dest_off);
                            }
                        } else {
                            let instr = InstructionBuilder::new(u32_fp_fp(op)?)
                                .with_operand(Operand::Literal(tmp))
                                .with_operand(Operand::Literal(ro))
                                .with_operand(Operand::Literal(dest_off))
                                .with_comment(format!(
                                    "u32([fp + {dest_off}], [fp + {}]) = u32([fp + {tmp}]) {op} u32([fp + {ro}])",
                                    dest_off + 1
                                ));
                            self.emit_push(instr);
                            self.emit_touch(dest_off, result_size);
                        }
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported op: {op}"
                        )))
                    }
                }
            }

            (Value::Literal(Literal::Integer(li)), Value::Literal(Literal::Integer(ri))) => {
                // Constant fold arith, bitwise, and comparisons for u32
                let l = { *li };
                let r = { *ri };

                if is_cmp_op {
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
                        BinaryOp::U32BitwiseAnd => l & r,
                        BinaryOp::U32BitwiseOr => l | r,
                        BinaryOp::U32BitwiseXor => l ^ r,
                        _ => {
                            return Err(CodegenError::UnsupportedInstruction(format!(
                                "Unsupported u32 lit-lit operation: {op}"
                            )));
                        }
                    };
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

/// Helper to split an i32 value (interpreted as u32) into low and high 16-bit parts
#[inline]
pub(super) const fn split_u32_i32(value: i32) -> (i32, i32) {
    let u = value as u32;
    ((u & 0xFFFF) as i32, ((u >> 16) & 0xFFFF) as i32)
}

/// Two's complement on 32-bit for u32 subtraction with an immediate
#[inline]
pub(super) const fn twos_complement_u32(imm: u32) -> u32 {
    (!imm).wrapping_add(1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{exec, ExecutionError, Mem};
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::instruction::{
        STORE_SUB_FP_FP, U32_STORE_ADD_FP_FP, U32_STORE_ADD_FP_IMM, U32_STORE_EQ_FP_IMM,
        U32_STORE_LT_FP_IMM,
    };
    use cairo_m_compiler_mir::{BinaryOp, Value, ValueId};

    // Helper to create a builder with two u32 operands
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
        assert_eq!(
            b.instructions[1].opcode,
            cairo_m_common::instruction::STORE_IMM
        );
        assert_eq!(
            b.instructions[2].opcode,
            cairo_m_common::instruction::STORE_ADD_FP_IMM
        );
        assert_eq!(b.instructions[3].opcode, STORE_SUB_FP_FP);
    }

    #[test]
    fn test_u32_greater_boundary_optimization() {
        let (mut b, a, _) = mk_builder_two_ops();

        // Test x > 0xFFFF_FFFF should always be false
        b.u32_op(
            BinaryOp::U32Greater,
            4,
            Value::operand(a),
            Value::integer(0xFFFF_FFFF),
        )
        .unwrap();

        // Should optimize to storing 0 directly
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(
            b.instructions[0].opcode,
            cairo_m_common::instruction::STORE_IMM
        );
        assert_eq!(b.instructions[0].op0(), Some(0)); // immediate value = 0
        assert_eq!(b.instructions[0].op1(), Some(4)); // destination offset
    }

    #[test]
    fn test_u32_less_equal_boundary_optimization() {
        let (mut b, a, _) = mk_builder_two_ops();

        // Test x <= 0xFFFF_FFFF should always be true
        b.u32_op(
            BinaryOp::U32LessEqual,
            5,
            Value::operand(a),
            Value::integer(0xFFFF_FFFF),
        )
        .unwrap();

        // Should optimize to storing 1 directly
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(
            b.instructions[0].opcode,
            cairo_m_common::instruction::STORE_IMM
        );
        assert_eq!(b.instructions[0].op0(), Some(1)); // immediate value = 1
        assert_eq!(b.instructions[0].op1(), Some(5)); // destination offset
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

    // -------------------------
    // Property/boundary tests
    // -------------------------

    use proptest::prelude::*;
    use proptest::strategy::{Just, Strategy};

    fn u32_strategy() -> impl Strategy<Value = u32> {
        prop_oneof![
            Just(0u32),        // Zero
            Just(1u32),        // One
            Just(0x7FFF_FFFF), // Max value in M31 field
            Just(0x8000_0000), // M31 field boundary
            Just(0xFFFF_FFFE), // Max u32 - 1
            Just(0xFFFF_FFFF), // Max u32
            any::<u32>(),
        ]
    }

    fn run_u32_op_generic(
        op: BinaryOp,
        left_reg: bool,
        a: u32,
        right_reg: bool,
        b: u32,
        dest_u32: bool,
    ) -> Result<u32, ExecutionError> {
        let mut layout = FunctionLayout::new_for_test();

        let left_id = ValueId::from_raw(1);
        let right_id = ValueId::from_raw(2);
        if left_reg {
            layout.allocate_value(left_id, 2).unwrap();
        }
        if right_reg {
            layout.allocate_value(right_id, 2).unwrap();
        }

        let mut bld = CasmBuilder::new(layout, 0);
        let mut next_off = 0;
        if left_reg {
            bld.store_u32_immediate(a, next_off, "a".into());
            next_off += 2;
        }
        if right_reg {
            bld.store_u32_immediate(b, next_off, "b".into());
        }

        // Recreate Values with their actual ids at known positions
        let left = if left_reg {
            Value::operand(left_id)
        } else {
            Value::integer(a)
        };
        let right = if right_reg {
            Value::operand(right_id)
        } else {
            Value::integer(b)
        };

        const DEST_OFF: i32 = 10;
        bld.u32_op(op, DEST_OFF, left, right).map_err(|e| match e {
            CodegenError::InvalidMir(msg) if msg.contains("Division by zero") => {
                ExecutionError::InvalidOperands
            }
            _ => panic!("Unexpected codegen error: {:?}", e),
        })?;

        // For (lit, lit) cases, the result is immediately computed and stored
        // Check if any instructions were generated
        if bld.instructions.is_empty() {
            // No instructions means it was an invalid operation
            return Err(ExecutionError::DivisionByZero);
        }

        let mut mem = Mem::new(64);
        exec(&mut mem, &bld.instructions)?;
        if dest_u32 {
            Ok(mem.get_u32(DEST_OFF))
        } else {
            Ok(mem.get(DEST_OFF).0)
        }
    }

    proptest! {
        #[test]
        fn property_u32_cmp_proptest(
            a in u32_strategy(),
            c in u32_strategy(),
            left_reg: bool,
            right_reg: bool,
        ) {
            let ops = [
                BinaryOp::U32Eq,
                BinaryOp::U32Neq,
                BinaryOp::U32Less,
                BinaryOp::U32LessEqual,
                BinaryOp::U32Greater,
                BinaryOp::U32GreaterEqual,
            ];
            for &op in &ops {
                let got = run_u32_op_generic(op, left_reg, a, right_reg, c, false);
                let exp = match op {
                    BinaryOp::U32Eq => (a == c) as u32,
                    BinaryOp::U32Neq => (a != c) as u32,
                    BinaryOp::U32Less => (a < c) as u32,
                    BinaryOp::U32LessEqual => (a <= c) as u32,
                    BinaryOp::U32Greater => (a > c) as u32,
                    BinaryOp::U32GreaterEqual => (a >= c) as u32,
                    _ => unreachable!(),
                };
                prop_assert_eq!(got.unwrap(), exp, "op={:?} a={} c={}", op, a, c);
            }
        }
    }

    proptest! {
        #[test]
        fn property_u32_arith_proptest(
            a in u32_strategy(),
            b in u32_strategy(),
            left_reg: bool,
            right_reg: bool,
        ) {
            let ops = [
                BinaryOp::U32Add,
                BinaryOp::U32Sub,
                BinaryOp::U32Mul,
                BinaryOp::U32Div,
                BinaryOp::U32BitwiseAnd,
                BinaryOp::U32BitwiseOr,
                BinaryOp::U32BitwiseXor,
            ];
            for &op in &ops {
                // Skip division by zero test in property test (handled separately)
                let got = run_u32_op_generic(op, left_reg, a, right_reg, b, true);
                if matches!(op, BinaryOp::U32Div) && b == 0 && right_reg {
                    prop_assert!(matches!(got.unwrap_err(), ExecutionError::DivisionByZero));
                    continue;
                }
                if matches!(op, BinaryOp::U32Div) && b == 0 && !right_reg {
                    let err = got.unwrap_err();
                    prop_assert!(matches!(err, ExecutionError::InvalidOperands), "got error: {:?}", err);
                    continue;
                }
                let exp = match op {
                    BinaryOp::U32Add => a.wrapping_add(b),
                    BinaryOp::U32Sub => a.wrapping_sub(b),
                    BinaryOp::U32Mul => a.wrapping_mul(b),
                    BinaryOp::U32Div => a.wrapping_div(b),
                    BinaryOp::U32BitwiseAnd => a & b,
                    BinaryOp::U32BitwiseOr => a | b,
                    BinaryOp::U32BitwiseXor => a ^ b,
                    _ => unreachable!(),
                };
                prop_assert_eq!(got.unwrap(), exp, "op={:?} a={} b={} left_reg={} right_reg={}",
                    op, a, b, left_reg, right_reg);
            }
        }
    }
}
