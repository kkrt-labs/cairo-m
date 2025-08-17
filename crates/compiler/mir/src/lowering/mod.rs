//! # MIR Lowering Module
//!
//! This module contains the infrastructure for lowering the semantic AST to MIR.
//! It's organized into focused submodules that each handle a specific aspect of
//! the lowering process.

pub mod array_guards;
pub mod builder;
pub mod control_flow;
pub mod expr;
pub mod function;
pub mod stmt;
pub mod utils;

#[cfg(test)]
mod value_based_tests;

#[cfg(test)]
mod array_memory_tests;

// Re-export the main entry point
pub use function::generate_mir;

// Re-export commonly used items
pub use builder::MirBuilder;
