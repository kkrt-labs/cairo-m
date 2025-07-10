mod db;
pub mod lexer;
pub mod parser;

pub use db::{parse_project, Db, ParsedProject, ParserDatabaseImpl, Project, SourceFile, Upcast};
// Re-export important types from parser module
pub use parser::{parse_file, ParseOutput, ParsedModule};
