#![allow(clippy::option_if_let_else)]
pub mod db;
pub mod lexer;
pub mod parser;

pub use db::{
    Db, DiscoveredCrate, ParsedCrate, ParserDatabaseImpl, SourceFile, Upcast, parse_crate,
    project_validate_parser,
};
// Re-export important types from parser module
pub use parser::{ParseOutput, ParsedModule, parse_file};
