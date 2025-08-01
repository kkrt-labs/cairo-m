#![allow(clippy::option_if_let_else)]
pub mod db;
pub mod lexer;
pub mod parser;

pub use db::{
    parse_crate, project_validate_parser, Db, DiscoveredCrate, ParsedCrate, ParserDatabaseImpl,
    SourceFile, Upcast,
};
// Re-export important types from parser module
pub use parser::{parse_file, ParseOutput, ParsedModule};
