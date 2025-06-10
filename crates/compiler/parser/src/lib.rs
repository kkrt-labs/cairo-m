mod db;
pub mod lexer;
pub mod parser;

pub use db::ParserDatabaseImpl;

// Define the database trait for the parser
#[salsa::db]
pub trait Db: salsa::Database {}

// Implement the trait for our concrete database
#[salsa::db]
impl Db for ParserDatabaseImpl {}

// Re-export important types from parser module
pub use parser::{parse_program, ParsedModule, SourceProgram};
