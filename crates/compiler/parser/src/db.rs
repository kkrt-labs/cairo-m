#[salsa::db]
#[derive(Clone, Default)]
pub struct ParserDatabaseImpl {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for ParserDatabaseImpl {}
