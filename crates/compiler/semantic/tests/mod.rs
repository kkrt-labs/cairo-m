//! # Semantic Validation Tests
//!
//! This module contains comprehensive tests for semantic validation organized by concern.
//! Tests are structured to provide clear visibility into what semantic features are
//! implemented and validated.
//!
//! ## Test Organization
//!
//! - `scoping/` - Variable scoping, visibility, and declaration tests
//! - `types/` - Type checking, type resolution, and type validation tests
//! - `control_flow/` - Control flow analysis, unreachable code, missing returns
//! - `functions/` - Function calls, parameter validation, return type checking
//! - `structures/` - Struct definition, field access, and struct validation
//! - `expressions/` - Expression validation, operator usage, literal validation
//! - `statements/` - Statement validation (let, assignment, etc.)
//! - `integration/` - End-to-end integration tests with complex scenarios
//!
//! ## Test Utilities
//!
//! This module re-exports the core testing utilities:
//! - `assert_semantic_ok!(code)` - Assert code validates without errors
//! - `assert_semantic_err!(code)` - Assert code produces validation errors
//! - `assert_diagnostics_snapshot!(file, name)` - Snapshot test for .cm files

// Import all common test utilities
pub mod common;
use cairo_m_compiler_parser::parser::{NamedType, Spanned, TypeExpr};
use chumsky::span::SimpleSpan;
pub use common::*;

/// Macro to assert that inline code validates successfully
#[macro_export]
macro_rules! assert_semantic_ok {
    ($code:expr) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_ok_impl($code, function_name)
    }};
    ($code:expr, show_unused) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_ok_impl_with_options($code, function_name, true)
    }};
}

/// Macro to assert that inline code fails semantic validation
#[macro_export]
macro_rules! assert_semantic_err {
    ($code:expr) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_err_impl($code, function_name)
    }};
    ($code:expr, show_unused) => {{
        let function_name = stdext::function_name!();
        $crate::assert_semantic_err_impl_with_options($code, function_name, true)
    }};
}

/// Helper macro for snapshot testing
#[macro_export]
macro_rules! assert_diagnostics_snapshot {
    ($fixture:expr, $snapshot_name:expr) => {
        $crate::assert_diagnostics_snapshot($fixture, $snapshot_name);
    };
    ($fixture:expr, $snapshot_name:expr, $description:expr) => {
        $crate::validation::tests::assert_diagnostics_snapshot_with_context(
            $fixture,
            $snapshot_name,
            Some($description),
            None::<&dyn std::fmt::Debug>,
        );
    };
    ($fixture:expr, $snapshot_name:expr, $description:expr, $context:expr) => {
        $crate::validation::tests::assert_diagnostics_snapshot_with_context(
            $fixture,
            $snapshot_name,
            Some($description),
            Some($context),
        );
    };
}

/// Helper macro for clean fixture testing
#[macro_export]
macro_rules! test_fixture_clean {
    ($fixture:expr) => {
        $crate::test_fixture_clean($fixture);
    };
}

/// Macro for parameterized semantic tests
/// Usage: assert_semantic_parameterized! {
///     ok: ["valid1", "valid2"],
///     err: ["invalid1", "invalid2"]
/// }
#[macro_export]
macro_rules! assert_semantic_parameterized {
    (ok: [$($ok:expr),* $(,)?], err: [$($err:expr),* $(,)?]) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($ok.to_string(), true),)*
            $(($err.to_string(), false),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        $crate::assert_semantic_parameterized_impl(&inputs_ref, function_name, false)
    }};
    (ok: [$($ok:expr),* $(,)?], err: [$($err:expr),* $(,)?], show_unused) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($ok.to_string(), true),)*
            $(($err.to_string(), false),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        $crate::assert_semantic_parameterized_impl(&inputs_ref, function_name, true)
    }};
    (ok: [$($ok:expr),* $(,)?]) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($ok.to_string(), true),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        $crate::assert_semantic_parameterized_impl(&inputs_ref, function_name, false)
    }};
    (err: [$($err:expr),* $(,)?]) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($err.to_string(), false),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        $crate::assert_semantic_parameterized_impl(&inputs_ref, function_name, false)
    }};
}

// Helper functions to create test AST nodes with dummy spans
pub(crate) fn spanned<T>(value: T) -> Spanned<T> {
    Spanned::new(value, SimpleSpan::from(0..0))
}

pub(crate) fn named_type(name: NamedType) -> Spanned<TypeExpr> {
    spanned(TypeExpr::Named(spanned(name)))
}

pub(crate) fn pointer_type(inner: Spanned<TypeExpr>) -> Spanned<TypeExpr> {
    spanned(TypeExpr::Pointer(Box::new(inner)))
}
// Test modules organized by concern
pub mod control_flow;
pub mod expressions;
pub mod functions;
pub mod integration;
pub mod scoping;
pub mod semantic_model;
pub mod statements;
pub mod structures;
pub mod types;
