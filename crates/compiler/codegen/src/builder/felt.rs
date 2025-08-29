//! Felt operations: arithmetic, boolean eq/neq/and/or/not. Delegates opcode
//! selection to `opcodes` and uses `emit` to push instructions.

use crate::{CodegenError, CodegenResult, InstructionBuilder, Operand};
use cairo_m_common::instruction::*;
use cairo_m_compiler_mir::{BinaryOp, Literal, Value};
use stwo_prover::core::fields::m31::M31;

impl super::CasmBuilder {
    pub(super) fn felt_arith(
        &mut self,
        op: BinaryOp,
        dest_off: i32,
        left: Value,
        right: Value,
    ) -> CodegenResult<()> {
        use super::opcodes::{felt_fp_fp, felt_fp_imm};

        // Normalize commutative immediate-left cases to immediate-right
        let (left, right) = super::normalize::canonicalize_commutative_felt(op, left, right);

        match (&left, &right) {
            (Value::Operand(lid), Value::Operand(rid)) => {
                let l_operand = self.layout.get_offset(*lid)?;
                let r_operand = self.layout.get_offset(*rid)?;

                let instr = InstructionBuilder::new(felt_fp_fp(op)?)
                    .with_operand(Operand::Literal(l_operand))
                    .with_operand(Operand::Literal(r_operand))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(format!(
                        "[fp + {dest_off}] = [fp + {l_operand}] op [fp + {r_operand}]"
                    ));
                self.emit_push(instr);
                self.emit_touch(dest_off, 1);
            }
            (Value::Operand(lid), Value::Literal(Literal::Integer(imm))) => {
                let lo = self.layout.get_offset(*lid)?;

                // a - imm = a + (-imm)
                // a / imm = a * inv(imm)
                let (opcode, imm_enc) = match op {
                    BinaryOp::Sub => (felt_fp_imm(BinaryOp::Add)?, super::m31_negate_imm(*imm)),
                    BinaryOp::Div => (felt_fp_imm(BinaryOp::Mul)?, super::m31_inverse_imm(*imm)?),
                    _ => (felt_fp_imm(op)?, *imm as i32),
                };
                let comment = match op {
                    BinaryOp::Add => format!("[fp + {dest_off}] = [fp + {lo}] + {imm}"),
                    BinaryOp::Sub => format!(
                        "[fp + {dest_off}] = [fp + {lo}] - {imm} (-{imm} as M31 -> {})",
                        super::fmt_m31_imm(imm_enc)
                    ),
                    BinaryOp::Mul => format!("[fp + {dest_off}] = [fp + {lo}] * {imm}"),
                    BinaryOp::Div => format!(
                        "[fp + {dest_off}] = [fp + {lo}] / {imm} (inv({imm}) as M31 -> {})",
                        super::fmt_m31_imm(imm_enc)
                    ),
                    _ => unreachable!(),
                };
                let instr = InstructionBuilder::new(opcode)
                    .with_operand(Operand::Literal(lo))
                    .with_operand(Operand::Literal(imm_enc))
                    .with_operand(Operand::Literal(dest_off))
                    .with_comment(comment);
                self.emit_push(instr);
                self.emit_touch(dest_off, 1);
            }
            (Value::Literal(Literal::Integer(imm)), Value::Operand(rid)) => {
                // Only add/mul are commutative; for sub/div use a temp
                match op {
                    BinaryOp::Add | BinaryOp::Mul => {
                        let ro = self.layout.get_offset(*rid)?;
                        let comment = match op {
                            BinaryOp::Add => format!("[fp + {dest_off}] = [fp + {ro}] + {imm}"),
                            BinaryOp::Mul => format!("[fp + {dest_off}] = [fp + {ro}] * {imm}"),
                            _ => format!("[fp + {dest_off}] = [fp + {ro}] {op} {imm}"),
                        };
                        let instr = InstructionBuilder::new(super::opcodes::felt_fp_imm(op)?)
                            .with_operand(Operand::Literal(ro))
                            .with_operand(Operand::Literal(*imm as i32))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(comment);
                        self.emit_push(instr);
                        self.emit_touch(dest_off, 1);
                    }
                    BinaryOp::Sub | BinaryOp::Div => {
                        // Stage immediate then use fp-fp form
                        let tmp = self.layout.reserve_stack(1);
                        self.store_immediate(*imm, tmp, format!("[fp + {tmp}] = {imm}"));
                        let ro = self.layout.get_offset(*rid)?;
                        let instr = InstructionBuilder::new(super::opcodes::felt_fp_fp(op)?)
                            .with_operand(Operand::Literal(tmp))
                            .with_operand(Operand::Literal(ro))
                            .with_operand(Operand::Literal(dest_off))
                            .with_comment(format!(
                                "[fp + {dest_off}] = [fp + {tmp}] op [fp + {ro}]"
                            ));
                        self.emit_push(instr);
                        self.emit_touch(dest_off, 1);
                    }
                    _ => unreachable!(),
                }
            }
            (Value::Literal(Literal::Integer(l)), Value::Literal(Literal::Integer(r))) => {
                // Constant fold on host for felt using M31 field arithmetic
                let l_m31 = M31::from(*l);
                let r_m31 = M31::from(*r);
                let res = match op {
                    BinaryOp::Add => l_m31 + r_m31,
                    BinaryOp::Sub => l_m31 - r_m31,
                    BinaryOp::Mul => l_m31 * r_m31,
                    BinaryOp::Div => {
                        if *r == 0 {
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
                self.emit_touch(dest_off, 1);
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
        let non_zero = self.new_label_name("not_zero");
        let end = self.new_label_name("end");
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
        let non_zero = self.new_label_name("neq_non_zero");
        let end = self.new_label_name("neq_end");
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

        self.add_label(crate::Label::new(non_zero_label));
        if true_on_zero {
            self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        } else {
            self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        }
        self.add_label(crate::Label::new(end_label));
    }

    pub(crate) fn felt_add_fp_fp(
        &mut self,
        src0_off: i32,
        src1_off: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_ADD_FP_FP)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(src1_off))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
    }

    pub(crate) fn felt_sub_fp_fp(
        &mut self,
        src0_off: i32,
        src1_off: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_SUB_FP_FP)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(src1_off))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
    }

    pub(crate) fn felt_mul_fp_fp(
        &mut self,
        src0_off: i32,
        src1_off: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_MUL_FP_FP)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(src1_off))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
    }

    pub(crate) fn felt_mul_fp_imm(
        &mut self,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_MUL_FP_IMM)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
    }

    pub(crate) fn felt_add_fp_imm(
        &mut self,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_ADD_FP_IMM)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
    }

    pub(crate) fn felt_lower_than_fp_imm(
        &mut self,
        src0_off: i32,
        imm: i32,
        dst_off: i32,
        comment: String,
    ) {
        let instr = InstructionBuilder::new(STORE_LOWER_THAN_FP_IMM)
            .with_operand(Operand::Literal(src0_off))
            .with_operand(Operand::Literal(imm))
            .with_operand(Operand::Literal(dst_off))
            .with_comment(comment);
        self.emit_push(instr);
        self.emit_touch(dst_off, 1);
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

    /// Compute boolean complement in place: dest = 1 - dest, alias-safe.
    pub(crate) fn complement_felt_in_place(&mut self, dest_off: i32) {
        // Stage 1 in tmp_a
        let tmp_a = self.layout.reserve_stack(1);
        self.store_immediate(1, tmp_a, format!("[fp + {tmp_a}] = 1"));
        // Copy current dest to tmp_b to avoid src/dst alias
        let tmp_b = self.layout.reserve_stack(1);
        self.store_copy_single(
            dest_off,
            tmp_b,
            format!("[fp + {tmp_b}] = [fp + {dest_off}] + 0"),
        );
        // Perform subtraction: dest = 1 - old_dest
        self.felt_sub_fp_fp(
            tmp_a,
            tmp_b,
            dest_off,
            format!("[fp + {dest_off}] = 1 - [fp + {dest_off}]")
        );
    }
}

#[cfg(test)]
mod tests {

    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::instruction::{STORE_ADD_FP_IMM, STORE_MUL_FP_IMM};
    use cairo_m_compiler_mir::{BinaryOp, Value, ValueId};
    use stwo_prover::core::fields::m31::M31;

    fn mk_builder_with_left_at(off: i32) -> (CasmBuilder, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let left = ValueId::from_raw(1);
        layout.allocate_value(left, 1).unwrap(); // fp + 0
                                                 // We want left at specific off: reserve stack if needed
        if off > 0 {
            // Allocate dummy to move current offset
            // Since allocate_value above placed at 0, for testing we will shift by copying
            // Alternatively, we pass the offset literals directly in checks.
        }
        (CasmBuilder::new(layout, 0), left)
    }

    #[test]
    fn test_felt_sub_immediate_uses_add_with_negated_imm() {
        let (mut b, left) = mk_builder_with_left_at(0);
        let dest_off = 10;
        let imm = 5u32;
        b.felt_arith(
            BinaryOp::Sub,
            dest_off,
            Value::operand(left),
            Value::integer(imm),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, STORE_ADD_FP_IMM);
        assert_eq!(i.op0(), Some(0)); // left at fp+0
        let neg = (M31::from(0) - M31::from(imm)).0 as i32;
        assert_eq!(i.op1(), Some(neg));
        assert_eq!(i.op2(), Some(dest_off));
    }

    #[test]
    fn test_felt_div_immediate_uses_mul_by_inverse() {
        let (mut b, left) = mk_builder_with_left_at(0);
        let dest_off = 7;
        let imm = 3u32;
        b.felt_arith(
            BinaryOp::Div,
            dest_off,
            Value::operand(left),
            Value::integer(imm),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, STORE_MUL_FP_IMM);
        assert_eq!(i.op0(), Some(0));
        let inv = M31::from(imm).inverse().0 as i32;
        assert_eq!(i.op1(), Some(inv));
        assert_eq!(i.op2(), Some(dest_off));
    }

    #[test]
    fn test_felt_div_by_zero_immediate_errors() {
        let (mut b, left) = mk_builder_with_left_at(0);
        let err = b.felt_arith(BinaryOp::Div, 3, Value::operand(left), Value::integer(0));
        assert!(err.is_err());
    }

    #[test]
    fn test_felt_add_immediate_left_normalized() {
        let (mut b, right) = mk_builder_with_left_at(0);
        let dest_off = 11;
        let imm = 9u32;
        b.felt_arith(
            BinaryOp::Add,
            dest_off,
            Value::integer(imm),
            Value::operand(right),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, STORE_ADD_FP_IMM);
        assert_eq!(i.op0(), Some(0)); // right at fp+0
        assert_eq!(i.op1(), Some(imm as i32));
        assert_eq!(i.op2(), Some(dest_off));
    }

    #[test]
    fn test_felt_mul_immediate_left_normalized() {
        let (mut b, right) = mk_builder_with_left_at(0);
        let dest_off = 12;
        let imm = 7u32;
        b.felt_arith(
            BinaryOp::Mul,
            dest_off,
            Value::integer(imm),
            Value::operand(right),
        )
        .unwrap();
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(i.opcode, STORE_MUL_FP_IMM);
        assert_eq!(i.op0(), Some(0));
        assert_eq!(i.op1(), Some(imm as i32));
        assert_eq!(i.op2(), Some(dest_off));
    }
}
