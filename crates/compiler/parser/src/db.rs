use cairo_m_compiler_diagnostics::Diagnostic;

use crate::ParsedModule;

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

#[salsa::input(debug)]
pub struct SourceFile {
    #[returns(ref)]
    pub text: String,
    #[returns(ref)]
    pub file_path: String,
}

#[salsa::input(debug)]
pub struct Project {
    #[returns(ref)]
    pub root_dir: String,
    #[returns(ref)]
    pub entry_file: String,
    pub files: Vec<SourceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProject {
    pub modules: std::collections::HashMap<String, ParsedModule>,
    pub diagnostics: Vec<Diagnostic>,
}

#[salsa::tracked]
pub fn parse_project(db: &dyn Db, project: Project) -> ParsedProject {
    let mut modules = std::collections::HashMap::new();
    let mut diagnostics = Vec::new();

    for file in project.files(db) {
        let parsed = super::parser::parse_file(db, file);
        diagnostics.extend(parsed.diagnostics);
        modules.insert(file.file_path(db).to_string(), parsed.module);
    }

    ParsedProject {
        modules,
        diagnostics,
    }
}
