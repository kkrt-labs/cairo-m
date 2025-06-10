pub mod ast;
pub mod db;
pub mod lexer;
pub mod parser;

#[allow(unused_imports)]
pub use db::ParserDatabaseImpl;
use salsa::Database as Db;
