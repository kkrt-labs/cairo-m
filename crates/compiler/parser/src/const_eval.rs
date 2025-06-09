//! # Compile-time Expression Evaluation
//!
//! This module provides functionality for evaluating constant expressions at compile-time.
//! It supports arithmetic operations on integer literals while ensuring that results
//! do not overflow the M31 maximum value (2^31 - 1).

use crate::parser::{Expression, BinaryOp};

/// Maximum value for M31 field (2^31 - 1)
const M31_MAX: u32 = 2147483647;

/// Errors that can occur during compile-time evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// Arithmetic overflow beyond M31 maximum value
    Overflow,
    /// Division by zero
    DivisionByZero,
    /// Cannot evaluate non-constant expression at compile time
    NonConstant,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => write!(f, "Arithmetic overflow: result exceeds M31 maximum value (2^31 - 1)"),
            Self::DivisionByZero => write!(f, "Division by zero in compile-time evaluation"),
            Self::NonConstant => write!(f, "Cannot evaluate non-constant expression at compile time"),
        }
    }
}

/// Attempts to evaluate an expression at compile-time if it consists only of literals and operators.
/// Returns the evaluated value if successful, or an error if evaluation fails.
pub fn try_evaluate_const_expr(expr: &Expression) -> Result<u32, EvalError> {
    match expr {
        Expression::Literal(n) => Ok(*n),
        Expression::BinaryOp { op, left, right } => {
            let left_val = try_evaluate_const_expr(left)?;
            let right_val = try_evaluate_const_expr(right)?;
            
            match op {
                BinaryOp::Add => {
                    let result = left_val as u64 + right_val as u64;
                    if result > M31_MAX as u64 {
                        Err(EvalError::Overflow)
                    } else {
                        Ok(result as u32)
                    }
                }
                BinaryOp::Sub => {
                    if left_val < right_val {
                        // Underflow would result in negative number, which is not valid for u32
                        Err(EvalError::Overflow)
                    } else {
                        Ok(left_val - right_val)
                    }
                }
                BinaryOp::Mul => {
                    let result = left_val as u64 * right_val as u64;
                    if result > M31_MAX as u64 {
                        Err(EvalError::Overflow)
                    } else {
                        Ok(result as u32)
                    }
                }
                BinaryOp::Div => {
                    if right_val == 0 {
                        Err(EvalError::DivisionByZero)
                    } else if left_val % right_val != 0 {
                        // Only allow division that results in exact integer division
                        Err(EvalError::NonConstant)
                    } else {
                        Ok(left_val / right_val)
                    }
                }
                // Comparison and logical operators don't produce numeric results
                // that can be used in arithmetic contexts, so we don't evaluate them
                _ => Err(EvalError::NonConstant),
            }
        }
        // Any other expression type cannot be evaluated at compile time
        _ => Err(EvalError::NonConstant),
    }
}

/// Attempts to perform compile-time evaluation on an expression.
/// If the expression can be evaluated to a constant, returns the simplified literal.
/// Otherwise, returns the original expression unchanged.
pub fn maybe_evaluate_const_expr(expr: Expression) -> Expression {
    match try_evaluate_const_expr(&expr) {
        Ok(value) => Expression::Literal(value),
        Err(_) => expr, // Keep original expression if evaluation fails
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_addition() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(4)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Ok(7));
        assert_eq!(maybe_evaluate_const_expr(expr), Expression::Literal(7));
    }

    #[test]
    fn test_complex_expression() {
        // 3 + 4 * 2 = 3 + 8 = 11
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::BinaryOp {
                op: BinaryOp::Mul,
                left: Box::new(Expression::Literal(4)),
                right: Box::new(Expression::Literal(2)),
            }),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Ok(11));
        assert_eq!(maybe_evaluate_const_expr(expr), Expression::Literal(11));
    }

    #[test]
    fn test_overflow() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Add,
            left: Box::new(Expression::Literal(M31_MAX)),
            right: Box::new(Expression::Literal(1)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::Overflow));
        
        // Should return original expression when evaluation fails
        let original = expr.clone();
        assert_eq!(maybe_evaluate_const_expr(expr), original);
    }

    #[test]
    fn test_division_by_zero() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(0)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn test_exact_division() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(2)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Ok(5));
        assert_eq!(maybe_evaluate_const_expr(expr), Expression::Literal(5));
    }

    #[test]
    fn test_inexact_division() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Div,
            left: Box::new(Expression::Literal(10)),
            right: Box::new(Expression::Literal(3)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::NonConstant));
    }

    #[test]
    fn test_subtraction_underflow() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Sub,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(5)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::Overflow));
    }

    #[test]
    fn test_non_constant_expression() {
        let expr = Expression::Identifier("x".to_string());
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::NonConstant));
        assert_eq!(maybe_evaluate_const_expr(expr.clone()), expr);
    }

    #[test]
    fn test_comparison_operators() {
        let expr = Expression::BinaryOp {
            op: BinaryOp::Eq,
            left: Box::new(Expression::Literal(3)),
            right: Box::new(Expression::Literal(3)),
        };
        
        assert_eq!(try_evaluate_const_expr(&expr), Err(EvalError::NonConstant));
    }
}

