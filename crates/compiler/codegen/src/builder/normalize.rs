//! Normalization helpers for operand shapes.
//!
//! Only commutative canonicalization remains: we move immediates to the RHS
//! for commutative ops so the builder can consistently use FP_IMM encodings.

use cairo_m_compiler_mir::{BinaryOp, Literal, Value};

/// Returns true if the felt binary op is commutative.
pub const fn is_commutative_felt(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Add | BinaryOp::Mul)
}

/// Returns true if the u32 binary op is commutative.
pub const fn is_commutative_u32(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::U32Add
            | BinaryOp::U32Mul
            | BinaryOp::U32BitwiseAnd
            | BinaryOp::U32BitwiseOr
            | BinaryOp::U32BitwiseXor
            | BinaryOp::U32Eq
            | BinaryOp::U32Neq
    )
}

/// If op is commutative and the shape is (imm, operand), swap to (operand, imm).
pub const fn canonicalize_commutative_felt(
    op: BinaryOp,
    left: Value,
    right: Value,
) -> (Value, Value) {
    if is_commutative_felt(op) {
        match (&left, &right) {
            (Value::Literal(Literal::Integer(_)), Value::Operand(_)) => (right, left),
            _ => (left, right),
        }
    } else {
        (left, right)
    }
}

/// If op is commutative and the shape is (imm, operand), swap to (operand, imm).
pub const fn canonicalize_commutative_u32(
    op: BinaryOp,
    left: Value,
    right: Value,
) -> (Value, Value) {
    if is_commutative_u32(op) {
        match (&left, &right) {
            (Value::Literal(Literal::Integer(_)), Value::Operand(_)) => (right, left),
            _ => (left, right),
        }
    } else {
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairo_m_compiler_mir::BinaryOp;

    #[test]
    fn commutative_felt_immediate_right() {
        let (l, r) = canonicalize_commutative_felt(
            BinaryOp::Add,
            Value::Literal(Literal::Integer(5)),
            Value::Operand(cairo_m_compiler_mir::ValueId::from_raw(1)),
        );
        assert!(matches!(l, Value::Operand(_)));
        assert!(matches!(r, Value::Literal(_)));
    }

    #[test]
    fn commutative_u32_immediate_right() {
        let (l, r) = canonicalize_commutative_u32(
            BinaryOp::U32BitwiseAnd,
            Value::Literal(Literal::Integer(0xFFFF_FFFF)),
            Value::Operand(cairo_m_compiler_mir::ValueId::from_raw(2)),
        );
        assert!(matches!(l, Value::Operand(_)));
        assert!(matches!(r, Value::Literal(_)));
    }
}
