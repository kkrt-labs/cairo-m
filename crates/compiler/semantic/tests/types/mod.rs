//! # Type System Tests
//!
//! This module contains comprehensive white-box tests for the type system.
//! These tests directly verify the correctness of type resolution queries
//! and the semantic model's type information.
//!
//! ## Test Categories
//!
//! - **Type Resolution**: Tests for `resolve_ast_type` and basic type lookup
//! - **Definition Types**: Tests for `definition_semantic_type` with various definition kinds
//! - **Expression Types**: Tests for `expression_semantic_type` and type inference
//! - **Function Signatures**: Tests for `function_semantic_signature` resolution
//! - **Struct Types**: Tests for `struct_semantic_data` and field resolution
//! - **Type Compatibility**: Tests for type compatibility and conversion rules

mod definition_type_tests;
mod expression_type_tests;
mod function_signature_tests;
mod literal_range_validation_tests;
mod literal_type_inference_tests;
mod query_integration_tests;
mod recursive_and_error_types_tests;
mod return_type_inference;
mod struct_type_tests;
mod type_compatibility_tests;
mod type_resolution_tests;
mod u32_type_tests;

// Re-export test utilities for use in submodules
pub use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type, function_semantic_signature,
    resolve_ast_type, struct_semantic_data,
};
pub use cairo_m_compiler_semantic::types::{TypeData, TypeId};
pub use cairo_m_compiler_semantic::{File, SemanticDb};

pub use crate::common::*;
