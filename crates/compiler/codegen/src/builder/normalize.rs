//! Normalization rules for operands and derived operations.
//!
//! Canonicalizes shapes and records transformations (swap/complement/bias) so
//! selection and emission remain simple and uniform.

use cairo_m_compiler_mir::{BinaryOp, Literal, Value};

#[derive(Clone, Copy, Debug)]
pub struct CmpNorm {
    pub op: BinaryOp,
    pub swap: bool,
    pub complement: bool,
}

pub const fn normalize_u32_cmp_fp_fp(op: BinaryOp) -> CmpNorm {
    match op {
        BinaryOp::U32Neq => CmpNorm {
            op: BinaryOp::U32Eq,
            swap: false,
            complement: true,
        },
        BinaryOp::U32Greater => CmpNorm {
            op: BinaryOp::U32Less,
            swap: true,
            complement: false,
        },
        BinaryOp::U32GreaterEqual => CmpNorm {
            op: BinaryOp::U32Less,
            swap: false,
            complement: true,
        },
        BinaryOp::U32LessEqual => CmpNorm {
            op: BinaryOp::U32Less,
            swap: true,
            complement: true,
        },
        _ => CmpNorm {
            op,
            swap: false,
            complement: false,
        },
    }
}

// For fp-imm comparisons, some ops bias the immediate. Caller handles boundary cases.
#[derive(Clone, Copy, Debug)]
pub struct ImmNorm {
    pub op: BinaryOp,
    pub complement: bool,
    pub biased_imm: Option<u32>,
}

pub const fn normalize_u32_cmp_fp_imm(op: BinaryOp, imm: u32) -> ImmNorm {
    match op {
        BinaryOp::U32Neq => ImmNorm {
            op: BinaryOp::U32Eq,
            complement: true,
            biased_imm: None,
        },
        BinaryOp::U32Greater => {
            // x > c == 1 - (x < c+1). Caller must handle c==MAX separately.
            ImmNorm {
                op: BinaryOp::U32Less,
                complement: true,
                biased_imm: Some(imm.wrapping_add(1)),
            }
        }
        BinaryOp::U32GreaterEqual => ImmNorm {
            op: BinaryOp::U32Less,
            complement: true,
            biased_imm: None,
        },
        BinaryOp::U32Less => ImmNorm {
            op,
            complement: false,
            biased_imm: None,
        },
        BinaryOp::U32LessEqual => {
            // x <= c == x < c+1. Caller must handle c==MAX separately (always true).
            ImmNorm {
                op: BinaryOp::U32Less,
                complement: false,
                biased_imm: Some(imm.wrapping_add(1)),
            }
        }
        _ => ImmNorm {
            op,
            complement: false,
            biased_imm: None,
        },
    }
}

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
pub fn canonicalize_commutative_felt(op: BinaryOp, left: Value, right: Value) -> (Value, Value) {
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
pub fn canonicalize_commutative_u32(op: BinaryOp, left: Value, right: Value) -> (Value, Value) {
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
    fn test_u32_cmp_fp_fp_normalization() {
        // neq -> eq with complement
        let n = normalize_u32_cmp_fp_fp(BinaryOp::U32Neq);
        assert!(matches!(n.op, BinaryOp::U32Eq));
        assert!(n.complement);
        assert!(!n.swap);

        // gt -> lt with swap
        let n = normalize_u32_cmp_fp_fp(BinaryOp::U32Greater);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(n.swap);
        assert!(!n.complement);

        // ge -> lt with complement
        let n = normalize_u32_cmp_fp_fp(BinaryOp::U32GreaterEqual);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(n.complement);
        assert!(!n.swap);

        // le -> lt with swap + complement
        let n = normalize_u32_cmp_fp_fp(BinaryOp::U32LessEqual);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(n.swap);
        assert!(n.complement);
    }

    #[test]
    fn test_u32_cmp_fp_imm_normalization_bias_and_complement() {
        // neq: complement, no bias
        let n = normalize_u32_cmp_fp_imm(BinaryOp::U32Neq, 123);
        assert!(matches!(n.op, BinaryOp::U32Eq));
        assert!(n.complement);
        assert!(n.biased_imm.is_none());

        // gt: complement + bias imm+1
        let n = normalize_u32_cmp_fp_imm(BinaryOp::U32Greater, 7);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(n.complement);
        assert_eq!(n.biased_imm, Some(8));

        // le: bias imm+1, no complement
        let n = normalize_u32_cmp_fp_imm(BinaryOp::U32LessEqual, 9);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(!n.complement);
        assert_eq!(n.biased_imm, Some(10));

        // ge: complement, no bias
        let n = normalize_u32_cmp_fp_imm(BinaryOp::U32GreaterEqual, 42);
        assert!(matches!(n.op, BinaryOp::U32Less));
        assert!(n.complement);
        assert!(n.biased_imm.is_none());

        // Edge: bias wraps for MAX (caller handles boundary semantics)
        let n = normalize_u32_cmp_fp_imm(BinaryOp::U32LessEqual, 0xFFFF_FFFF);
        assert_eq!(n.biased_imm, Some(0));
    }
}
