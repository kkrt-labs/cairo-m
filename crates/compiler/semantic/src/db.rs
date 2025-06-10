use cairo_m_compiler_parser as parser;
use parser::Db as ParserDb;

// We may need the parser implementation later for cross-database operations
#[allow(unused_imports)]
use parser::ParserDatabaseImpl;

/// Database trait for semantic analysis, extending the parser database
#[salsa::db]
pub trait SemanticDb: ParserDb {
    // Future: Add semantic-specific database methods here
    // fn semantic_settings(&self) -> &SemanticSettings;
}

/// Concrete database implementation for semantic analysis
#[salsa::db]
#[derive(Clone, Default)]
pub struct SemanticDatabaseImpl {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for SemanticDatabaseImpl {}

#[salsa::db]
impl ParserDb for SemanticDatabaseImpl {}

#[salsa::db]
impl SemanticDb for SemanticDatabaseImpl {}
