//! Legalization pass: rewrite MIR into a VM-legal subset expected by CASM builder.
//!
//! Goals:
//! - Normalize u32 comparisons to a canonical set supported by the VM: `U32Eq` and strict `U32Less`.
//! - Expand unsupported comparisons using `UnaryOp::Not` and/or operand swaps.
//! - (Optionally) canonicalize commutative u32 ops to put immediates on the right.
//!
//! This pass runs inside `codegen` so MIR remains target-agnostic.
//!
//! Performance note: some rewrites (e.g., `<=, >=, !=`) expand to one compare
//! plus a boolean `Not`, increasing instruction count. We choose to fuse them into independent instructions
//! to reduce the amount of columns in the prover.

use cairo_m_compiler_mir::instruction::InstructionKind;
use cairo_m_compiler_mir::{BinaryOp, MirFunction, MirModule, MirType, Value};
use cairo_m_compiler_parser::parser::UnaryOp;

use crate::builder::normalize::is_commutative_u32;

/// Apply legalization to a whole module (in place).
pub fn legalize_module_for_vm(module: &mut MirModule) {
    for func in module.functions_mut() {
        legalize_function_for_vm(func);
    }
}

/// Apply legalization to a single function (in place).
pub fn legalize_function_for_vm(function: &mut MirFunction) {
    let blocks_len = function.basic_blocks.len();
    for idx in 0..blocks_len {
        let bid = cairo_m_compiler_mir::BasicBlockId::from_raw(idx);
        // Take the old instruction list out to avoid aliasing borrows while creating new values.
        let old_instrs = {
            let block = function.get_basic_block_mut(bid).expect("valid block id");
            std::mem::take(&mut block.instructions)
        };
        let mut new_instrs = Vec::with_capacity(old_instrs.len());

        for instr in old_instrs.into_iter() {
            match instr.kind {
                InstructionKind::BinaryOp {
                    op,
                    dest,
                    left,
                    right,
                } => {
                    // First, canonicalize commutative u32 ops to keep immediates on the right
                    let (op, left, right) = canonicalize_commutative_u32(op, left, right);

                    match op {
                        // Leave canonical ops as-is
                        BinaryOp::U32Eq | BinaryOp::U32Less => {
                            new_instrs.push(instr);
                        }

                        // a != b  ==>  tmp = (a == b); dest = !tmp
                        BinaryOp::U32Neq => {
                            let tmp = function.new_typed_value_id(MirType::bool());
                            new_instrs.push(cairo_m_compiler_mir::Instruction::binary_op(
                                BinaryOp::U32Eq,
                                tmp,
                                left,
                                right,
                            ));
                            new_instrs.push(cairo_m_compiler_mir::Instruction::unary_op(
                                UnaryOp::Not,
                                dest,
                                Value::operand(tmp),
                            ));
                        }

                        // a > b  ==>  (b < a)
                        BinaryOp::U32Greater => {
                            new_instrs.push(cairo_m_compiler_mir::Instruction::binary_op(
                                BinaryOp::U32Less,
                                dest,
                                right,
                                left,
                            ));
                        }

                        // a >= b  ==>  tmp = (a < b); dest = !tmp
                        BinaryOp::U32GreaterEqual => {
                            let tmp = function.new_typed_value_id(MirType::bool());
                            new_instrs.push(cairo_m_compiler_mir::Instruction::binary_op(
                                BinaryOp::U32Less,
                                tmp,
                                left,
                                right,
                            ));
                            new_instrs.push(cairo_m_compiler_mir::Instruction::unary_op(
                                UnaryOp::Not,
                                dest,
                                Value::operand(tmp),
                            ));
                        }

                        // a <= b  ==>  tmp = (b < a); dest = !tmp
                        BinaryOp::U32LessEqual => {
                            let tmp = function.new_typed_value_id(MirType::bool());
                            new_instrs.push(cairo_m_compiler_mir::Instruction::binary_op(
                                BinaryOp::U32Less,
                                tmp,
                                right,
                                left,
                            ));
                            new_instrs.push(cairo_m_compiler_mir::Instruction::unary_op(
                                UnaryOp::Not,
                                dest,
                                Value::operand(tmp),
                            ));
                        }

                        // Felt comparisons and others: keep as-is (codegen will reject unsupported felt cmps)
                        _ => {
                            new_instrs.push(instr);
                        }
                    }
                }
                _ => new_instrs.push(instr),
            }
        }

        // Put the rewritten list back.
        let block = function.get_basic_block_mut(bid).expect("valid block id");
        block.instructions = new_instrs;
    }
}

const fn canonicalize_commutative_u32(
    op: BinaryOp,
    left: Value,
    right: Value,
) -> (BinaryOp, Value, Value) {
    if !is_commutative_u32(op) {
        return (op, left, right);
    }
    match (&left, &right) {
        (Value::Literal(_), Value::Operand(_)) => (op, right, left),
        _ => (op, left, right),
    }
}

#[cfg(test)]
mod tests {
    use cairo_m_compiler_mir::instruction::Instruction;

    use super::*;

    fn mk_simple_fn() -> MirFunction {
        let mut f = MirFunction::new("test".to_string());
        // ensure at least one block exists
        let bid = f.entry_block;
        // a, b, dest
        let a = f.new_typed_value_id(MirType::u32());
        let b = f.new_typed_value_id(MirType::u32());
        let d = f.new_typed_value_id(MirType::bool());

        f.get_basic_block_mut(bid).unwrap().instructions = vec![Instruction::binary_op(
            BinaryOp::U32GreaterEqual,
            d,
            Value::operand(a),
            Value::operand(b),
        )];
        f
    }

    #[test]
    fn rewrites_ge_to_not_lt() {
        let mut f = mk_simple_fn();
        legalize_function_for_vm(&mut f);
        let insts = &f.basic_blocks[f.entry_block].instructions;
        assert_eq!(insts.len(), 2);
        match &insts[0].kind {
            InstructionKind::BinaryOp { op, .. } => assert!(matches!(op, BinaryOp::U32Less)),
            _ => panic!(),
        }
        match &insts[1].kind {
            InstructionKind::UnaryOp { op, .. } => assert!(matches!(op, UnaryOp::Not)),
            _ => panic!(),
        }
    }

    #[test]
    fn rewrites_gt_to_swapped_lt() {
        let mut f = MirFunction::new("gt".to_string());
        let bid = f.entry_block;
        let a = f.new_typed_value_id(MirType::u32());
        let b = f.new_typed_value_id(MirType::u32());
        let d = f.new_typed_value_id(MirType::bool());
        f.get_basic_block_mut(bid).unwrap().instructions = vec![Instruction::binary_op(
            BinaryOp::U32Greater,
            d,
            Value::operand(a),
            Value::operand(b),
        )];
        legalize_function_for_vm(&mut f);
        let insts = &f.basic_blocks[bid].instructions;
        assert_eq!(insts.len(), 1);
        match &insts[0].kind {
            InstructionKind::BinaryOp {
                op, left, right, ..
            } => {
                assert!(matches!(op, BinaryOp::U32Less));
                assert_eq!(left.as_operand(), Some(b));
                assert_eq!(right.as_operand(), Some(a));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn rewrites_neq_to_not_eq() {
        let mut f = MirFunction::new("neq".to_string());
        let bid = f.entry_block;
        let a = f.new_typed_value_id(MirType::u32());
        let b = f.new_typed_value_id(MirType::u32());
        let d = f.new_typed_value_id(MirType::bool());
        f.get_basic_block_mut(bid).unwrap().instructions = vec![Instruction::binary_op(
            BinaryOp::U32Neq,
            d,
            Value::operand(a),
            Value::operand(b),
        )];
        legalize_function_for_vm(&mut f);
        let insts = &f.basic_blocks[bid].instructions;
        assert_eq!(insts.len(), 2);
        match &insts[0].kind {
            InstructionKind::BinaryOp { op, .. } => assert!(matches!(op, BinaryOp::U32Eq)),
            _ => panic!(),
        }
        match &insts[1].kind {
            InstructionKind::UnaryOp { op, .. } => assert!(matches!(op, UnaryOp::Not)),
            _ => panic!(),
        }
    }

    #[test]
    fn rewrites_le_to_not_swapped_lt() {
        let mut f = MirFunction::new("le".to_string());
        let bid = f.entry_block;
        let a = f.new_typed_value_id(MirType::u32());
        let b = f.new_typed_value_id(MirType::u32());
        let d = f.new_typed_value_id(MirType::bool());
        f.get_basic_block_mut(bid).unwrap().instructions = vec![Instruction::binary_op(
            BinaryOp::U32LessEqual,
            d,
            Value::operand(a),
            Value::operand(b),
        )];
        legalize_function_for_vm(&mut f);
        let insts = &f.basic_blocks[bid].instructions;
        assert_eq!(insts.len(), 2);
        match &insts[0].kind {
            InstructionKind::BinaryOp {
                op, left, right, ..
            } => {
                assert!(matches!(op, BinaryOp::U32Less));
                assert_eq!(left.as_operand(), Some(b));
                assert_eq!(right.as_operand(), Some(a));
            }
            _ => panic!(),
        }
        match &insts[1].kind {
            InstructionKind::UnaryOp { op, .. } => assert!(matches!(op, UnaryOp::Not)),
            _ => panic!(),
        }
    }
}
