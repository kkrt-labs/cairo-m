//! # MIR Lowering Module
//!
//! This module contains the infrastructure for lowering the semantic AST to MIR.
//! It's organized into focused submodules that each handle a specific aspect of
//! the lowering process.

pub mod builder;
pub mod expr;
pub mod function;
pub mod stmt;
pub mod utils;

#[cfg(test)]
mod value_based_tests;

#[cfg(test)]
mod array_memory_tests;

#[cfg(test)]
mod member_access_array_index_tests;

// Re-export the main entry points
pub use function::{generate_mir, generate_mir_with_config};

// Re-export commonly used items
pub use builder::MirBuilder;
