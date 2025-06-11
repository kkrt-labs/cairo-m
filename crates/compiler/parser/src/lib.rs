mod db;
pub mod error;
pub mod lexer;
pub mod parser;

pub use db::{Db, ParserDatabaseImpl, Upcast};

// Re-export important types from parser module
pub use parser::{parse_program, ParseDiagnostic, ParseOutput, ParsedModule, SourceProgram};

// Re-export error utilities
pub use error::*;
