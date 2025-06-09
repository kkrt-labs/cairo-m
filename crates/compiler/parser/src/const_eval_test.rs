#[cfg(test)]
mod tests {
    use crate::const_eval::{maybe_evaluate_const_expr, try_evaluate_const_expr, EvalError};
    use crate::parser::{BinaryOp, Expression};

    #[test]
    fn test_simple_addition() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(4)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(7));
    }

    #[test]
    fn test_simple_subtraction() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Sub,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(3)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(7));
    }

    #[test]
    fn test_simple_multiplication() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(4)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(12));
    }

    #[test]
    fn test_simple_division() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(12)),
            right: Box::new(Expression::Literal(3)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(4));
    }

    #[test]
    fn test_complex_expression() {
        // Test: 3 + 4 * 2 = 11
        let mul_expr = Expression::BinaryOp {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Literal(4)),
            right: Box::new(Expression::Literal(2)),
        };

        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(mul_expr),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(11));
    }

    #[test]
    fn test_overflow_protection() {
        // Test M31 overflow: 2^31 - 1 + 1 should not evaluate
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(2147483647)), // M31_MAX
            right: Box::new(Expression::Literal(1)),
        };

        let result = try_evaluate_const_expr(&expr);
        assert!(matches!(result, Err(EvalError::Overflow)));
    }

    #[test]
    fn test_multiplication_overflow() {
        // Test multiplication overflow
        let expr = Expression::BinaryOp {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Literal(100000)),
            right: Box::new(Expression::Literal(100000)),
        };

        let result = try_evaluate_const_expr(&expr);
        assert!(matches!(result, Err(EvalError::Overflow)));
    }

    #[test]
    fn test_division_by_zero() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(0)),
        };

        let result = try_evaluate_const_expr(&expr);
        assert!(matches!(result, Err(EvalError::DivisionByZero)));
    }

    #[test]
    fn test_inexact_division() {
        // Test division that doesn't result in a whole number
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(3)),
        };

        let result = try_evaluate_const_expr(&expr);
        assert!(matches!(result, Err(EvalError::NonConstant)));
    }

    #[test]
    fn test_subtraction_underflow() {
        // Test subtraction that would result in negative number
        let expr = Expression::BinaryOp {
            op: BinaryOp::Sub,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(5)),
        };

        let result = try_evaluate_const_expr(&expr);
        assert!(matches!(result, Err(EvalError::Overflow)));
    }

    #[test]
    fn test_non_constant_expression() {
        // Test with a variable (non-constant)
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Identifier("x".to_string())),
            right: Box::new(Expression::Literal(5)),
        };

        let result = maybe_evaluate_const_expr(expr.clone());
        assert_eq!(result, expr); // Should return original expression unchanged
    }

    #[test]
    fn test_comparison_operators_not_evaluated() {
        // Comparison operators should not be evaluated at compile time
        let expr = Expression::BinaryOp {
            op: BinaryOp::Eq,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(3)),
        };

        let result = maybe_evaluate_const_expr(expr.clone());
        assert_eq!(result, expr); // Should return original expression unchanged
    }

    #[test]
    fn test_logical_operators_not_evaluated() {
        // Logical operators should not be evaluated at compile time
        let expr = Expression::BinaryOp {
            op: BinaryOp::And,
            left: Box::new(Expression::Literal(1)),
            right: Box::new(Expression::Literal(1)),
        };

        let result = maybe_evaluate_const_expr(expr.clone());
        assert_eq!(result, expr); // Should return original expression unchanged
    }

    #[test]
    fn test_max_value_operations() {
        // Test operations at the boundary of M31_MAX
        let expr = Expression::BinaryOp {
            op: BinaryOp::Sub,
            left: Box::new(Expression::Literal(2147483647)), // M31_MAX
            right: Box::new(Expression::Literal(1)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(2147483646));
    }

    #[test]
    fn test_zero_operations() {
        // Test operations with zero
        let expr = Expression::BinaryOp {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Literal(0)),
            right: Box::new(Expression::Literal(12345)),
        };

        let result = maybe_evaluate_const_expr(expr);
        assert_eq!(result, Expression::Literal(0));
    }
}
