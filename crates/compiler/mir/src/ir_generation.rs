//! # Semantic AST to MIR Lowering
//!
//! This module serves as a compatibility layer that delegates to the new
//! modular lowering implementation in the `lowering` submodule.
//!
//! ## Migration Note
//!
//! This module is being refactored. The actual implementation has been moved to
//! the `lowering` submodule for better organization and maintainability.
//! This file now only re-exports the main entry point for backward compatibility.

#[cfg(test)]
mod tests {
    mod mir_generation_tests;
    mod test_harness;
}

// Re-export the main entry point from the new lowering module
pub use crate::lowering::generate_mir;
