//! Function call handling: argument passing and in-place detection.

use crate::{CodegenError, CodegenResult, InstructionBuilder};
use cairo_m_common::Instruction as CasmInstr;
use cairo_m_compiler_mir::{
    instruction::CalleeSignature, DataLayout, Literal, MirType, Value, ValueId,
};
use stwo_prover::core::fields::m31::M31;

impl super::CasmBuilder {
    /// Shared lowering for all call flavors (void, single, multiple).
    pub(crate) fn lower_call(
        &mut self,
        callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
        dests: &[ValueId],
    ) -> CodegenResult<()> {
        let args_offset = self.pass_arguments(callee_name, args, signature)?;

        // Total parameter slots (m) and return slots (k)
        let m: usize = signature
            .param_types
            .iter()
            .map(DataLayout::memory_size_of)
            .sum();
        let k: usize = signature
            .return_types
            .iter()
            .map(DataLayout::memory_size_of)
            .sum();

        // Map return values (if any)
        if dests.is_empty() {
            if !signature.return_types.is_empty() {
                return Err(CodegenError::InvalidMir(
                    "void_call used with non-void signature".to_string(),
                ));
            }
        } else {
            // Map destinations sequentially starting at return area
            let mut current_offset = args_offset + m as i32;
            for (i, dest) in dests.iter().enumerate() {
                self.layout.map_value(*dest, current_offset);
                if i < signature.return_types.len() {
                    current_offset += DataLayout::memory_size_of(&signature.return_types[i]) as i32;
                }
            }
            // Reserve return slots and update written high-water mark
            self.layout.reserve_stack(k);
            let last_return_offset = args_offset + m as i32 + k as i32 - 1;
            self.max_written_offset = self.max_written_offset.max(last_return_offset);
        }

        // frame_off = args_offset + m (+ k for non-void)
        let frame_off = if dests.is_empty() {
            args_offset + m as i32
        } else {
            args_offset + m as i32 + k as i32
        };
        let instr = InstructionBuilder::new(
            CasmInstr::CallAbsImm {
                frame_off: M31::from(frame_off),
                target: M31::from(0),
            },
            Some(format!("call {callee_name}")),
        )
        .with_label(callee_name.to_string());
        self.emit_push(instr);
        Ok(())
    }

    /// Helper to pass arguments for a function call.
    ///
    /// This method implements the "Argument-in-Place" optimization that avoids unnecessary
    /// copying when arguments are already positioned correctly at the top of the stack.
    ///
    /// ## How the optimization works
    ///
    /// The optimization checks if all arguments are already contiguous at the top of the
    /// caller's stack frame. If they are, no copy instructions are generated.
    ///
    /// ### Example 1: Optimization applies (single-slot types)
    /// ```text
    /// Before call:
    /// | fp + 0 | value_a |  <- arg 0
    /// | fp + 1 | value_b |  <- arg 1
    /// | fp + 2 | value_c |  <- arg 2
    /// L = 3 (current frame size)
    ///
    /// Since args are at [L-3, L-2, L-1] = [0, 1, 2], no copies needed.
    /// Returns L - total_slots = 0
    /// ```
    ///
    /// ### Example 2: Optimization applies (multi-slot types)
    /// ```text
    /// Before call with f(u32, felt):
    /// | fp + 0 | u32_lo  |  <- arg 0 (u32, slot 0)
    /// | fp + 1 | u32_hi  |  <- arg 0 (u32, slot 1)
    /// | fp + 2 | felt_val|  <- arg 1 (felt)
    /// L = 3
    ///
    /// Args occupy slots [0-1] and [2], contiguous at stack top.
    /// Returns L - total_slots = 0
    /// ```
    ///
    /// ### Example 3: Optimization does NOT apply
    /// ```text
    /// Before call:
    /// | fp + 0 | value_a |
    /// | fp + 1 | temp    |  <- intermediate value
    /// | fp + 2 | value_b |
    /// L = 3
    ///
    /// Args at [0] and [2] are not contiguous, must copy to [3] and [4].
    /// Returns L = 3
    /// ```
    ///
    /// ## Return value
    ///
    /// Returns the starting offset where arguments begin:
    /// - If optimization applied: `L - total_arg_slots`
    /// - If optimization not applied: `L` (after copying args to [L, L+1, ...])
    pub(super) fn pass_arguments(
        &mut self,
        _callee_name: &str,
        args: &[Value],
        signature: &CalleeSignature,
    ) -> CodegenResult<i32> {
        let l = self.layout.current_frame_usage();
        let mut arg_offsets = Vec::new();
        let mut current_offset = l;
        for param_type in &signature.param_types {
            let abi_slots = DataLayout::memory_size_of(param_type) as i32;
            arg_offsets.push(current_offset);
            current_offset += abi_slots;
        }

        if args.len() != signature.param_types.len() {
            return Err(CodegenError::InvalidMir(format!(
                "Argument count mismatch: expected {}, got {}",
                signature.param_types.len(),
                args.len()
            )));
        }

        // Argument-in-place optimization
        {
            let all_operands = args.iter().all(|arg| matches!(arg, Value::Operand(_)));
            if all_operands && !args.is_empty() {
                if let Value::Operand(first_arg_id) = &args[0] {
                    if let Ok(first_offset) = self.layout.get_offset(*first_arg_id) {
                        let mut expected_offset = first_offset;
                        let mut all_args_contiguous = true;
                        for (arg, param_type) in args.iter().zip(&signature.param_types) {
                            let size = DataLayout::memory_size_of(param_type);
                            if let Value::Operand(arg_id) = arg {
                                if !self.layout.is_contiguous(*arg_id, expected_offset, size) {
                                    all_args_contiguous = false;
                                    break;
                                }
                                expected_offset += size as i32;
                            }
                        }
                        if all_args_contiguous {
                            let total_arg_size: usize = signature
                                .param_types
                                .iter()
                                .map(DataLayout::memory_size_of)
                                .sum();
                            let args_end = first_offset + total_arg_size as i32;
                            if args_end == self.layout.current_frame_usage()
                                || (self.max_written_offset >= 0
                                    && args_end == self.live_frame_usage())
                            {
                                return Ok(first_offset);
                            }
                        }
                    }
                }
            }
        }

        // Standard path: copy arguments to designated region
        for (i, (arg, param_type)) in args.iter().zip(&signature.param_types).enumerate() {
            let arg_offset = arg_offsets[i];
            let arg_size = DataLayout::memory_size_of(param_type);
            match arg {
                Value::Literal(Literal::Integer(imm)) => match param_type {
                    MirType::Bool | MirType::Felt => {
                        self.store_immediate(
                            *imm,
                            arg_offset,
                            format!("Arg {i}: [fp + {arg_offset}] = {imm}"),
                        );
                    }
                    MirType::U32 => {
                        self.store_u32_immediate(
                            *imm,
                            arg_offset,
                            format!("Arg {i}: [fp + {arg_offset}] = {imm}"),
                        );
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(format!(
                            "Unsupported literal argument type: {:?}",
                            param_type
                        )));
                    }
                },
                Value::Operand(arg_id) => {
                    let src_off = self.layout.get_offset(*arg_id)?;
                    if src_off == arg_offset
                        && self.layout.is_contiguous(*arg_id, arg_offset, arg_size)
                    {
                        continue;
                    }
                    self.copy_slots(src_off, arg_offset, arg_size, &format!("Arg {i}"));
                }
                _ => {
                    return Err(CodegenError::UnsupportedInstruction(
                        "Unsupported argument type".to_string(),
                    ));
                }
            }
        }
        Ok(l)
    }

    /// Generate `return` instruction with multiple return values.
    pub fn return_values(
        &mut self,
        values: &[Value],
        return_types: &[MirType],
    ) -> CodegenResult<()> {
        let k = self.layout.num_return_slots() as i32;

        // Store each return value in its designated slot
        let mut cumulative_slot_offset = 0;
        for (i, return_val) in values.iter().enumerate() {
            // Get the type of this return value
            let return_type = return_types.get(i).expect("Missing return type");
            // Return value starts at [fp - K - 2 + cumulative_slot_offset]
            let return_slot_offset = -(k + 2) + cumulative_slot_offset;

            // Check if the value is already in the return slot (optimization for direct returns)
            let needs_copy = match return_val {
                Value::Operand(val_id) => {
                    let current_offset = self.layout.get_offset(*val_id).unwrap_or(0);
                    current_offset != return_slot_offset
                }
                _ => true, // Literals always need to be stored
            };

            if needs_copy {
                match return_val {
                    Value::Literal(Literal::Integer(imm)) => {
                        let imm = { *imm };
                        if matches!(return_type, MirType::U32) {
                            self.store_u32_immediate(imm, return_slot_offset, format!(
                                "Return value {i}: [fp {return_slot_offset}, fp {return_slot_offset} + 1] = u32({imm})"
                            ));
                        } else {
                            self.store_immediate(
                                imm,
                                return_slot_offset,
                                format!("Return value {i}: [fp {return_slot_offset}] = {imm}"),
                            );
                        }
                    }
                    Value::Literal(Literal::Boolean(imm)) => {
                        let imm = *imm as u32;
                        if matches!(return_type, MirType::U32) {
                            self.store_u32_immediate(imm, return_slot_offset, format!(
                                "Return value {i}: [fp {return_slot_offset}, fp {return_slot_offset} + 1] = u32({imm})"
                            ));
                        } else {
                            self.store_immediate(
                                imm,
                                return_slot_offset,
                                format!("Return value {i}: [fp {return_slot_offset}] = {imm}"),
                            );
                        }
                    }
                    Value::Operand(val_id) => {
                        // Emit per-slot copies with legacy comment shape:
                        // "Return value {i} slot {slot}: [fp -N] = [fp + S] + 0"
                        let src_off = self.layout.get_offset(*val_id)?;
                        let value_size = self.layout.get_value_size(*val_id);
                        for slot in 0..value_size {
                            let dst = return_slot_offset + slot as i32;
                            let src = src_off + slot as i32;
                            let fmt_dst = if dst >= 0 {
                                format!("[fp + {dst}]")
                            } else {
                                format!("[fp {dst}]")
                            };
                            let comment = if value_size == 1 {
                                format!("Return value {i}: {fmt_dst} = [fp + {src}] + 0")
                            } else {
                                format!(
                                    "Return value {i} slot {slot}: {fmt_dst} = [fp + {src}] + 0"
                                )
                            };
                            self.store_copy_single(src, dst, comment);
                        }
                    }
                    _ => {
                        return Err(CodegenError::UnsupportedInstruction(
                            "Unsupported return value type".to_string(),
                        ));
                    }
                }

                let value_size = match return_val {
                    Value::Operand(val_id) => self.layout.get_value_size(*val_id),
                    Value::Literal(_) => {
                        if matches!(return_type, MirType::U32) {
                            2
                        } else {
                            1
                        }
                    }
                    _ => 1,
                };

                cumulative_slot_offset += value_size as i32;
            } else {
                let value_size = match return_val {
                    Value::Operand(val_id) => self.layout.get_value_size(*val_id),
                    _ => 1,
                };
                cumulative_slot_offset += value_size as i32;
            }
        }

        self.emit_push(InstructionBuilder::new(
            CasmInstr::Ret {},
            Some("return".to_string()),
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_compiler_mir::{MirType, Value, ValueId};
    use stwo_prover::core::fields::m31::M31;

    #[test]
    fn test_call_single_emits_call_and_maps_dest() {
        let mut layout = FunctionLayout::new_for_test();
        // Pre-allocate arg at top
        let a = ValueId::from_raw(1);
        layout.allocate_value(a, 1).unwrap(); // fp + 0
        let mut b = CasmBuilder::new(layout, 0);
        let sig = CalleeSignature {
            param_types: vec![MirType::Felt],
            return_types: vec![MirType::Felt],
        };
        let dest = ValueId::from_raw(99);
        b.lower_call("callee", &[Value::operand(a)], &sig, &[dest])
            .unwrap();
        // Expect one CALL
        assert_eq!(b.instructions.len(), 1);
        let i = &b.instructions[0];
        assert_eq!(
            i.inner_instr(),
            &CasmInstr::CallAbsImm {
                frame_off: M31::from(2),
                target: M31::from(0)
            }
        );
        // frame_off = args_offset + m + k; here args_offset is 0 via in-place optimization, m=1,k=1
        match i.inner_instr() {
            CasmInstr::CallAbsImm { frame_off, target } => {
                assert_eq!(*frame_off, M31::from(2));
                assert_eq!(*target, M31::from(0));
            }
            other => panic!("expected CallAbsImm, got {other:?}"),
        }
        // dest mapped to args_offset + m = 1
        let off = b.layout.get_offset(dest).unwrap();
        assert_eq!(off, 1);
    }

    #[test]
    fn test_call_multiple_emits_call_and_maps_dests() {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        let b = ValueId::from_raw(2);
        layout.allocate_value(a, 1).unwrap(); // 0
        layout.allocate_value(b, 1).unwrap(); // 1
        let mut builder = CasmBuilder::new(layout, 0);
        let sig = CalleeSignature {
            param_types: vec![MirType::Felt, MirType::Felt],
            return_types: vec![MirType::Felt, MirType::Felt],
        };
        let d0 = ValueId::from_raw(10);
        let d1 = ValueId::from_raw(11);
        builder
            .lower_call(
                "callee2",
                &[Value::operand(a), Value::operand(b)],
                &sig,
                &[d0, d1],
            )
            .unwrap();
        assert_eq!(builder.instructions.len(), 1);
        let i = &builder.instructions[0];
        // args_offset 0, m=2, k=2 => frame_off=4
        assert_eq!(
            i.inner_instr(),
            &CasmInstr::CallAbsImm {
                frame_off: M31::from(4),
                target: M31::from(0)
            }
        );
        // dests mapped to offsets 2 and 3
        assert_eq!(builder.layout.get_offset(d0).unwrap(), 2);
        assert_eq!(builder.layout.get_offset(d1).unwrap(), 3);
    }

    #[test]
    fn test_void_call() {
        let mut layout = FunctionLayout::new_for_test();
        let a = ValueId::from_raw(1);
        layout.allocate_value(a, 1).unwrap();
        let mut b = CasmBuilder::new(layout, 0);
        let sig = CalleeSignature {
            param_types: vec![MirType::Felt],
            return_types: vec![],
        };
        b.lower_call("noop", &[Value::operand(a)], &sig, &[])
            .unwrap();
        assert_eq!(b.instructions.len(), 1);
        assert_eq!(
            b.instructions[0].inner_instr(),
            &CasmInstr::CallAbsImm {
                frame_off: M31::from(1),
                target: M31::from(0)
            }
        );
    }

    #[test]
    fn test_return_values_literal_and_operand() {
        let mut layout = FunctionLayout::new_for_test();
        // Place an operand at fp + 0
        let v = ValueId::from_raw(1);
        layout.allocate_value(v, 1).unwrap();
        let mut b = CasmBuilder::new(layout, 0);

        // Return felt literal at [fp - 2] (k=0 for new_for_test)
        b.return_values(&[Value::integer(5)], &[MirType::Felt])
            .unwrap();
        // Then return operand value; produces copy STORE_ADD_FP_IMM to [fp - 2]
        b.return_values(&[Value::operand(v)], &[MirType::Felt])
            .unwrap();

        assert!(b.instructions.len() >= 3);
        // Find the last STORE_ADD_FP_IMM (copy) before the final RET
        let pos = b
            .instructions
            .iter()
            .rposition(|i| {
                i.inner_instr().opcode_value() == cairo_m_common::instruction::STORE_ADD_FP_IMM
            })
            .expect("missing STORE_ADD_FP_IMM copy");
        let copy = &b.instructions[pos];
        match copy.inner_instr() {
            CasmInstr::StoreAddFpImm {
                src_off,
                imm,
                dst_off,
            } => {
                assert_eq!(*src_off, M31::from(0));
                assert_eq!(*imm, M31::from(0));
                assert_eq!(*dst_off, M31::from(-2));
            }
            other => panic!("expected StoreAddFpImm, got {other:?}"),
        }
        // And ensure the last instruction is RET
        assert_eq!(
            b.instructions.last().unwrap().inner_instr().opcode_value(),
            cairo_m_common::instruction::RET
        );
    }
}
