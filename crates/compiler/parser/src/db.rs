use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCollection};

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

/// Represents raw discovered files from the filesystem.
/// This is a parser-level input representing what files were found,
/// not yet organized into a semantic module structure.
#[salsa::input(debug)]
pub struct DiscoveredCrate {
    #[returns(ref)]
    pub root_dir: String,
    #[returns(ref)]
    pub entry_file: String,
    pub files: Vec<SourceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCrate {
    pub modules: std::collections::HashMap<String, ParsedModule>,
    pub diagnostics: Vec<Diagnostic>,
}

#[salsa::tracked]
pub fn parse_crate(db: &dyn Db, cairo_m_crate: DiscoveredCrate) -> ParsedCrate {
    let mut modules = std::collections::HashMap::new();
    let mut diagnostics = Vec::new();

    for file in cairo_m_crate.files(db) {
        tracing::debug!("Parsing file with content: {}", file.text(db));
        let parsed = super::parser::parse_file(db, file);
        diagnostics.extend(parsed.diagnostics);
        modules.insert(file.file_path(db).to_string(), parsed.module);
    }

    ParsedCrate {
        modules,
        diagnostics,
    }
}

/// Project-level parser validation that returns diagnostics in the same format as semantic validation
#[salsa::tracked]
pub fn project_validate_parser(
    db: &dyn Db,
    cairo_m_crate: DiscoveredCrate,
) -> DiagnosticCollection {
    let parsed_crate = parse_crate(db, cairo_m_crate);
    DiagnosticCollection::new(parsed_crate.diagnostics)
}
