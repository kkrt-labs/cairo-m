mod db;
pub mod lexer;
pub mod parser;

#[allow(unused_imports)]
use db::ParserDatabaseImpl;
use salsa::Database as Db;
