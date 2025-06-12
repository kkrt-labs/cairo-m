#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

//! # Cairo-M Semantic Analysis
//!
//! This module implements semantic analysis for the Cairo-M language using Salsa for
//! incremental compilation. It builds upon the parser AST to create a semantic model
//! that understands scopes, symbols, definitions, and use-def relationships.
//!
//! ## Architecture
//!
//! The semantic analysis follows a layered approach inspired by Ruff:
//! 1. **Places & Scopes**: Track all named entities and their containing scopes
//! 2. **Definitions**: Link AST nodes to semantic entities
//! 3. **Use-Def Analysis**: Resolve identifier uses to their definitions
//! 4. **Control Flow** (Advanced): Handle conditional visibility and reachability
//!
//! ## Main Query
//!
//! The primary entry point is `semantic_index(db, file)` which produces a complete
//! semantic model for a source file, cached by Salsa for incremental compilation.

use cairo_m_compiler_parser as parser;

// Import file types from parser
pub use parser::{parse_program, ParsedModule, SourceProgram};

pub mod definition;
pub mod place;
pub mod semantic_index;
pub mod type_resolution;
pub mod types;

pub mod db;
pub use db::{SemanticDatabaseImpl, SemanticDb};
pub mod validation;

// Re-export main types and functions
pub use definition::{Definition, DefinitionKind, Definitions};
pub use place::{FileScopeId, PlaceFlags, PlaceTable, Scope, ScopeKind, ScopedPlaceId};
pub use semantic_index::{
    semantic_index as analyze_semantics, validate_semantics, DefinitionId, ExpressionId,
    SemanticIndex,
};
pub use types::{FunctionSignatureId, StructTypeId, TypeData, TypeId};

/// A file in the semantic analysis system
/// For now, we reuse the parser's file concept
pub type File = SourceProgram;
