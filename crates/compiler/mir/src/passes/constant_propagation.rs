use rustc_hash::FxHashMap;

use super::{const_eval::ConstEvaluator, MirPass};
use crate::{InstructionKind, Literal, MirFunction, Value, ValueId};

/// Lattice of constant propagation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lattice {
    /// Not yet known (may become const/non-const later)
    Unknown,
    /// Compile-time constant with this literal
    Const(Literal),
    /// Proven non-constant
    NonConst,
}

impl Lattice {
    fn join(self, other: Self) -> Self {
        use Lattice::*;
        match (self, other) {
            (NonConst, _) | (_, NonConst) => NonConst,
            (Unknown, x) | (x, Unknown) => x,
            (Const(a), Const(b)) if a == b => Const(a),
            (Const(_), Const(_)) => NonConst,
        }
    }
}

/// Constant Propagation Pass
///
/// Performs forward data-flow analysis to compute constant values for SSA
/// `ValueId`s and then rewrites uses of constant values to literals. This
/// exposes further folding opportunities for `ConstantFolding` and
/// `ArithmeticSimplify`.
#[derive(Debug, Default)]
pub struct ConstantPropagation {
    evaluator: ConstEvaluator,
}

impl ConstantPropagation {
    pub const fn new() -> Self {
        Self {
            evaluator: ConstEvaluator::new(),
        }
    }

    fn evaluate_instruction(
        &self,
        instr: &crate::Instruction,
        state: &FxHashMap<ValueId, Lattice>,
    ) -> Option<(ValueId, Lattice)> {
        use InstructionKind as K;
        use Lattice::*;

        let dest = instr.destination()?;
        let lattice = match &instr.kind {
            K::Assign { source, .. } => match source {
                Value::Literal(l) => Const(*l),
                Value::Operand(id) => *state.get(id).unwrap_or(&Unknown),
                Value::Error => NonConst,
            },

            K::UnaryOp { op, source, .. } => {
                let lit = match source {
                    Value::Literal(l) => Some(*l),
                    Value::Operand(id) => match state.get(id).copied().unwrap_or(Unknown) {
                        Const(l) => Some(l),
                        NonConst => None,
                        Unknown => return Some((dest, Unknown)),
                    },
                    Value::Error => None,
                };

                match lit.and_then(|l| self.evaluator.eval_unary_op(*op, l)) {
                    Some(l) => Const(l),
                    None => NonConst,
                }
            }

            K::BinaryOp {
                op, left, right, ..
            } => {
                // Resolve left literal if available
                let l_lit = match left {
                    Value::Literal(l) => Some(*l),
                    Value::Operand(id) => match state.get(id).copied().unwrap_or(Unknown) {
                        Const(l) => Some(l),
                        NonConst => None,
                        Unknown => None,
                    },
                    Value::Error => None,
                };
                // Resolve right literal if available
                let r_lit = match right {
                    Value::Literal(l) => Some(*l),
                    Value::Operand(id) => match state.get(id).copied().unwrap_or(Unknown) {
                        Const(l) => Some(l),
                        NonConst => None,
                        Unknown => None,
                    },
                    Value::Error => None,
                };

                match (l_lit, r_lit) {
                    (Some(a), Some(b)) => match self.evaluator.eval_binary_op(*op, a, b) {
                        Some(res) => Const(res),
                        None => NonConst,
                    },
                    (None, _) | (_, None) => {
                        // If either side is definitely NonConst, result is NonConst.
                        let left_nc = matches!(left, Value::Operand(id) if matches!(state.get(id), Some(NonConst)));
                        let right_nc = matches!(right, Value::Operand(id) if matches!(state.get(id), Some(NonConst)));
                        if left_nc || right_nc {
                            NonConst
                        } else {
                            Unknown
                        }
                    }
                }
            }

            // Phi: join over incoming values
            K::Phi { sources, .. } => {
                let mut acc = Unknown;
                for (_bb, val) in sources {
                    let l = match val {
                        Value::Literal(l) => Lattice::Const(*l),
                        Value::Operand(id) => *state.get(id).unwrap_or(&Unknown),
                        Value::Error => NonConst,
                    };
                    acc = acc.join(l);
                    if acc == NonConst {
                        break;
                    }
                }
                acc
            }

            // Pure constructions without a literal representation in `Literal`
            K::MakeTuple { .. }
            | K::ExtractTupleElement { .. }
            | K::MakeStruct { .. }
            | K::ExtractStructField { .. }
            | K::InsertField { .. }
            | K::InsertTuple { .. }
            | K::MakeFixedArray { .. }
            | K::ArrayIndex { .. }
            | K::ArrayInsert { .. }
            | K::Cast { .. }
            | K::Call { .. }
            | K::Debug { .. }
            | K::Nop
            | K::AssertEq { .. } => NonConst,
        };

        Some((dest, lattice))
    }

    /// After analysis, rewrite uses of constants to literals across the function
    fn rewrite_uses(
        &self,
        function: &mut MirFunction,
        state: &FxHashMap<ValueId, Lattice>,
    ) -> bool {
        let mut modified = false;

        // Helper to replace a single value if it refers to a constant operand
        let mut replace_value = |val: &mut Value| {
            if let Value::Operand(id) = val {
                if let Some(Lattice::Const(lit)) = state.get(id) {
                    *val = Value::Literal(*lit);
                    modified = true;
                }
            }
        };

        for block in function.basic_blocks.iter_mut() {
            for instr in &mut block.instructions {
                match &mut instr.kind {
                    InstructionKind::Assign { source, .. } => replace_value(source),
                    InstructionKind::UnaryOp { source, .. } => replace_value(source),
                    InstructionKind::BinaryOp { left, right, .. } => {
                        replace_value(left);
                        replace_value(right);
                    }
                    InstructionKind::Call { args, .. } => {
                        for a in args {
                            replace_value(a);
                        }
                    }
                    InstructionKind::Cast { source, .. } => replace_value(source),
                    InstructionKind::Debug { values, .. } => {
                        for v in values {
                            replace_value(v);
                        }
                    }
                    InstructionKind::Phi { sources, .. } => {
                        for (_, v) in sources {
                            replace_value(v);
                        }
                    }
                    InstructionKind::MakeTuple { elements, .. } => {
                        for e in elements {
                            replace_value(e);
                        }
                    }
                    InstructionKind::ExtractTupleElement { tuple, .. } => replace_value(tuple),
                    InstructionKind::MakeStruct { fields, .. } => {
                        for (_, v) in fields {
                            replace_value(v);
                        }
                    }
                    InstructionKind::ExtractStructField { struct_val, .. } => {
                        replace_value(struct_val)
                    }
                    InstructionKind::InsertField {
                        struct_val,
                        new_value,
                        ..
                    } => {
                        replace_value(struct_val);
                        replace_value(new_value);
                    }
                    InstructionKind::InsertTuple {
                        tuple_val,
                        new_value,
                        ..
                    } => {
                        replace_value(tuple_val);
                        replace_value(new_value);
                    }
                    InstructionKind::MakeFixedArray { elements, .. } => {
                        for e in elements {
                            replace_value(e);
                        }
                    }
                    InstructionKind::ArrayIndex { array, index, .. } => {
                        replace_value(array);
                        replace_value(index);
                    }
                    InstructionKind::ArrayInsert {
                        array_val,
                        index,
                        new_value,
                        ..
                    } => {
                        replace_value(array_val);
                        replace_value(index);
                        replace_value(new_value);
                    }
                    InstructionKind::AssertEq { left, right } => {
                        replace_value(left);
                        replace_value(right);
                    }
                    InstructionKind::Nop => {}
                }
            }

            // Note: We intentionally do NOT rewrite values inside terminators
            // here. Branch conditions becoming literals should be handled by
            // dedicated control-flow passes (e.g., SimplifyBranches) after
            // folding at instruction level, reducing chances of creating
            // dangling uses during aggressive propagation.
        }

        modified
    }
}

impl MirPass for ConstantPropagation {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        use Lattice::*;

        // Initialize lattice state for all known values
        let mut state: FxHashMap<ValueId, Lattice> = FxHashMap::default();
        for id in function.value_types.keys() {
            state.insert(*id, Unknown);
        }
        // Parameters are non-constant (unknown at compile time)
        for param in &function.parameters {
            state.insert(*param, NonConst);
        }

        // Worklist: iterate until no changes
        let mut changed = true;
        while changed {
            changed = false;
            for (_bb_id, block) in function.basic_blocks() {
                for instr in &block.instructions {
                    if let Some((dest, new_lat)) = self.evaluate_instruction(instr, &state) {
                        let old = state.get(&dest).copied().unwrap_or(Unknown);
                        let next = match (old, new_lat) {
                            (Unknown, x) => x,
                            (Const(a), Const(b)) if a == b => Const(a),
                            (Const(_), Const(_)) => NonConst, // conflicting constants
                            (Const(a), Unknown) => Const(a),  // keep existing knowledge
                            (NonConst, _) => NonConst,
                            (_, NonConst) => NonConst,
                        };
                        if next != old {
                            state.insert(dest, next);
                            changed = true;
                        }
                    }
                }
            }
        }

        // After convergence, rewrite uses of constants to literals

        self.rewrite_uses(function, &state)
    }

    fn name(&self) -> &'static str {
        "ConstantPropagation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOp, Instruction, MirType, Terminator};

    #[test]
    fn test_propagate_into_binary_then_fold() {
        // %a = 1
        // %b = 2
        // %c = %a + %b  => should become c = 3 after const fold
        // return %c
        let mut f = MirFunction::new("test".to_string());
        let entry = f.add_basic_block();
        f.entry_block = entry;

        let a = f.new_typed_value_id(MirType::u32());
        let b = f.new_typed_value_id(MirType::u32());
        let c = f.new_typed_value_id(MirType::u32());

        let block = f.get_basic_block_mut(entry).unwrap();
        block.push_instruction(Instruction::assign(a, Value::integer(1), MirType::u32()));
        block.push_instruction(Instruction::assign(b, Value::integer(2), MirType::u32()));
        block.push_instruction(Instruction::binary_op(
            BinaryOp::U32Add,
            c,
            Value::operand(a),
            Value::operand(b),
        ));
        block.set_terminator(Terminator::return_value(Value::operand(c)));

        // Run propagation only
        let mut cp = ConstantPropagation::new();
        let modified = cp.run(&mut f);
        assert!(modified);

        // The add should now have literal operands
        let block = f.get_basic_block(entry).unwrap();
        if let InstructionKind::BinaryOp { left, right, .. } = &block.instructions[2].kind {
            assert_eq!(*left, Value::integer(1));
            assert_eq!(*right, Value::integer(2));
        } else {
            panic!("Expected binary op");
        }
    }

    #[test]
    fn test_propagate_across_blocks() {
        // entry: %a = 1; jump b1
        // b1: %c = %a + 2  => operands should become literals (1 + 2)
        let mut f = MirFunction::new("test".to_string());
        let b0 = f.add_basic_block();
        let b1 = f.add_basic_block();
        f.entry_block = b0;

        let a = f.new_typed_value_id(MirType::felt());
        let c = f.new_typed_value_id(MirType::felt());

        let block0 = f.get_basic_block_mut(b0).unwrap();
        block0.push_instruction(Instruction::assign(a, Value::integer(1), MirType::felt()));
        block0.set_terminator(Terminator::jump(b1));

        let block1 = f.get_basic_block_mut(b1).unwrap();
        block1.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            c,
            Value::operand(a),
            Value::integer(2),
        ));
        block1.set_terminator(Terminator::return_value(Value::operand(c)));

        let mut cp = ConstantPropagation::new();
        let modified = cp.run(&mut f);
        assert!(modified);

        let block1 = f.get_basic_block(b1).unwrap();
        if let InstructionKind::BinaryOp { left, right, .. } = &block1.instructions[0].kind {
            assert_eq!(*left, Value::integer(1));
            assert_eq!(*right, Value::integer(2));
        } else {
            panic!("Expected binary op in b1");
        }
    }

    #[test]
    fn test_phi_with_same_constants_becomes_constant_use() {
        // Artificial phi with identical constant sources; using it in an add should
        // see literal operand due to CP rewrite.
        let mut f = MirFunction::new("test".to_string());
        let b0 = f.add_basic_block();
        let b1 = f.add_basic_block();
        f.entry_block = b0;

        let p = f.new_typed_value_id(MirType::felt());
        let r = f.new_typed_value_id(MirType::felt());

        let block0 = f.get_basic_block_mut(b0).unwrap();
        // Create a phi in b1 with two constant sources (we don't rely on preds for this test)
        block0.set_terminator(Terminator::jump(b1));

        let block1 = f.get_basic_block_mut(b1).unwrap();
        block1.push_instruction(Instruction::phi(
            p,
            MirType::felt(),
            vec![(b0, Value::integer(5)), (b0, Value::integer(5))],
        ));
        block1.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            r,
            Value::operand(p),
            Value::integer(1),
        ));
        block1.set_terminator(Terminator::return_value(Value::operand(r)));

        let mut cp = ConstantPropagation::new();
        let modified = cp.run(&mut f);
        assert!(modified);

        let block1 = f.get_basic_block(b1).unwrap();
        if let InstructionKind::BinaryOp { left, right, .. } = &block1.instructions[1].kind {
            assert_eq!(*left, Value::integer(5));
            assert_eq!(*right, Value::integer(1));
        } else {
            panic!("Expected binary op using propagated phi constant");
        }
    }

    #[test]
    fn test_phi_with_conflicting_constants_stays_unknown() {
        // Phi with conflicting constant sources should not rewrite uses to a literal.
        let mut f = MirFunction::new("test".to_string());
        let b0 = f.add_basic_block();
        let b1 = f.add_basic_block();
        f.entry_block = b0;

        let p = f.new_typed_value_id(MirType::felt());
        let r = f.new_typed_value_id(MirType::felt());

        let block0 = f.get_basic_block_mut(b0).unwrap();
        block0.set_terminator(Terminator::jump(b1));

        let block1 = f.get_basic_block_mut(b1).unwrap();
        block1.push_instruction(Instruction::phi(
            p,
            MirType::felt(),
            vec![(b0, Value::integer(5)), (b0, Value::integer(6))],
        ));
        block1.push_instruction(Instruction::binary_op(
            BinaryOp::Add,
            r,
            Value::operand(p),
            Value::integer(1),
        ));
        block1.set_terminator(Terminator::return_value(Value::operand(r)));

        let mut cp = ConstantPropagation::new();
        let _modified = cp.run(&mut f);
        // We specifically assert that the use of the phi result is NOT rewritten
        // to a literal when phi sources conflict, i.e., the left operand remains
        // an operand reference rather than a literal.

        let block1 = f.get_basic_block(b1).unwrap();
        if let InstructionKind::BinaryOp { left, .. } = &block1.instructions[1].kind {
            // Left should remain an operand (not a literal 5/6)
            assert!(matches!(left, Value::Operand(_)));
        } else {
            panic!("Expected binary op using phi result");
        }
    }
}
