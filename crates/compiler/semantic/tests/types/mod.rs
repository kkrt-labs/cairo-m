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
mod recursive_and_error_types_tests;
mod struct_type_tests;
mod type_compatibility_tests;
mod type_resolution_tests;
mod u32_type_tests;

// Re-export test utilities for use in submodules
use cairo_m_compiler_parser::{Db as ParserDb, Upcast};
use cairo_m_compiler_semantic::SemanticDb;

pub use crate::{assert_semantic_err, assert_semantic_ok};

#[salsa::db]
#[derive(Clone, Default)]
pub struct TestDb {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TestDb {}
#[salsa::db]
impl ParserDb for TestDb {}
#[salsa::db]
impl SemanticDb for TestDb {}

impl Upcast<dyn ParserDb> for TestDb {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

pub fn test_db() -> TestDb {
    TestDb::default()
}

pub use cairo_m_compiler_semantic::File;
pub use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type, function_semantic_signature,
    resolve_ast_type, struct_semantic_data,
};
pub use cairo_m_compiler_semantic::types::{TypeData, TypeId};
