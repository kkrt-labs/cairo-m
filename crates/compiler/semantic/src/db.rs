//! # Semantic Analysis Database
//!
//! This module defines the database traits and implementations for semantic analysis
//! using the Salsa incremental computation framework. It extends the parser database
//! to provide semantic-specific functionality.
//!
//! The database system enables incremental recompilation by caching query results
//! and invalidating them only when their dependencies change.
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCollection};
use cairo_m_compiler_parser::parser::TopLevelItem;
use cairo_m_compiler_parser::{self as parser, parse_file};
#[allow(unused_imports)]
use parser::ParserDatabaseImpl;
use parser::{Db as ParserDb, Upcast};

use crate::semantic_index::{ProjectSemanticIndex, SemanticIndex, semantic_index_from_module};
use crate::validation::validator::create_default_registry;
use crate::{File, ParsedModule};

/// Database trait for semantic analysis, extending the parser database
///
/// This trait defines the interface for semantic-specific database operations.
/// Type resolution and inference queries are defined as standalone tracked functions.
#[salsa::db]
pub trait SemanticDb: ParserDb + Upcast<dyn ParserDb> {
    // Type queries are defined as standalone tracked functions in type_resolution.rs
    // This trait includes any database-specific configuration or settings
}

/// Concrete database implementation for semantic analysis
///
/// This provides the actual storage and implementation for all database queries.
/// It combines both parser and semantic analysis capabilities in a single database.
///
/// # Thread Safety
///
/// This implementation is `Clone` and can be safely shared between threads.
/// Salsa handles the synchronization internally.
#[salsa::db]
#[derive(Clone, Default)]
pub struct SemanticDatabaseImpl {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for SemanticDatabaseImpl {}
#[salsa::db]
impl ParserDb for SemanticDatabaseImpl {}
#[salsa::db]
impl SemanticDb for SemanticDatabaseImpl {}

impl Upcast<dyn ParserDb> for SemanticDatabaseImpl {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

#[salsa::tracked]
pub fn project_validate_semantics(db: &dyn SemanticDb, project: Project) -> DiagnosticCollection {
    let parse_diag = project_parse_diagnostics(db, project);
    if !parse_diag.is_empty() {
        return parse_diag;
    }

    let sem_result = project_semantic_index(db, project);
    match sem_result {
        Ok(sem) => {
            let mut coll = DiagnosticCollection::default();
            let registry = create_default_registry();
            for (module_name, index) in sem.modules().iter() {
                let module_file = *project
                    .modules(db)
                    .get(module_name)
                    .unwrap_or_else(|| panic!("Module file should exist: {}", module_name));
                coll.extend(registry.validate_all(db, project, module_file, index));
            }
            coll
        }
        Err(err_diag) => err_diag,
    }
}

#[salsa::input(debug)]
pub struct Project {
    #[return_ref]
    pub modules: HashMap<String, File>,
    pub entry_point: String,
}

/// Find the module name for a given file in the project
pub fn module_name_for_file(db: &dyn SemanticDb, project: Project, file: File) -> Option<String> {
    for (module_name, module_file) in project.modules(db).iter() {
        if *module_file == file {
            return Some(module_name.clone());
        }
    }
    None
}

#[salsa::tracked]
pub fn project_parse_diagnostics(db: &dyn SemanticDb, project: Project) -> DiagnosticCollection {
    let mut coll = DiagnosticCollection::default();
    for file in project.modules(db).values() {
        let parsed = parse_file(db.upcast(), *file);
        coll.extend(parsed.diagnostics);
    }
    coll
}

#[salsa::tracked]
pub fn project_parsed_modules(
    db: &dyn SemanticDb,
    project: Project,
) -> HashMap<String, ParsedModule> {
    let mut parsed = HashMap::new();
    for (name, file) in project.modules(db) {
        let parsed_module = parse_file(db.upcast(), file);
        parsed.insert(name.clone(), parsed_module.module);
    }
    parsed
}

#[salsa::tracked]
pub fn project_import_graph(db: &dyn SemanticDb, project: Project) -> HashMap<String, Vec<String>> {
    let parsed = project_parsed_modules(db, project);
    let mut graph = HashMap::new();
    for (module_name, parsed_module) in parsed.iter() {
        let mut imports = Vec::new();
        for item in parsed_module.items() {
            if let TopLevelItem::Use(use_spanned) = item {
                let use_stmt = use_spanned.value();
                let path_len = use_stmt.path.len();
                if path_len > 0 {
                    let imported_module = use_stmt
                        .path
                        .iter()
                        .map(|p| p.value().clone())
                        .collect::<Vec<_>>()
                        .join("::");
                    imports.push(imported_module);
                }
            }
        }
        graph.insert(module_name.clone(), imports);
    }
    graph
}

fn detect_import_cycle(graph: &HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    let mut visited: HashMap<String, i32> = HashMap::new();
    let mut path: Vec<String> = Vec::new();
    let mut cycle: Option<Vec<String>> = None;

    fn dfs(
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashMap<String, i32>,
        path: &mut Vec<String>,
        node: &String,
        cycle: &mut Option<Vec<String>>,
    ) {
        visited.insert(node.clone(), 1);
        path.push(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                match visited.entry(neighbor.clone()) {
                    Entry::Vacant(e) => {
                        e.insert(0);
                        dfs(graph, visited, path, neighbor, cycle);
                    }
                    Entry::Occupied(e) if *e.get() == 1 => {
                        let idx = path.iter().position(|n| n == neighbor).unwrap();
                        let mut c = path[idx..].to_vec();
                        c.push(neighbor.clone());
                        *cycle = Some(c);
                        return;
                    }
                    _ => {}
                }
                if cycle.is_some() {
                    return;
                }
            }
        }

        path.pop();
        visited.insert(node.clone(), 2);
    }

    for node in graph.keys() {
        if !visited.contains_key(node) {
            dfs(graph, &mut visited, &mut path, node, &mut cycle);
            if cycle.is_some() {
                return cycle;
            }
        }
    }
    cycle
}

fn topological_sort(graph: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut visited: HashMap<String, bool> = HashMap::new();
    let mut topo: Vec<String> = Vec::new();

    fn dfs_topo(
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashMap<String, bool>,
        topo: &mut Vec<String>,
        node: &String,
    ) {
        visited.insert(node.clone(), true);

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains_key(neighbor) {
                    dfs_topo(graph, visited, topo, neighbor);
                }
            }
        }

        topo.push(node.clone());
    }

    for node in graph.keys() {
        if !visited.contains_key(node) {
            dfs_topo(graph, &mut visited, &mut topo, node);
        }
    }

    // Don't reverse - we want dependencies before dependents
    topo
}

#[salsa::tracked]
pub fn project_semantic_index(
    db: &dyn SemanticDb,
    project: Project,
) -> Result<std::sync::Arc<ProjectSemanticIndex>, DiagnosticCollection> {
    let graph = project_import_graph(db, project);

    if let Some(cycle) = detect_import_cycle(&graph) {
        let mut coll = DiagnosticCollection::default();
        coll.add(Diagnostic::error(
            DiagnosticCode::SyntaxError,
            format!("Cyclic import: {}", cycle.join(" -> ")),
        ));
        return Err(coll);
    }

    let topo = topological_sort(&graph);
    let mut module_indices = HashMap::new();

    for module_name in topo {
        // Only process modules that actually exist in the project
        if project.modules(db).contains_key(&module_name) {
            let module_index = module_semantic_index(db, project, module_name.clone());
            module_indices.insert(module_name, module_index);
        }
    }

    Ok(std::sync::Arc::new(ProjectSemanticIndex::new(
        module_indices,
    )))
}

#[salsa::tracked]
pub fn module_semantic_index(
    db: &dyn SemanticDb,
    project: Project,
    module_name: String,
) -> SemanticIndex {
    let parsed_modules = project_parsed_modules(db, project);
    let parsed_module = parsed_modules
        .get(&module_name)
        .cloned()
        .expect("Module should exist");
    let file = *project
        .modules(db)
        .get(&module_name)
        .expect("File should exist");
    semantic_index_from_module(db, &parsed_module, file, project, module_name)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[salsa::db]
    #[derive(Clone)]
    pub struct TestDb {
        storage: salsa::Storage<Self>,
    }

    impl salsa::Database for TestDb {}
    #[salsa::db]
    impl ParserDb for TestDb {}
    #[salsa::db]
    impl SemanticDb for TestDb {}

    impl Upcast<dyn ParserDb> for TestDb {
        fn upcast(&self) -> &(dyn ParserDb + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
            self
        }
    }

    pub fn test_db() -> TestDb {
        TestDb {
            storage: salsa::Storage::default(),
        }
    }

    // For tests only - ideally not present there
    fn single_file_project(db: &dyn SemanticDb, file: File) -> Project {
        let mut modules = HashMap::new();
        modules.insert("main".to_string(), file);
        Project::new(db, modules, "main".to_string())
    }

    pub fn project_from_program(db: &dyn SemanticDb, program: &str) -> Project {
        let file = File::new(db, program.to_string(), "test.cm".to_string());
        single_file_project(db, file)
    }
}
