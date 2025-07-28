pub mod api;
pub mod comment_attachment;
pub mod comment_preserver;
pub mod config;
pub mod context;
pub mod doc;
pub mod rules;
pub mod simple_comment_preserver;
pub mod trivia;
pub mod utils;

use doc::Doc;

/// Trait for formatting AST nodes into Doc IR
pub trait Format {
    fn format(&self, ctx: &mut context::FormatterCtx) -> Doc;
}

pub use api::{format_parsed_module, format_source_file};
pub use config::FormatterConfig;
