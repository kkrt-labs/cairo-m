//! # Variable Scoping Tests
//!
//! Tests for variable scoping, visibility, declarations, and scope-related errors.
//! This module validates that the semantic analyzer correctly handles:
//!
//! - Variable declarations and visibility
//! - Scope boundaries and nested scopes
//! - Undeclared variable detection
//! - Duplicate definition detection
//! - Unused variable warnings
//! - Parameter vs local variable scoping

pub mod duplicate_definitions;
pub mod nested_scopes;
pub mod undeclared_variables;
pub mod unused_variables;
