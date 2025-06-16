//! # Control Flow Validation Tests
//!
//! Tests for control flow analysis including:
//! - Unreachable code detection
//! - Missing return statement detection
//! - Control flow path analysis
//! - Dead code elimination validation

pub mod control_flow_paths;
pub mod missing_returns;
pub mod unreachable_code;
