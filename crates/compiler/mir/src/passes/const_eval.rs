/// Centralized constant evaluation module for MIR passes
///
/// This module provides a single source of truth for constant evaluation
/// to ensure consistent semantics across all optimization passes.
use cairo_m_compiler_parser::parser::UnaryOp;
use stwo_prover::core::fields::m31::M31;

use crate::{BinaryOp, Literal, Value};

/// Centralized constant evaluator with correct domain semantics
#[derive(Debug, Default)]
pub struct ConstEvaluator;

impl ConstEvaluator {
    /// Create a new constant evaluator
    pub const fn new() -> Self {
        Self
    }

    /// Evaluate a binary operation on two literal values
    ///
    /// Returns None if the operation cannot be evaluated at compile time
    /// or would result in undefined behavior (e.g., division by zero)
    pub(crate) fn eval_binary_op(
        &self,
        op: BinaryOp,
        left: Literal,
        right: Literal,
    ) -> Option<Literal> {
        match (op, left, right) {
            // Felt arithmetic - uses M31 field operations
            (BinaryOp::Add, Literal::Integer(a), Literal::Integer(b)) => {
                let a_m31 = M31::from(a);
                let b_m31 = M31::from(b);
                let result = a_m31 + b_m31;
                Some(Literal::Integer(result.0))
            }
            (BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b)) => {
                let a_m31 = M31::from(a);
                let b_m31 = M31::from(b);
                let result = a_m31 - b_m31;
                Some(Literal::Integer(result.0))
            }
            (BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b)) => {
                let a_m31 = M31::from(a);
                let b_m31 = M31::from(b);
                let result = a_m31 * b_m31;
                Some(Literal::Integer(result.0))
            }
            (BinaryOp::Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 => {
                let a_m31 = M31::from(a);
                let b_m31 = M31::from(b);
                let result = a_m31 / b_m31; // Uses modular inverse
                Some(Literal::Integer(result.0))
            }

            // Felt comparisons - compare as field elements
            (BinaryOp::Eq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a == b))
            }
            (BinaryOp::Neq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a != b))
            }
            // Note: Ordering comparisons on field elements are typically not well-defined
            // and disabled at semantic index level - thus not getting evaluated.

            // U32 arithmetic - proper unsigned 32-bit operations
            (BinaryOp::U32Add, Literal::Integer(a), Literal::Integer(b)) => {
                // Wrapping add in u32 domain
                let result = a.wrapping_add(b);
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32Sub, Literal::Integer(a), Literal::Integer(b)) => {
                // Wrapping sub in u32 domain
                let result = a.wrapping_sub(b);
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32Mul, Literal::Integer(a), Literal::Integer(b)) => {
                // Wrapping mul in u32 domain
                let result = a.wrapping_mul(b);
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32Div, Literal::Integer(a), Literal::Integer(b)) if b != 0 => {
                // U32 division - standard unsigned division
                let result = a / b;
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32Rem, Literal::Integer(a), Literal::Integer(b)) if b != 0 => {
                // U32 remainder
                let result = a % b;
                Some(Literal::Integer(result))
            }

            // U32 comparisons - proper unsigned comparisons
            (BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a == b))
            }
            (BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a != b))
            }
            (BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a < b))
            }
            (BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a > b))
            }
            (BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a <= b))
            }
            (BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Boolean(a >= b))
            }

            // U32 bitwise operations
            (BinaryOp::U32BitwiseAnd, Literal::Integer(a), Literal::Integer(b)) => {
                let result = a & b;
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32BitwiseOr, Literal::Integer(a), Literal::Integer(b)) => {
                let result = a | b;
                Some(Literal::Integer(result))
            }
            (BinaryOp::U32BitwiseXor, Literal::Integer(a), Literal::Integer(b)) => {
                let result = a ^ b;
                Some(Literal::Integer(result))
            }

            // Boolean operations
            (BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b)) => {
                Some(Literal::Boolean(a && b))
            }
            (BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b)) => {
                Some(Literal::Boolean(a || b))
            }

            _ => None, // Cannot fold or type mismatch
        }
    }

    /// Evaluate a unary operation on a literal value
    pub(crate) fn eval_unary_op(&self, op: UnaryOp, operand: Literal) -> Option<Literal> {
        match (op, operand) {
            (UnaryOp::Not, Literal::Boolean(b)) => Some(Literal::Boolean(!b)),
            (UnaryOp::Neg, Literal::Integer(i)) => {
                // Negation in M31 field
                let m31_value = M31::from(i);
                Some(Literal::Integer((-m31_value).0))
            }
            _ => None,
        }
    }

    /// Check if a value is zero
    pub const fn is_zero(&self, value: &Value) -> bool {
        match value {
            Value::Literal(Literal::Integer(n)) => *n == 0,
            Value::Literal(Literal::Boolean(false)) => true,
            Value::Literal(Literal::Unit) => false,
            _ => false,
        }
    }

    /// Check if a value is one
    pub const fn is_one(&self, value: &Value) -> bool {
        match value {
            Value::Literal(Literal::Integer(n)) => *n == 1,
            Value::Literal(Literal::Boolean(true)) => true,
            _ => false,
        }
    }

    /// Try to convert a literal to a boolean value
    pub const fn as_bool(&self, literal: Literal) -> Option<bool> {
        match literal {
            Literal::Boolean(b) => Some(b),
            Literal::Integer(0) => Some(false),
            Literal::Integer(_) => Some(true),
            Literal::Unit => None,
        }
    }

    /// Get the identity value for a binary operation
    pub const fn identity_value(&self, op: BinaryOp) -> Option<Literal> {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::U32Add | BinaryOp::U32Sub => {
                Some(Literal::Integer(0))
            }
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::U32Mul | BinaryOp::U32Div => {
                Some(Literal::Integer(1))
            }
            BinaryOp::And => Some(Literal::Boolean(true)),
            BinaryOp::Or => Some(Literal::Boolean(false)),
            _ => None,
        }
    }

    /// Get the absorbing value for a binary operation (value that makes result constant)
    pub const fn absorbing_value(&self, op: BinaryOp) -> Option<Literal> {
        match op {
            BinaryOp::Mul | BinaryOp::U32Mul => Some(Literal::Integer(0)),
            BinaryOp::And => Some(Literal::Boolean(false)),
            BinaryOp::Or => Some(Literal::Boolean(true)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use stwo_prover::core::fields::m31::{M31, P};

    proptest! {
        #[test]
        fn test_felt_arithmetic_modular(a in 0..P, b in 0..P) {
            let evaluator = ConstEvaluator::new();

            // Test addition
            let result = evaluator.eval_binary_op(BinaryOp::Add, Literal::Integer(a), Literal::Integer(b));
            let expected = M31::from(a) + M31::from(b);
            assert_eq!(result, Some(Literal::Integer(expected.0)));

            // Test subtraction
            let result = evaluator.eval_binary_op(BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b));
            let expected = M31::from(a) - M31::from(b);
            assert_eq!(result, Some(Literal::Integer(expected.0)));

            // Test multiplication
            let result = evaluator.eval_binary_op(BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b));
            let expected = M31::from(a) * M31::from(b);
            assert_eq!(result, Some(Literal::Integer(expected.0)));
        }

        #[test]
        fn test_felt_division_modular_inverse(a in 1..P, b in 1..P) {
            let evaluator = ConstEvaluator::new();

            // Test division using modular inverse
            let result = evaluator.eval_binary_op(BinaryOp::Div, Literal::Integer(a), Literal::Integer(b));
            let expected = M31::from(a) / M31::from(b);
            assert_eq!(result, Some(Literal::Integer(expected.0)));

            // Verify that (a/b) * b = a in the field
            if let Some(Literal::Integer(quotient)) = result {
                let product = M31::from(quotient) * M31::from(b);
                assert_eq!(product, M31::from(a), "Division should satisfy (a/b)*b = a");
            }
        }

        #[test]
        fn test_u32_arithmetic_wrapping(a in any::<u32>(), b in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Test u32 addition with wrapping
            let result = evaluator.eval_binary_op(BinaryOp::U32Add, Literal::Integer(a), Literal::Integer(b));
            let expected = a.wrapping_add(b);
            assert_eq!(result, Some(Literal::Integer(expected)));

            // Test u32 subtraction with wrapping
            let result = evaluator.eval_binary_op(BinaryOp::U32Sub, Literal::Integer(a), Literal::Integer(b));
            let expected = a.wrapping_sub(b);
            assert_eq!(result, Some(Literal::Integer(expected)));

            // Test u32 multiplication with wrapping
            let result = evaluator.eval_binary_op(BinaryOp::U32Mul, Literal::Integer(a), Literal::Integer(b));
            let expected = a.wrapping_mul(b);
            assert_eq!(result, Some(Literal::Integer(expected)));
        }

        #[test]
        fn test_u32_division(a in any::<u32>(), b in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Test u32 division (b != 0)
            let result = evaluator.eval_binary_op(BinaryOp::U32Div, Literal::Integer(a), Literal::Integer(b));
            if b != 0 {
                let expected = a / b;
                assert_eq!(result, Some(Literal::Integer(expected)));
            } else {
                assert_eq!(result, None);
            }
        }

        #[test]
        fn test_u32_comparisons_unsigned(a in any::<u32>(), b in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Test equality
            let result = evaluator.eval_binary_op(BinaryOp::U32Eq, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a == b)));

            // Test inequality
            let result = evaluator.eval_binary_op(BinaryOp::U32Neq, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a != b)));

            // Test less than
            let result = evaluator.eval_binary_op(BinaryOp::U32Less, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a < b)));

            // Test greater than
            let result = evaluator.eval_binary_op(BinaryOp::U32Greater, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a > b)));

            // Test less than or equal
            let result = evaluator.eval_binary_op(BinaryOp::U32LessEqual, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a <= b)));

            // Test greater than or equal
            let result = evaluator.eval_binary_op(BinaryOp::U32GreaterEqual, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Boolean(a >= b)));
        }

        #[test]
        fn test_division_by_zero(a in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Felt division by zero should return None
            let result = evaluator.eval_binary_op(BinaryOp::Div, Literal::Integer(a), Literal::Integer(0));
            assert_eq!(result, None);

            // U32 division by zero should return None
            let result = evaluator.eval_binary_op(BinaryOp::U32Div, Literal::Integer(a), Literal::Integer(0));
            assert_eq!(result, None);
        }

        #[test]
        fn test_boolean_operations(a in any::<bool>(), b in any::<bool>()) {
            let evaluator = ConstEvaluator::new();

            // Test AND
            let result = evaluator.eval_binary_op(BinaryOp::And, Literal::Boolean(a), Literal::Boolean(b));
            assert_eq!(result, Some(Literal::Boolean(a && b)));

            // Test OR
            let result = evaluator.eval_binary_op(BinaryOp::Or, Literal::Boolean(a), Literal::Boolean(b));
            assert_eq!(result, Some(Literal::Boolean(a || b)));
        }

        #[test]
        fn test_boolean_not(a in any::<bool>()) {
            let evaluator = ConstEvaluator::new();

            // Test NOT
            let result = evaluator.eval_unary_op(UnaryOp::Not, Literal::Boolean(a));
            assert_eq!(result, Some(Literal::Boolean(!a)));
        }

        #[test]
        fn test_unary_negation(a in 0..P) {
            let evaluator = ConstEvaluator::new();

            // Test negation in M31 field
            let result = evaluator.eval_unary_op(UnaryOp::Neg, Literal::Integer(a));
            let expected = (-M31::from(a)).0;
            assert_eq!(result, Some(Literal::Integer(expected)));

            // Verify that a + (-a) = 0 in the field
            if let Some(Literal::Integer(neg_a)) = result {
                let sum = M31::from(a) + M31::from(neg_a);
                assert_eq!(sum.0, 0, "a + (-a) should equal 0 in M31");
            }
        }

        #[test]
        fn test_u32_bitwise_operations(a in any::<u32>(), b in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Test AND
            let result = evaluator.eval_binary_op(BinaryOp::U32BitwiseAnd, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Integer(a & b)));

            // Test OR
            let result = evaluator.eval_binary_op(BinaryOp::U32BitwiseOr, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Integer(a | b)));

            // Test XOR
            let result = evaluator.eval_binary_op(BinaryOp::U32BitwiseXor, Literal::Integer(a), Literal::Integer(b));
            assert_eq!(result, Some(Literal::Integer(a ^ b)));
        }

        #[test]
        fn test_u32_bitwise_associativity(a in any::<u32>(), b in any::<u32>(), c in any::<u32>()) {
            let evaluator = ConstEvaluator::new();

            // Test AND associativity: (a & b) & c = a & (b & c)
            let left_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseAnd, Literal::Integer(a), Literal::Integer(b))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseAnd, Literal::Integer(val), Literal::Integer(c)),
                    _ => None,
                });
            let right_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseAnd, Literal::Integer(b), Literal::Integer(c))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseAnd, Literal::Integer(a), Literal::Integer(val)),
                    _ => None,
                });
            assert_eq!(left_first, right_first, "AND should be associative");

            // Test OR associativity: (a | b) | c = a | (b | c)
            let left_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseOr, Literal::Integer(a), Literal::Integer(b))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseOr, Literal::Integer(val), Literal::Integer(c)),
                    _ => None,
                });
            let right_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseOr, Literal::Integer(b), Literal::Integer(c))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseOr, Literal::Integer(a), Literal::Integer(val)),
                    _ => None,
                });
            assert_eq!(left_first, right_first, "OR should be associative");

            // Test XOR associativity: (a ^ b) ^ c = a ^ (b ^ c)
            let left_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseXor, Literal::Integer(a), Literal::Integer(b))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseXor, Literal::Integer(val), Literal::Integer(c)),
                    _ => None,
                });
            let right_first = evaluator.eval_binary_op(BinaryOp::U32BitwiseXor, Literal::Integer(b), Literal::Integer(c))
                .and_then(|res| match res {
                    Literal::Integer(val) => evaluator.eval_binary_op(BinaryOp::U32BitwiseXor, Literal::Integer(a), Literal::Integer(val)),
                    _ => None,
                });
            assert_eq!(left_first, right_first, "XOR should be associative");
        }
    }

    // Keep some simple unit tests for identity/absorbing values since they're testing const functions
    #[test]
    fn test_identity_and_absorbing_values() {
        let evaluator = ConstEvaluator::new();

        // Test identity values
        assert_eq!(
            evaluator.identity_value(BinaryOp::Add),
            Some(Literal::Integer(0))
        );
        assert_eq!(
            evaluator.identity_value(BinaryOp::Mul),
            Some(Literal::Integer(1))
        );
        assert_eq!(
            evaluator.identity_value(BinaryOp::And),
            Some(Literal::Boolean(true))
        );
        assert_eq!(
            evaluator.identity_value(BinaryOp::Or),
            Some(Literal::Boolean(false))
        );

        // Test absorbing values
        assert_eq!(
            evaluator.absorbing_value(BinaryOp::Mul),
            Some(Literal::Integer(0))
        );
        assert_eq!(
            evaluator.absorbing_value(BinaryOp::And),
            Some(Literal::Boolean(false))
        );
        assert_eq!(
            evaluator.absorbing_value(BinaryOp::Or),
            Some(Literal::Boolean(true))
        );
    }

    #[test]
    fn test_is_zero_and_is_one() {
        let evaluator = ConstEvaluator::new();

        assert!(evaluator.is_zero(&Value::Literal(Literal::Integer(0))));
        assert!(!evaluator.is_zero(&Value::Literal(Literal::Integer(1))));
        assert!(evaluator.is_zero(&Value::Literal(Literal::Boolean(false))));

        assert!(evaluator.is_one(&Value::Literal(Literal::Integer(1))));
        assert!(!evaluator.is_one(&Value::Literal(Literal::Integer(0))));
        assert!(evaluator.is_one(&Value::Literal(Literal::Boolean(true))));
    }
}
