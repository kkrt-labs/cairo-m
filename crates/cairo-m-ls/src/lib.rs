#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

//! Language server library public exports.

// Re-export modules needed for testing
pub mod db;
pub mod diagnostics;
pub mod lsp_ext;
pub mod lsp_tracing;
pub mod project;
pub mod utils;

// Re-export the Backend struct for testing
mod backend;
pub use backend::Backend;
