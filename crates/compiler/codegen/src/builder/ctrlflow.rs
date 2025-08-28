//! Control-flow lowering templates for JNZ/JMP sequencing and short-circuit.

use crate::{CodegenError, CodegenResult, InstructionBuilder, Operand};
use cairo_m_common::instruction::{JMP_ABS_IMM, JNZ_FP_IMM};
use cairo_m_compiler_mir::{Literal, Value};

impl super::CasmBuilder {
    /// Generates an unconditional jump to a label.
    pub(crate) fn jump(&mut self, target_label: &str) {
        let instr = InstructionBuilder::new(JMP_ABS_IMM)
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("jump abs {target_label}"));

        self.emit_push(instr);
    }

    /// Generates a conditional jump instruction that triggers if the value at `cond_off` is non-zero.
    /// The `target_label` is a placeholder that will be resolved to a relative offset later.
    pub(crate) fn jnz(&mut self, condition: Value, target_label: &str) -> CodegenResult<()> {
        // Get the condition value offset
        let cond_off = match condition {
            Value::Operand(cond_id) => self.layout.get_offset(cond_id)?,
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Condition must be a value operand".to_string(),
                ));
            }
        };

        self.jnz_offset(cond_off, target_label);
        Ok(())
    }

    /// Generates a conditional jump based on a direct fp-relative offset.
    pub(crate) fn jnz_offset(&mut self, cond_off: i32, target_label: &str) {
        let instr = InstructionBuilder::new(JNZ_FP_IMM)
            .with_operand(Operand::Literal(cond_off))
            .with_operand(Operand::Label(target_label.to_string()))
            .with_comment(format!("if [fp + {cond_off}] != 0 jmp rel {target_label}"));

        self.emit_push(instr);
    }

    /// Short-circuit OR: dest = (left != 0) || (right != 0)
    pub(super) fn sc_or(
        &mut self,
        dest_off: i32,
        left: &Value,
        right: &Value,
    ) -> CodegenResult<()> {
        // Initialize result to 0
        self.store_immediate(0, dest_off, "Initialize OR result to 0".to_string());
        let set_true = self.new_label_name("or_true");
        let end = self.new_label_name("or_end");

        // Check left and right
        self.branch_if_nonzero_to(left, &set_true, true)?;
        self.branch_if_nonzero_to(right, &set_true, true)?;

        // End sequence
        self.jump(&end);
        self.add_label(crate::Label::new(set_true));
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        self.add_label(crate::Label::new(end));
        Ok(())
    }

    /// Short-circuit AND: dest = (left != 0) && (right != 0)
    pub(super) fn sc_and(
        &mut self,
        dest_off: i32,
        left: &Value,
        right: &Value,
    ) -> CodegenResult<()> {
        // Default to 0
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        let check_right = self.new_label_name("and_check_right");
        let set_true = self.new_label_name("and_true");
        let end = self.new_label_name("and_end");

        // Check left: if zero -> end; else -> check right
        match Self::const_truthiness(left) {
            Some(false) => {
                self.add_label(crate::Label::new(end.clone()));
                return Ok(());
            }
            Some(true) => { /* fallthrough to check_right */ }
            None => {
                // Dynamic: emit jnz to check_right else jump end
                if let Value::Operand(id) = left {
                    let off = self.layout.get_offset(*id)?;
                    self.jnz_offset(off, &check_right);
                    self.jump(&end);
                } else {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Unsupported left in AND".into(),
                    ));
                }
            }
        }

        // check_right label
        self.add_label(crate::Label::new(check_right));
        // Right: branch to set_true on non-zero. If const true, emit JMP; const false, no-op.
        self.branch_if_nonzero_to(right, &set_true, true)?;

        // Done: both were zero at some point
        self.jump(&end);
        // set_true: set dest 1
        self.add_label(crate::Label::new(set_true));
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // end
        self.add_label(crate::Label::new(end));
        Ok(())
    }

    /// NOT: dest = ([source] == 0)
    pub(super) fn sc_not(&mut self, dest_off: i32, source: &Value) -> CodegenResult<()> {
        let set_zero = self.new_label_name("not_zero");
        let end = self.new_label_name("not_end");
        match source {
            Value::Operand(id) => {
                let off = self.layout.get_offset(*id)?;
                self.jnz_offset(off, &set_zero);
            }
            Value::Literal(Literal::Boolean(b)) => {
                self.store_immediate((!b) as u32, dest_off, format!("[fp + {dest_off}] = {}", !b));
                return Ok(());
            }
            Value::Literal(Literal::Integer(v)) => {
                self.store_immediate(
                    (*v == 0) as u32,
                    dest_off,
                    format!("[fp + {dest_off}] = {}", *v == 0),
                );
                return Ok(());
            }
            _ => {
                return Err(CodegenError::UnsupportedInstruction(
                    "Unsupported NOT source".into(),
                ))
            }
        }
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        self.jump(&end);
        self.add_label(crate::Label::new(set_zero));
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        self.add_label(crate::Label::new(end));
        Ok(())
    }

    /// Emits a branch to `label` if `value` is non-zero.
    /// - When `value` is an operand, emits a JNZ.
    /// - When `value` is a literal and truthy, emits an unconditional JMP if `emit_jmp_if_const_true` is true.
    /// - Returns Some(true/false) when `value` is a constant; None when dynamic.
    fn branch_if_nonzero_to(
        &mut self,
        value: &Value,
        label: &str,
        emit_jmp_if_const_true: bool,
    ) -> CodegenResult<Option<bool>> {
        if let Some(t) = Self::const_truthiness(value) {
            if t && emit_jmp_if_const_true {
                self.jump(label);
            }
            return Ok(Some(t));
        }
        match value {
            Value::Operand(id) => {
                let off = self.layout.get_offset(*id)?;
                self.jnz_offset(off, label);
                Ok(None)
            }
            _ => Err(CodegenError::UnsupportedInstruction(
                "Unsupported value in branch".into(),
            )),
        }
    }

    const fn const_truthiness(v: &Value) -> Option<bool> {
        match v {
            Value::Literal(Literal::Integer(i)) => Some(*i != 0),
            Value::Literal(Literal::Boolean(b)) => Some(*b),
            _ => None,
        }
    }

    // Removed local emit_jnz_to_label and emit_jmp_to_label; use builder's jnz/jump instead.
}

#[cfg(test)]
mod tests {
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::instruction::{JMP_ABS_IMM, JNZ_FP_IMM, STORE_IMM};
    use cairo_m_compiler_mir::{Value, ValueId};

    fn mk_builder_with_args() -> (CasmBuilder, ValueId, ValueId) {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        let b = ValueId::from_raw(2);
        layout.allocate_value(a, 1).unwrap();
        layout.allocate_value(b, 1).unwrap();
        (CasmBuilder::new(layout, 0), a, b)
    }

    #[test]
    fn test_sc_or_operands() {
        let (mut b, a, c) = mk_builder_with_args();
        let dest = 5;
        b.sc_or(dest, &Value::operand(a), &Value::operand(c))
            .unwrap();
        let ins = &b.instructions;
        assert!(ins.len() >= 5);
        assert_eq!(ins[0].opcode, STORE_IMM);
        assert_eq!(ins[0].op0(), Some(0));
        assert_eq!(ins[0].op1(), Some(dest));
        assert_eq!(ins[1].opcode, JNZ_FP_IMM);
        assert_eq!(ins[1].op0(), Some(0));
        assert_eq!(ins[2].opcode, JNZ_FP_IMM);
        assert_eq!(ins[2].op0(), Some(1));
        assert_eq!(ins[3].opcode, JMP_ABS_IMM);
        assert_eq!(ins[4].opcode, STORE_IMM);
        assert_eq!(ins[4].op0(), Some(1));
        assert_eq!(ins[4].op1(), Some(dest));
        let names: Vec<_> = b.labels().iter().map(|l| l.name.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("or_true_")));
        assert!(names.iter().any(|n| n.starts_with("or_end_")));
    }

    #[test]
    fn test_sc_and_operands() {
        let (mut b, a, c) = mk_builder_with_args();
        let dest = 7;
        b.sc_and(dest, &Value::operand(a), &Value::operand(c))
            .unwrap();
        let ins = &b.instructions;
        assert_eq!(ins[0].opcode, STORE_IMM);
        assert_eq!(ins[1].opcode, JNZ_FP_IMM);
        assert_eq!(ins[2].opcode, JMP_ABS_IMM);
        assert!(ins[3].opcode == JNZ_FP_IMM || ins[3].opcode == JMP_ABS_IMM);
        assert_eq!(ins[4].opcode, JMP_ABS_IMM);
        let last_store = ins.iter().rposition(|i| i.opcode == STORE_IMM).unwrap();
        assert_eq!(ins[last_store].op0(), Some(1));
        assert_eq!(ins[last_store].op1(), Some(dest));
        let names: Vec<_> = b.labels().iter().map(|l| l.name.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("and_check_right_")));
        assert!(names.iter().any(|n| n.starts_with("and_true_")));
        assert!(names.iter().any(|n| n.starts_with("and_end_")));
    }

    #[test]
    fn test_sc_not_operand_and_immediate() {
        let (mut b, a, _) = mk_builder_with_args();
        let dest = 9;
        b.sc_not(dest, &Value::operand(a)).unwrap();
        let ins = &b.instructions;
        assert!(ins.len() >= 4);
        assert_eq!(ins[0].opcode, JNZ_FP_IMM);
        assert_eq!(ins[1].opcode, STORE_IMM);
        assert_eq!(ins[1].op0(), Some(1));
        assert_eq!(ins[1].op1(), Some(dest));
        assert_eq!(ins[2].opcode, JMP_ABS_IMM);
        let zero_store = ins
            .iter()
            .rposition(|i| i.opcode == STORE_IMM && i.op0() == Some(0) && i.op1() == Some(dest))
            .unwrap();
        assert!(zero_store > 2);

        let (mut b2, _, _) = mk_builder_with_args();
        b2.sc_not(dest, &Value::boolean(true)).unwrap();
        assert_eq!(b2.instructions.len(), 1);
        assert_eq!(b2.instructions[0].opcode, STORE_IMM);
        assert_eq!(b2.instructions[0].op0(), Some(0));
        assert_eq!(b2.instructions[0].op1(), Some(dest));
    }
}
