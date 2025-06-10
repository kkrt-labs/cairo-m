#[salsa::db]
#[derive(Clone, Default)]
pub struct ParserDatabaseImpl {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for ParserDatabaseImpl {}

// Most basic database that gives access to the parsed AST.
#[salsa::db]
pub trait Db: salsa::Database {}

/// Trait for upcasting a reference to a base trait object.
pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
    fn upcast_mut(&mut self) -> &mut T;
}

// Implement the trait for our concrete database
#[salsa::db]
impl Db for ParserDatabaseImpl {}
