//! # Control Flow Validation Tests
//!
//! Tests for control flow analysis including:
//! - Unreachable code detection
//! - Missing return statement detection
//! - Control flow path analysis
//! - Dead code elimination validation
//! - Loop validation (break/continue)
//! - Loop scoping
//! - Loop type checking

pub mod for_loops;
pub mod loop_scoping;
pub mod loop_type_checking;
pub mod loop_validation;
pub mod missing_returns;
pub mod unreachable_code;
