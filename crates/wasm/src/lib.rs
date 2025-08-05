//! Cairo-M WASM Frontend
//!
//! This crate provides functionality for loading and analyzing WASM modules
//! as part of the Cairo-M compiler toolchain.

pub mod loader;

pub use loader::{format_wasm_module, load_module, print_wasm_module};
