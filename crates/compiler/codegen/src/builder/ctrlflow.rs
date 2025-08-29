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
        let set_true = self.emit_new_label_name("or_true");
        let end = self.emit_new_label_name("or_end");

        // Check left and right
        self.branch_if_nonzero_to(left, &set_true, true)?;
        self.branch_if_nonzero_to(right, &set_true, true)?;

        // End sequence
        self.jump(&end);
        self.emit_add_label(crate::Label::new(set_true));
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));
        self.emit_add_label(crate::Label::new(end));
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
        let check_right = self.emit_new_label_name("and_check_right");
        let set_true = self.emit_new_label_name("and_true");
        let end = self.emit_new_label_name("and_end");

        // Check left: if zero -> end; else -> check right
        match Self::const_truthiness(left) {
            Some(false) => {
                self.emit_add_label(crate::Label::new(end.clone()));
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
        self.emit_add_label(crate::Label::new(check_right));
        // Right: branch to set_true on non-zero. If const true, emit JMP; const false, no-op.
        self.branch_if_nonzero_to(right, &set_true, true)?;

        // Done: both were zero at some point
        self.jump(&end);
        // set_true: set dest 1
        self.emit_add_label(crate::Label::new(set_true));
        self.store_immediate(1, dest_off, format!("[fp + {dest_off}] = 1"));

        // end
        self.emit_add_label(crate::Label::new(end));
        Ok(())
    }

    /// NOT: dest = ([source] == 0)
    pub(super) fn sc_not(&mut self, dest_off: i32, source: &Value) -> CodegenResult<()> {
        let set_zero = self.emit_new_label_name("not_zero");
        let end = self.emit_new_label_name("not_end");
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
        self.emit_add_label(crate::Label::new(set_zero));
        self.store_immediate(0, dest_off, format!("[fp + {dest_off}] = 0"));
        self.emit_add_label(crate::Label::new(end));
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
    use crate::Operand;
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::instruction::{JMP_ABS_IMM, JNZ_FP_IMM};
    use cairo_m_compiler_mir::{Value, ValueId};

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

    // =========================================================================
    // Basic Control Flow Operations
    // =========================================================================

    #[test]
    fn test_jump_generates_correct_instruction() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);

        b.jump("my_label");

        assert_eq!(b.instructions.len(), 1);
        assert_eq!(b.instructions[0].opcode, JMP_ABS_IMM);
        assert_eq!(
            b.instructions[0].operands[0],
            Operand::Label("my_label".to_string())
        );
    }
    #[test]
    fn test_jnz_with_operand() {
        let (mut b, a) = mk_builder_with_value(1);

        b.jnz(Value::operand(a), "target_label").unwrap();

        // Should have store_imm + jnz
        let jnz_instr = b.instructions.iter().find(|i| i.opcode == JNZ_FP_IMM);
        assert!(jnz_instr.is_some());
        assert_eq!(
            jnz_instr.unwrap().operands[1],
            Operand::Label("target_label".to_string())
        );
    }

    #[test]
    fn test_jnz_with_literal_fails() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);

        let result = b.jnz(Value::integer(1), "target_label");
        assert!(result.is_err());
    }

    #[test]
    fn test_branch_if_nonzero_const_true() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);

        let result = b
            .branch_if_nonzero_to(&Value::integer(1), "label", true)
            .unwrap();
        assert_eq!(result, Some(true));
        // Should generate unconditional jump
        assert_eq!(b.instructions[0].opcode, JMP_ABS_IMM);
    }

    #[test]
    fn test_branch_if_nonzero_const_false() {
        let layout = FunctionLayout::new_for_test();
        let mut b = CasmBuilder::new(layout, 0);

        let result = b
            .branch_if_nonzero_to(&Value::integer(0), "label", true)
            .unwrap();
        assert_eq!(result, Some(false));
        // Should not generate any instruction
        assert_eq!(b.instructions.len(), 0);
    }

    #[test]
    fn test_branch_if_nonzero_operand() {
        let (mut b, a) = mk_builder_with_value(0);

        let result = b
            .branch_if_nonzero_to(&Value::operand(a), "label", true)
            .unwrap();
        assert_eq!(result, None); // Dynamic value
                                  // Should generate conditional jump (after the store_imm)
        assert_eq!(b.instructions[1].opcode, JNZ_FP_IMM);
    }
}
