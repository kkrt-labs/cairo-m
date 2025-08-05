//! Cairo-M WASM Frontend
//!
//! This crate provides functionality for loading and analyzing WASM modules
//! as part of the Cairo-M compiler toolchain.

pub mod loader;

// Re-export key types and functions for convenience
pub use loader::{format_womir_program, load_module, print_womir_program};
