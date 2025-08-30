//! u32 operations: arithmetic, comparisons, bitwise, immediate transforms.
//!
//! Comment conventions for immediates:
//! - We split 32-bit immediates into 16-bit limbs passed as operands (lo, hi).
//! - For readability, we also include the full 32-bit immediate in hex in a
//!   trailing comment `/* imm = 0x........ */`.
//! - For `Sub` with an immediate, we bias using two's complement and note the
//!   original vs encoded value in the comment.
use crate::{CodegenError, CodegenResult, InstructionBuilder};
use cairo_m_common::Instruction as CasmInstr;
use cairo_m_compiler_mir::{BinaryOp, Literal, Value};
use stwo_prover::core::fields::m31::M31;

macro_rules! u32_fp_fp_op {
    ($name:ident, $instr:ident) => {
        pub(crate) fn $name(
            &mut self,
            src0_off: i32,
            src1_off: i32,
            dst_off: i32,
            comment: String,
        ) {
            let instr: InstructionBuilder = InstructionBuilder::from(CasmInstr::$instr {
                src0_off: M31::from(src0_off),
                src1_off: M31::from(src1_off),
                dst_off: M31::from(dst_off),
            })
            .with_comment(comment);
            self.emit_push(instr);
        }
    };
}

macro_rules! u32_fp_imm_op {
    ($name:ident, $instr:ident) => {
        pub(crate) fn $name(&mut self, src0_off: i32, imm: u32, dst_off: i32, comment: String) {
            let (imm_lo, imm_hi) = super::split_u32_value(imm);
            let instr: InstructionBuilder = InstructionBuilder::from(CasmInstr::$instr {
                src_off: M31::from(src0_off),
                imm_lo: M31::from(imm_lo),
                imm_hi: M31::from(imm_hi),
                dst_off: M31::from(dst_off),
            })
            .with_comment(comment);
            self.emit_push(instr);
        }
    };
}
// No local is_u32_cmp_op: comparisons are restricted by design to U32Eq|U32Less.

fn check_op(op: BinaryOp) -> CodegenResult<()> {
    // Unsupported comparison forms must be legalized earlier.
    if matches!(
        op,
        BinaryOp::U32Neq
            | BinaryOp::U32Greater
            | BinaryOp::U32GreaterEqual
            | BinaryOp::U32LessEqual
    ) {
        return Err(CodegenError::UnsupportedInstruction(format!(
            "Unsupported u32 comparison in builder: {op}. Expected U32Eq or U32Less"
        )));
    }
    Ok(())
}

const fn is_cmp_u32_op(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::U32Eq | BinaryOp::U32Less)
}

impl super::CasmBuilder {
    pub(super) fn u32_op(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        check_op(op)?;
        let is_cmp_op = is_cmp_u32_op(op);
        // Normalize commutative immediate-left cases to immediate-right
        let (left, right) = super::normalize::canonicalize_commutative_u32(op, left, right);

        match (&left, &right) {
            (Value::Operand(lid), Value::Operand(rid)) => {
                let lo = self.layout.get_offset(*lid)?;
                let ro = self.layout.get_offset(*rid)?;

                if is_cmp_op {
                    self.u32_fp_fp_cmp(op, lo, ro, dest_off)?;
                } else {
                    self.u32_fp_fp_op(op, lo, ro, dest_off)?;
                };
            }

            (Value::Operand(lid), Value::Literal(Literal::Integer(imm))) => {
                let lo = self.layout.get_offset(*lid)?;

                if matches!(op, BinaryOp::U32Div) && *imm == 0 {
                    return Err(CodegenError::InvalidMir("Division by zero".into()));
                }
                self.u32_fp_imm_op(op, lo, *imm, dest_off)?;
            }

            (Value::Literal(Literal::Integer(imm)), Value::Operand(rid)) => {
                let ro = self.layout.get_offset(*rid)?;
                match op {
                    // Commutative operations
                    BinaryOp::U32Add
                    | BinaryOp::U32Mul
                    | BinaryOp::U32Eq
                    | BinaryOp::U32BitwiseAnd
                    | BinaryOp::U32BitwiseOr
                    | BinaryOp::U32BitwiseXor => {
                        self.u32_fp_imm_op(op, ro, *imm, dest_off)?;
                    }
                    // Non-commutative operations - store imm in tmp
                    BinaryOp::U32Sub | BinaryOp::U32Div | BinaryOp::U32Less => {
                        let tmp = self.layout.reserve_stack(2);
                        self.store_u32_immediate(
                            *imm,
                            tmp,
                            format!("[fp + {}], [fp + {}] = u32({imm})", tmp, tmp + 1),
                        );
                        if matches!(op, BinaryOp::U32Eq | BinaryOp::U32Less) {
                            self.u32_fp_fp_cmp(op, tmp, ro, dest_off)?;
                        } else {
                            self.u32_fp_fp_op(op, tmp, ro, dest_off)?;
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

                if matches!(op, BinaryOp::U32Eq | BinaryOp::U32Less) {
                    let res = match op {
                        BinaryOp::U32Eq => (l == r) as u32,
                        BinaryOp::U32Less => (l < r) as u32,
                        _ => unreachable!(),
                    };
                    self.store_immediate(res, dest_off, format!("[fp + {dest_off}] = {res}"));
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

    pub(crate) fn u32_fp_fp_cmp(
        &mut self,
        op: BinaryOp,
        src0_off: i32,
        src1_off: i32,
        dest_off: i32,
    ) -> CodegenResult<()> {
        let comment = format!("[fp + {dest_off}] = u32([fp + {src0_off}], [fp + {}]) {op} u32([fp + {src1_off}], [fp + {}])", src0_off + 1, src1_off + 1);
        let instr: InstructionBuilder = match op {
            BinaryOp::U32Eq => InstructionBuilder::from(CasmInstr::U32StoreEqFpFp {
                src0_off: M31::from(src0_off),
                src1_off: M31::from(src1_off),
                dst_off: M31::from(dest_off),
            }),
            BinaryOp::U32Less => InstructionBuilder::from(CasmInstr::U32StoreLtFpFp {
                src0_off: M31::from(src0_off),
                src1_off: M31::from(src1_off),
                dst_off: M31::from(dest_off),
            }),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported u32 cmp op".into(),
                ))
            }
        }
        .with_comment(comment);
        self.emit_push(instr);
        Ok(())
    }

    pub(crate) fn u32_fp_fp_op(
        &mut self,
        op: BinaryOp,
        src0_off: i32,
        src1_off: i32,
        dest_off: i32,
    ) -> CodegenResult<()> {
        let comment = format!("u32([fp + {dest_off}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) {op} u32([fp + {src1_off}], [fp + {}])", dest_off + 1, src0_off + 1, src1_off + 1);
        match op {
            BinaryOp::U32Add => self.u32_add_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32Sub => self.u32_sub_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32Mul => self.u32_mul_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32Div => self.u32_div_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32BitwiseAnd => self.u32_and_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32BitwiseOr => self.u32_or_fp_fp(src0_off, src1_off, dest_off, comment),
            BinaryOp::U32BitwiseXor => self.u32_xor_fp_fp(src0_off, src1_off, dest_off, comment),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported u32 fp-fp op".into(),
                ))
            }
        };
        Ok(())
    }

    pub(crate) fn u32_fp_imm_op(
        &mut self,
        op: BinaryOp,
        src0_off: i32,
        imm: u32,
        dest_off: i32,
    ) -> CodegenResult<()> {
        let (imm_lo, imm_hi) = super::split_u32_value(imm);
        let is_cmp_op = matches!(op, BinaryOp::U32Eq | BinaryOp::U32Less);
        let imm_hex = format!("{:#010x}", imm);
        let comment = if is_cmp_op {
            let base = format!("[fp + {dest_off}] = u32([fp + {src0_off}], [fp + {}]) {op} u32({imm_lo}, {imm_hi})", src0_off + 1);
            format!("{base} /* imm = {imm_hex} */")
        } else {
            let base = format!("u32([fp + {dest_off}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) {op} u32({imm_lo}, {imm_hi})", dest_off + 1, src0_off + 1);
            format!("{base} /* imm = {imm_hex} */")
        };
        match op {
            BinaryOp::U32Add => self.u32_add_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32Sub => self.u32_sub_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32Mul => self.u32_mul_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32Div => self.u32_div_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32Eq => self.u32_eq_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32Less => self.u32_less_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32BitwiseAnd => self.u32_and_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32BitwiseOr => self.u32_or_fp_imm(src0_off, imm, dest_off, comment),
            BinaryOp::U32BitwiseXor => self.u32_xor_fp_imm(src0_off, imm, dest_off, comment),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported u32 fp-imm op".into(),
                ))
            }
        }
        Ok(())
    }

    u32_fp_fp_op!(u32_add_fp_fp, U32StoreAddFpFp);
    u32_fp_fp_op!(u32_sub_fp_fp, U32StoreSubFpFp);
    u32_fp_fp_op!(u32_mul_fp_fp, U32StoreMulFpFp);
    u32_fp_fp_op!(u32_div_fp_fp, U32StoreDivFpFp);
    u32_fp_fp_op!(u32_and_fp_fp, U32StoreAndFpFp);
    u32_fp_fp_op!(u32_or_fp_fp, U32StoreOrFpFp);
    u32_fp_fp_op!(u32_xor_fp_fp, U32StoreXorFpFp);

    u32_fp_imm_op!(u32_add_fp_imm, U32StoreAddFpImm);
    u32_fp_imm_op!(u32_mul_fp_imm, U32StoreMulFpImm);
    u32_fp_imm_op!(u32_div_fp_imm, U32StoreDivFpImm);
    u32_fp_imm_op!(u32_eq_fp_imm, U32StoreEqFpImm);
    u32_fp_imm_op!(u32_less_fp_imm, U32StoreLtFpImm);
    u32_fp_imm_op!(u32_and_fp_imm, U32StoreAndFpImm);
    u32_fp_imm_op!(u32_or_fp_imm, U32StoreOrFpImm);
    u32_fp_imm_op!(u32_xor_fp_imm, U32StoreXorFpImm);

    pub(crate) fn u32_sub_fp_imm(
        &mut self,
        src0_off: i32,
        imm: u32,
        dst_off: i32,
        _comment: String,
    ) {
        // Use U32StoreAddFpImm with two's complement of imm
        let neg = twos_complement_u32(imm);
        let (low, high) = super::split_u32_value(neg);
        let comment = format!("u32([fp + {dst_off}], [fp + {}]) = u32([fp + {src0_off}], [fp + {}]) U32Sub u32({low}, {high}) (two's complement of {imm} -> {neg})", dst_off + 1, src0_off + 1, imm = imm, neg = neg);
        let instr = InstructionBuilder::from(CasmInstr::U32StoreAddFpImm {
            src_off: M31::from(src0_off),
            imm_lo: M31::from(low),
            imm_hi: M31::from(high),
            dst_off: M31::from(dst_off),
        })
        .with_comment(comment);
        self.emit_push(instr);
    }
}

/// Two's complement on 32-bit for u32 subtraction with an immediate
#[inline]
pub(super) const fn twos_complement_u32(imm: u32) -> u32 {
    (!imm).wrapping_add(1)
}

// Notes on edge cases for immediate biasing:
// - When `imm == 0`, two's complement is also `0`, so `x - 0` becomes
//   `x + 0` and the encoded immediate stays zero — no special case needed.
// - Wrapping semantics apply (as everywhere in u32 ops), so behavior is
//   consistent with hardware-like u32 arithmetic.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{exec, ExecutionError, Mem};
    use crate::{builder::CasmBuilder, layout::FunctionLayout};

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
        assert_eq!(
            b.instructions[0].inner_instr(),
            &CasmInstr::U32StoreAddFpFp {
                src0_off: M31::from(0),
                src1_off: M31::from(2),
                dst_off: M31::from(8),
            }
        );
    }

    #[test]
    fn test_u32_sub_fp_imm_uses_add_with_twos_complement() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Sub, 5, Value::operand(a), Value::integer(7))
            .unwrap();
        let expected_imm = twos_complement_u32(7);
        let expected_imm_lo = expected_imm as i32 & 0xFFFF;
        let expected_imm_hi = (expected_imm >> 16) as i32;
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(
            b.instructions[0].inner_instr(),
            &CasmInstr::U32StoreAddFpImm {
                src_off: M31::from(0),
                imm_lo: M31::from(expected_imm_lo),
                imm_hi: M31::from(expected_imm_hi),
                dst_off: M31::from(5),
            }
        );
    }

    #[test]
    fn test_u32_eq_fp_imm() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Eq, 3, Value::operand(a), Value::integer(42))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(
            b.instructions[0].inner_instr(),
            &CasmInstr::U32StoreEqFpImm {
                src_off: M31::from(0),
                imm_lo: M31::from(42u32 as i32 & 0xFFFF),
                imm_hi: M31::from(0),
                dst_off: M31::from(3),
            }
        );
    }

    #[test]
    fn test_u32_rejected_op_in_builder() {
        let ops = vec![
            BinaryOp::U32Greater,
            BinaryOp::U32GreaterEqual,
            BinaryOp::U32LessEqual,
            BinaryOp::U32Neq,
        ];
        for op in ops {
            let (mut b, a, _) = mk_builder_two_ops();
            let err = b
                .u32_op(op, 4, Value::operand(a), Value::integer(10))
                .unwrap_err();
            match err {
                CodegenError::UnsupportedInstruction(msg) => {
                    assert!(msg.contains("Unsupported u32 comparison"));
                }
                _ => panic!("unexpected error kind: {:?}", err),
            }
        }
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
        assert_eq!(
            i.inner_instr(),
            &CasmInstr::U32StoreAndFpImm {
                src_off: M31::from(0),
                imm_lo: M31::from(0xF0F0_F0F0u32 as i32 & 0xFFFF),
                imm_hi: M31::from(((0xF0F0_F0F0u32 >> 16) & 0xFFFF) as i32),
                dst_off: M31::from(6),
            }
        );
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
        assert_eq!(
            i.inner_instr(),
            &CasmInstr::U32StoreOrFpImm {
                src_off: M31::from(0),
                imm_lo: M31::from(0x0000_FFFFu32 as i32 & 0xFFFF),
                imm_hi: M31::from(0),
                dst_off: M31::from(7),
            }
        );
    }

    #[test]
    fn test_u32_eq_immediate_left_normalized() {
        let (mut b, a, _) = mk_builder_two_ops();
        b.u32_op(BinaryOp::U32Eq, 2, Value::integer(12345), Value::operand(a))
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(
            i.inner_instr(),
            &CasmInstr::U32StoreEqFpImm {
                src_off: M31::from(0),
                imm_lo: M31::from(12345u32 as i32 & 0xFFFF),
                imm_hi: M31::from(0),
                dst_off: M31::from(2),
            }
        );
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
            let ops = [BinaryOp::U32Eq, BinaryOp::U32Less];
            for &op in &ops {
                let got = run_u32_op_generic(op, left_reg, a, right_reg, c, false);
                let exp = match op {
                    BinaryOp::U32Eq => (a == c) as u32,
                    BinaryOp::U32Less => (a < c) as u32,
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
