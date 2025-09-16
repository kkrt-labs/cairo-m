//! Felt operations: arithmetic, boolean eq/neq/and/or/not. Delegates opcode
//! selection to `opcodes` and uses `emit` to push instructions.

use crate::{CodegenError, CodegenResult, InstructionBuilder};
use cairo_m_common::Instruction as CasmInstr;
use cairo_m_compiler_mir::{BinaryOp, Literal, Value};
use stwo_prover::core::fields::m31::M31;

macro_rules! felt_fp_fp_op {
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

macro_rules! felt_fp_imm_op {
    ($name:ident, $instr:ident) => {
        pub(crate) fn $name(&mut self, src0_off: i32, imm: i32, dst_off: i32, comment: String) {
            let instr: InstructionBuilder = InstructionBuilder::from(CasmInstr::$instr {
                src_off: M31::from(src0_off),
                imm: M31::from(imm),
                dst_off: M31::from(dst_off),
            })
            .with_comment(comment);
            self.emit_push(instr);
        }
    };
}

impl super::CasmBuilder {
    #[inline]
    const fn literal_to_i32(l: &Literal) -> i32 {
        match l {
            Literal::Integer(v) => *v as i32,
            Literal::Boolean(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    pub(super) fn felt_arith(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Normalize commutative immediate-left cases to immediate-right
        let (left, right) = super::normalize::canonicalize_commutative_felt(op, left, right);

        match (&left, &right) {
            (Value::Operand(lid), Value::Operand(rid)) => {
                let l_operand = self.layout.get_offset(*lid)?;
                let r_operand = self.layout.get_offset(*rid)?;
                self.felt_fp_fp_op(op, l_operand, r_operand, dest_off)?;
            }
            (Value::Operand(lid), Value::Literal(lit)) => {
                let lo = self.layout.get_offset(*lid)?;
                let imm = Self::literal_to_i32(lit);
                self.felt_fp_imm_op(op, lo, imm, dest_off)?;
            }
            (Value::Literal(lit), Value::Operand(rid)) => {
                // Only add/mul are commutative; for sub/div use a temp
                match op {
                    BinaryOp::Add | BinaryOp::Mul => {
                        let ro = self.layout.get_offset(*rid)?;
                        let imm = Self::literal_to_i32(lit);
                        self.felt_fp_imm_op(op, ro, imm, dest_off)?;
                    }
                    BinaryOp::Sub | BinaryOp::Div => {
                        // Stage immediate then use fp-fp form
                        let tmp = self.layout.reserve_stack(1);
                        let imm = Self::literal_to_i32(lit) as u32;
                        self.store_immediate(imm, tmp, format!("[fp + {tmp}] = {imm}"));
                        let ro = self.layout.get_offset(*rid)?;
                        self.felt_fp_fp_op(op, tmp, ro, dest_off)?;
                    }
                    _ => unreachable!(),
                }
            }
            (Value::Literal(l), Value::Literal(r)) => {
                // Constant fold on host for felt using M31 field arithmetic, booleans map to 0/1
                let l_m31 = M31::from(Self::literal_to_i32(l) as u32);
                let r_m31 = M31::from(Self::literal_to_i32(r) as u32);
                let res = match op {
                    BinaryOp::Add => l_m31 + r_m31,
                    BinaryOp::Sub => l_m31 - r_m31,
                    BinaryOp::Mul => l_m31 * r_m31,
                    BinaryOp::Div => {
                        // Reject division by zero in the field (including values congruent to 0 mod M31)
                        if r_m31.0 == 0 {
                            return Err(CodegenError::InvalidMir("Division by zero".into()));
                        }
                        l_m31 * r_m31.inverse()
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Invalid felt const op".into(),
                        ))
                    }
                }
                .0;
                self.store_immediate(res, dest_off, format!("[fp + {dest_off}] = {res}"));
                return Ok(());
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub(super) fn felt_eq(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        // Compute left - right, then set 1 if zero else 0
        self.felt_arith(BinaryOp::Sub, dest_off, left, right)?;
        let non_zero = self.emit_new_label_name("not_zero");
        let end = self.emit_new_label_name("end");
        self.emit_branch_on_nonzero_set_bool(dest_off, true, non_zero, end);
        Ok(())
    }

    pub(super) fn felt_neq(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        self.felt_arith(BinaryOp::Sub, dest_off, left, right)?;
        let non_zero = self.emit_new_label_name("neq_non_zero");
        let end = self.emit_new_label_name("neq_end");
        self.emit_branch_on_nonzero_set_bool(dest_off, false, non_zero, end);
        Ok(())
    }

    fn emit_branch_on_nonzero_set_bool(
        &mut self,
        dest_off: i32,
        true_on_zero: bool,
        non_zero_label: String,
        end_label: String,
    ) {
        self.jnz_offset(dest_off, &non_zero_label);

        if true_on_zero {
            self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        } else {
            self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        }

        self.jump(&end_label);

        self.emit_add_label(crate::Label::new(non_zero_label));
        if true_on_zero {
            self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        } else {
            self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        }
        self.emit_add_label(crate::Label::new(end_label));
    }

    pub(crate) fn felt_fp_fp_op(
        &mut self,
        op: BinaryOp,
        src0_off: i32,
        src1_off: i32,
        dst_off: i32,
    ) -> CodegenResult<()> {
        let comment = format!("[fp + {dst_off}] = [fp + {src0_off}] op [fp + {src1_off}]");
        match op {
            BinaryOp::Add => self.felt_add_fp_fp(src0_off, src1_off, dst_off, comment),
            BinaryOp::Sub => self.felt_sub_fp_fp(src0_off, src1_off, dst_off, comment),
            BinaryOp::Mul => self.felt_mul_fp_fp(src0_off, src1_off, dst_off, comment),
            BinaryOp::Div => self.felt_div_fp_fp(src0_off, src1_off, dst_off, comment),
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Invalid felt fp-fp op".into(),
                ))
            }
        };
        Ok(())
    }

    pub(crate) fn felt_fp_imm_op(
        &mut self,
        op: BinaryOp,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
    ) -> CodegenResult<()> {
        let comment = format!("[fp + {dst_off}] = [fp + {src0_off}] {op} {imm}");
        match op {
            BinaryOp::Add => self.felt_add_fp_imm(src0_off, imm, dst_off, comment),
            BinaryOp::Sub => self.felt_sub_fp_imm(src0_off, imm, dst_off, comment),
            BinaryOp::Mul => self.felt_mul_fp_imm(src0_off, imm, dst_off, comment),
            BinaryOp::Div => self.felt_div_fp_imm(src0_off, imm, dst_off, comment)?,
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Invalid felt fp-imm op".into(),
                ))
            }
        };
        Ok(())
    }

    felt_fp_fp_op!(felt_add_fp_fp, StoreAddFpFp);
    felt_fp_fp_op!(felt_sub_fp_fp, StoreSubFpFp);
    felt_fp_fp_op!(felt_mul_fp_fp, StoreMulFpFp);
    felt_fp_fp_op!(felt_div_fp_fp, StoreDivFpFp);

    felt_fp_imm_op!(felt_mul_fp_imm, StoreMulFpImm);
    felt_fp_imm_op!(felt_add_fp_imm, StoreAddFpImm);
    felt_fp_imm_op!(felt_le_fp_imm, StoreLeFpImm);

    pub(crate) fn felt_sub_fp_imm(
        &mut self,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
        _comment: String,
    ) {
        // a - imm = a + (-imm)
        let imm_enc = m31_negate_imm(imm as u32);
        let comment = format!(
            "[fp + {dst_off}] = [fp + {src0_off}] + (-{imm}) (-{imm} as M31 -> {})",
            fmt_m31_imm(imm_enc)
        );
        self.felt_add_fp_imm(src0_off, imm_enc, dst_off, comment)
    }

    pub(crate) fn felt_div_fp_imm(
        &mut self,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
        _comment: String,
    ) -> CodegenResult<()> {
        // a / imm = a * inv(imm)
        let imm_enc = m31_inverse_imm(imm as u32)?;
        let comment = format!(
            "[fp + {dst_off}] = [fp + {src0_off}] * (1/{imm}) (inv({imm}) as M31 -> {})",
            fmt_m31_imm(imm_enc)
        );
        self.felt_mul_fp_imm(src0_off, imm_enc, dst_off, comment);
        Ok(())
    }

    pub(super) fn bool_and(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        self.sc_and(dest_off, &left, &right)
    }

    pub(super) fn bool_or(
        &mut self,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        self.sc_or(dest_off, &left, &right)
    }

    pub(super) fn bool_not(&mut self, dest_off: i32, source: Value) -> CodegenResult<()> {
        self.sc_not(dest_off, &source)
    }
}

/// Compute the M31-representation of `-imm` (i.e. add with negated immediate).
#[inline]
pub(super) fn m31_negate_imm(imm: u32) -> i32 {
    (M31::from(0) - M31::from(imm)).0 as i32
}

/// Compute the M31-representation of `imm.inverse()` used to compile divisions
/// by an immediate as a multiplication by the inverse.
///
/// Notes:
/// - Rejects all immediates that are zero in the M31 field.
///   For example `2147483647 (== 0 mod M31)` has no inverse and is rejected.
/// - This mirrors constant-folding behavior above and ensures consistent
///   division-by-zero detection at codegen time.
#[inline]
pub(super) fn m31_inverse_imm(imm: u32) -> CodegenResult<i32> {
    // Treat values congruent to 0 mod M31 as zero (e.g., 2147483647)
    if imm == 0 || M31::from(imm).0 == 0 {
        return Err(CodegenError::InvalidMir(
            "Division by zero with felt immediate".to_string(),
        ));
    }
    Ok(M31::from(imm).inverse().0 as i32)
}

/// Pretty-print an M31 immediate showing both its raw value and signed view when helpful.
/// Example: 2147483642 (= -5 mod M31)
#[inline]
pub(super) fn fmt_m31_imm(raw: i32) -> String {
    // M31 modulus
    const P: i32 = 2147483647; // 2^31 - 1
    if raw == 0 {
        return "0".to_string();
    }
    // If this looks like a large positive close to P, show the negative representative too
    // Threshold: values > P/2 are displayed as negatives for readability
    if raw > P / 2 {
        let neg = raw - P; // guaranteed negative
        format!("{raw} (=-{:#} mod M31)", -neg)
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{exec, ExecutionError, Mem};
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_compiler_mir::{BinaryOp, Value, ValueId};
    use proptest::prelude::*;
    use stwo_prover::core::fields::m31::{self, M31};

    // =========================================================================
    // Test Setup Helpers
    // =========================================================================

    fn mk_builder_with_value(val: u32) -> (CasmBuilder, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        layout.allocate_value(a, 1).unwrap();
        let mut builder = CasmBuilder::new(layout, 0);
        // Store initial value at fp+0
        builder.store_immediate(val, 0, format!("[fp + 0] = {val}"));
        (builder, a)
    }

    // Edge case value generation for property tests
    fn felt_value_strategy() -> impl Strategy<Value = u32> {
        prop_oneof![
            Just(0u32),          // Zero
            Just(1u32),          // One
            Just(2147483646u32), // M31 - 1 (max value in field)
            Just(2147483647u32), // M31 itself (becomes 0 in field)
            Just(1073741823u32), // (M31 - 1) / 2
            Just(2u32),          // Small prime
            Just(3u32),          // Small prime
            Just(5u32),          // Small prime
            Just(7u32),          // Small prime
            (0u32..m31::P - 1),  // Valid M31 range
        ]
    }

    // =========================================================================
    // Immediate Operation Structure Checks
    // =========================================================================

    #[test]
    fn test_felt_sub_immediate_uses_add_with_negated_imm() {
        let (mut b, left) = mk_builder_with_value(100);
        let imm = 30u32;
        b.felt_arith(BinaryOp::Sub, 5, Value::operand(left), Value::integer(imm))
            .unwrap();

        // Should use ADD with negated immediate
        let neg_imm = m31_negate_imm(imm);
        match b.instructions[1].inner_instr() {
            CasmInstr::StoreAddFpImm { imm, .. } => assert_eq!(*imm, M31::from(neg_imm)),
            other => panic!("expected StoreAddFpImm, got {other:?}"),
        }

        let mut mem = Mem::new(10);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(5), M31::from(70));
    }

    #[test]
    fn test_felt_div_immediate_uses_mul_by_inverse() {
        let (mut b, left) = mk_builder_with_value(21);
        let imm = 7u32;
        b.felt_arith(BinaryOp::Div, 5, Value::operand(left), Value::integer(imm))
            .unwrap();

        // Should use MUL with inverse
        let inv = m31_inverse_imm(imm).unwrap();
        match b.instructions[1].inner_instr() {
            CasmInstr::StoreMulFpImm { imm, .. } => assert_eq!(*imm, M31::from(inv)),
            other => panic!("expected StoreMulFpImm, got {other:?}"),
        }
    }

    #[test]
    fn test_felt_div_by_zero_immediate_errors() {
        let (mut b, left) = mk_builder_with_value(100);
        let err = b.felt_arith(BinaryOp::Div, 3, Value::operand(left), Value::integer(0));
        assert!(err.is_err());
    }

    #[test]
    fn test_m31_inverse_imm_rejects_p_equiv_zero() {
        // M31 modulus (2^31 - 1) maps to zero in the field and must be rejected
        let p = 2147483647u32;
        let inv = m31_inverse_imm(p);
        assert!(
            inv.is_err(),
            "expected division-by-zero error for P ≡ 0 mod M31"
        );
    }

    // =========================================================================
    // Commutative Operations (imm on left) structure
    // =========================================================================
    #[test]
    fn test_felt_add_imm_left_normalized() {
        let (mut b, right) = mk_builder_with_value(50);
        b.felt_arith(BinaryOp::Add, 5, Value::integer(100), Value::operand(right))
            .unwrap();

        // Should normalize to fp+imm form
        match b.instructions[1].inner_instr() {
            CasmInstr::StoreAddFpImm { imm, dst_off, .. } => {
                assert_eq!(*imm, M31::from(100));
                assert_eq!(*dst_off, M31::from(5));
            }
            other => panic!("expected StoreAddFpImm, got {other:?}"),
        }

        let mut mem = Mem::new(10);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(5), M31::from(150));
    }

    #[test]
    fn test_felt_mul_imm_left_normalized() {
        let (mut b, right) = mk_builder_with_value(7);
        b.felt_arith(BinaryOp::Mul, 5, Value::integer(11), Value::operand(right))
            .unwrap();

        // Should normalize to fp*imm form
        match b.instructions[1].inner_instr() {
            CasmInstr::StoreMulFpImm { imm, dst_off, .. } => {
                assert_eq!(*imm, M31::from(11));
                assert_eq!(*dst_off, M31::from(5));
            }
            other => panic!("expected StoreMulFpImm, got {other:?}"),
        }

        let mut mem = Mem::new(10);
        exec(&mut mem, &b.instructions).unwrap();
        assert_eq!(mem.get(5), M31::from(77));
    }

    // =========================================================================
    // Constant Folding
    // =========================================================================

    proptest! {
        #[test]
        fn test_felt_const_fold_add(lhs in felt_value_strategy(), rhs in felt_value_strategy()) {
            let layout = FunctionLayout::new_for_test();
            let mut b = CasmBuilder::new(layout, 0);

            b.felt_arith(BinaryOp::Add, 5, Value::integer(lhs), Value::integer(rhs))
                .unwrap();

            // Should fold to single store_imm
            assert_eq!(b.instructions.len(), 1);
            let expected = (M31::from(lhs) + M31::from(rhs)).0;
            match b.instructions[0].inner_instr() {
                CasmInstr::StoreImm { imm, .. } => assert_eq!(imm.0, expected),
                other => panic!("expected StoreImm, got {other:?}"),
            }
        }

        #[test]
        fn test_felt_const_fold_sub(lhs in felt_value_strategy(), rhs in felt_value_strategy()) {
            let layout = FunctionLayout::new_for_test();
            let mut b = CasmBuilder::new(layout, 0);

            b.felt_arith(BinaryOp::Sub, 5, Value::integer(lhs), Value::integer(rhs))
                .unwrap();

            assert_eq!(b.instructions.len(), 1);
            let expected = (M31::from(lhs) - M31::from(rhs)).0;
            match b.instructions[0].inner_instr() {
                CasmInstr::StoreImm { imm, .. } => assert_eq!(imm.0, expected),
                other => panic!("expected StoreImm, got {other:?}"),
            }
        }

        #[test]
        fn test_felt_const_fold_mul(lhs in felt_value_strategy(), rhs in felt_value_strategy()) {
            let layout = FunctionLayout::new_for_test();
            let mut b = CasmBuilder::new(layout, 0);

            b.felt_arith(BinaryOp::Mul, 5, Value::integer(lhs), Value::integer(rhs))
                .unwrap();

            assert_eq!(b.instructions.len(), 1);
            let expected = (M31::from(lhs) * M31::from(rhs)).0;
            match b.instructions[0].inner_instr() {
                CasmInstr::StoreImm { imm, .. } => assert_eq!(imm.0, expected),
                other => panic!("expected StoreImm, got {other:?}"),
            }
        }

        #[test]
        fn test_felt_const_fold_div_by_zero_errors(lhs in felt_value_strategy()) {
            let layout = FunctionLayout::new_for_test();
            let mut b = CasmBuilder::new(layout, 0);

            let err = b.felt_arith(BinaryOp::Div, 5, Value::integer(lhs), Value::integer(0));
            assert!(err.is_err());
        }
    }

    // =========================================================================
    // Execution Tests
    // =========================================================================

    proptest! {
        #[test]
        fn prop_felt_arith_proptest(
            a in felt_value_strategy(),
            b in felt_value_strategy(),
            left_reg: bool,
            right_reg: bool,
            op in prop::sample::select(&[BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div]),
        ) {
                let got = run_felt_op_generic(op, left_reg, a, right_reg, b);
                // In M31 field, values congruent to 0 (e.g., 0 and 2147483647) have no inverse
                let b_is_zero_mod_p = M31::from(b).0 == 0;
                if matches!(op, BinaryOp::Div) && b_is_zero_mod_p && !right_reg {
                    // Division by immediate zero (mod P) is rejected at codegen time
                    prop_assert!(matches!(got.unwrap_err(), ExecutionError::InvalidOperands));
                    return Ok(());
                }
                if matches!(op, BinaryOp::Div) && b_is_zero_mod_p && right_reg {
                    // Division by zero (mod P) register is a runtime error in executor
                    prop_assert!(matches!(got.unwrap_err(), ExecutionError::DivisionByZero));
                    return Ok(());
                }
                let exp = match op {
                    BinaryOp::Add => (M31::from(a) + M31::from(b)).0,
                    BinaryOp::Sub => (M31::from(a) - M31::from(b)).0,
                    BinaryOp::Mul => (M31::from(a) * M31::from(b)).0,
                    BinaryOp::Div => (M31::from(a) * M31::from(b).inverse()).0,
                    _ => unreachable!(),
                };
                prop_assert_eq!(got.unwrap(), exp, "op={:?} a={} b={} left_reg={} right_reg={}",
                op, a, b, left_reg, right_reg);
        }
    }

    // Generic runner for felt arithmetic operations used in property tests
    fn run_felt_op_generic(
        op: BinaryOp,
        left_reg: bool,
        a: u32,
        right_reg: bool,
        b: u32,
    ) -> Result<u32, ExecutionError> {
        let mut layout = FunctionLayout::new_for_test();
        let left_id = ValueId::from_raw(1);
        let right_id = ValueId::from_raw(2);
        if left_reg {
            layout.allocate_value(left_id, 1).unwrap();
        }
        if right_reg {
            layout.allocate_value(right_id, 1).unwrap();
        }

        let mut bld = CasmBuilder::new(layout, 0);
        let mut next_off = 0;
        if left_reg {
            bld.store_immediate(a, next_off, "a".into());
            next_off += 1;
        }
        if right_reg {
            bld.store_immediate(b, next_off, "b".into());
        }

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

        let dest_off: i32 = bld.layout_mut().reserve_stack(2);
        match bld.felt_arith(op, dest_off, left, right) {
            Ok(()) => {}
            Err(CodegenError::InvalidMir(msg)) if msg.contains("Division by zero") => {
                return Err(ExecutionError::InvalidOperands)
            }
            Err(e) => panic!("Unexpected codegen error: {:?}", e),
        }

        let mut mem = Mem::new(64);
        exec(&mut mem, &bld.instructions)?;
        Ok(mem.get(dest_off).0)
    }
}
