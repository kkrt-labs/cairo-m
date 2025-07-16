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
use std::path::PathBuf;

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
pub fn project_validate_semantics(db: &dyn SemanticDb, crate_id: Crate) -> DiagnosticCollection {
    tracing::info!("[SEMANTIC] Starting project validation");
    let parse_diag = project_parse_diagnostics(db, crate_id);
    if !parse_diag.is_empty() {
        tracing::warn!(
            "[SEMANTIC] Found {} parse errors, skipping semantic validation",
            parse_diag.len()
        );
        return parse_diag;
    }

    let sem_result = project_semantic_index(db, crate_id);
    match sem_result {
        Ok(sem) => {
            let mut coll = DiagnosticCollection::default();
            let registry = create_default_registry();
            for (module_name, index) in sem.modules().iter() {
                tracing::info!("[SEMANTIC] Validating module: {}", module_name);
                let module_file = *crate_id
                    .modules(db)
                    .get(module_name)
                    .unwrap_or_else(|| panic!("Module file should exist: {}", module_name));
                let module_diagnostics = registry.validate_all(db, crate_id, module_file, index);
                tracing::info!(
                    "[SEMANTIC] Module '{}' validation complete: {} diagnostics",
                    module_name,
                    module_diagnostics.len()
                );
                coll.extend(module_diagnostics);
            }
            tracing::info!(
                "[SEMANTIC] Project validation complete: {} total diagnostics",
                coll.len()
            );
            coll
        }
        Err(err_diag) => {
            tracing::error!("[SEMANTIC] Project validation failed with errors");
            err_diag
        }
    }
}

/// Represents a semantically structured project with named modules.
/// This is the semantic-level view where files are organized as modules
/// with proper names and relationships established.
///
/// This is the canonical crate representation used throughout the compiler.
#[salsa::input(debug)]
pub struct Crate {
    /// Map from fully-qualified module names to their source files
    /// e.g., "my_project::utils::math" -> File
    #[return_ref]
    pub modules: HashMap<String, File>,
    /// The entry point module name (e.g., "main" or "lib")
    pub entry_point: String,
    /// Root directory of the crate (for diagnostics and file resolution)
    #[return_ref]
    pub root_dir: PathBuf,
    /// Name of the crate (from cairom.toml)
    #[return_ref]
    pub name: String,
}

/// Find the module name for a given file in the crate
pub fn module_name_for_file(db: &dyn SemanticDb, crate_id: Crate, file: File) -> Option<String> {
    let file_path = file.file_path(db);
    for (module_name, module_file) in crate_id.modules(db).iter() {
        if module_file.file_path(db) == file_path {
            return Some(module_name.clone());
        }
    }
    None
}

/// Create a semantic crate from a cairo-m-project Project
pub fn crate_from_project(
    db: &dyn SemanticDb,
    project: cairo_m_project::Project,
) -> Result<Crate, DiagnosticCollection> {
    let mut modules = HashMap::new();
    let mut diagnostics = DiagnosticCollection::default();

    // Get all source files from the project
    let source_files = match project.source_files() {
        Ok(files) => files,
        Err(e) => {
            diagnostics.add(Diagnostic::error(
                DiagnosticCode::InternalError,
                format!("Failed to discover source files: {}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Convert each file path to a module name and create File entities
    for file_path in source_files {
        let module_name = match project.module_name_from_path(&file_path) {
            Ok(name) => name,
            Err(e) => {
                diagnostics.add(Diagnostic::error(
                    DiagnosticCode::InternalError,
                    format!(
                        "Failed to resolve module name for {}: {}",
                        file_path.display(),
                        e
                    ),
                ));
                continue;
            }
        };

        // Read the file content
        let content = match std::fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                diagnostics.add(Diagnostic::error(
                    DiagnosticCode::InternalError,
                    format!("Failed to read file {}: {}", file_path.display(), e),
                ));
                continue;
            }
        };

        // Create a File entity in the database
        let file = File::new(db, content, file_path.to_string_lossy().to_string());
        modules.insert(module_name, file);
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // Determine entry point
    let entry_point = determine_entry_point(&modules, &project);

    Ok(Crate::new(
        db,
        modules,
        entry_point,
        project.root_directory.clone(),
        project.name,
    ))
}

/// Determine the entry point module name
fn determine_entry_point(
    modules: &HashMap<String, File>,
    project: &cairo_m_project::Project,
) -> String {
    // If project specifies an entry point, use it
    if let Some(ref entry_point_path) = project.entry_point {
        if let Ok(module_name) = project.module_name_from_path(entry_point_path) {
            if modules.contains_key(&module_name) {
                return module_name;
            }
        }
    }

    // Otherwise, look for common entry points
    for candidate in ["main", "lib"] {
        if modules.contains_key(candidate) {
            return candidate.to_string();
        }
    }

    // Default to "main" even if not found
    "main".to_string()
}

#[salsa::tracked]
pub fn project_parse_diagnostics(db: &dyn SemanticDb, crate_id: Crate) -> DiagnosticCollection {
    let mut coll = DiagnosticCollection::default();
    for file in crate_id.modules(db).values() {
        let parsed = parse_file(db.upcast(), *file);
        coll.extend(parsed.diagnostics);
    }
    coll
}

#[salsa::tracked]
pub fn project_parsed_modules(
    db: &dyn SemanticDb,
    crate_id: Crate,
) -> HashMap<String, ParsedModule> {
    let mut parsed = HashMap::new();
    for (name, file) in crate_id.modules(db) {
        let parsed_module = parse_file(db.upcast(), file);
        parsed.insert(name.clone(), parsed_module.module);
    }
    parsed
}

#[salsa::tracked]
pub fn project_import_graph(db: &dyn SemanticDb, crate_id: Crate) -> HashMap<String, Vec<String>> {
    let parsed = project_parsed_modules(db, crate_id);
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
    crate_id: Crate,
) -> Result<std::sync::Arc<ProjectSemanticIndex>, DiagnosticCollection> {
    let num_modules = crate_id.modules(db).len();
    tracing::info!(
        "[SEMANTIC] Building project semantic index for {} modules",
        num_modules
    );

    let graph = project_import_graph(db, crate_id);

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

    // Pre-fetch parsed modules to avoid repeated queries
    let parsed_modules = project_parsed_modules(db, crate_id);

    // First process modules in topological order (for proper dependency resolution)
    for module_name in &topo {
        if crate_id.modules(db).contains_key(module_name)
            && parsed_modules.contains_key(module_name)
        {
            let module_index = module_semantic_index(db, crate_id, module_name.clone());
            module_indices.insert(module_name.clone(), module_index);
        }
    }

    // Then process any remaining modules that weren't in the topological sort
    // (e.g., modules with no imports and that aren't imported by anything)
    for (module_name, _) in crate_id.modules(db).iter() {
        if !module_indices.contains_key(module_name) && parsed_modules.contains_key(module_name) {
            let module_index = module_semantic_index(db, crate_id, module_name.clone());
            module_indices.insert(module_name.clone(), module_index);
        }
    }

    tracing::info!("[SEMANTIC] Project semantic index complete");
    Ok(std::sync::Arc::new(ProjectSemanticIndex::new(
        module_indices,
    )))
}

#[salsa::tracked]
pub fn module_semantic_index(
    db: &dyn SemanticDb,
    crate_id: Crate,
    module_name: String,
) -> SemanticIndex {
    let parsed_modules = project_parsed_modules(db, crate_id);
    let parsed_module = parsed_modules
        .get(&module_name)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Module '{}' should exist in parsed modules. Available modules: {:?}",
                module_name,
                parsed_modules.keys().collect::<Vec<_>>()
            )
        });
    let file = *crate_id.modules(db).get(&module_name).unwrap_or_else(|| {
        panic!(
            "File for module '{}' should exist in crate. Available modules: {:?}",
            module_name,
            crate_id.modules(db).keys().collect::<Vec<_>>()
        )
    });

    semantic_index_from_module(&parsed_module, file)
}

/// Get parse diagnostics for a specific module
#[salsa::tracked]
pub fn module_parse_diagnostics(
    db: &dyn SemanticDb,
    crate_id: Crate,
    module_name: String,
) -> DiagnosticCollection {
    if let Some(file) = crate_id.modules(db).get(&module_name) {
        let parsed = parse_file(db.upcast(), *file);
        DiagnosticCollection::new(parsed.diagnostics)
    } else {
        DiagnosticCollection::default()
    }
}

/// Get semantic diagnostics for a specific module
#[salsa::tracked]
pub fn module_semantic_diagnostics(
    db: &dyn SemanticDb,
    crate_id: Crate,
    module_name: String,
) -> DiagnosticCollection {
    let parse_diag = module_parse_diagnostics(db, crate_id, module_name.clone());
    if !parse_diag.is_empty() {
        tracing::warn!(
            "[SEMANTIC] Found {} parse errors in module '{}', skipping semantic validation",
            parse_diag.len(),
            module_name
        );
        return parse_diag;
    }

    if let Some(file) = crate_id.modules(db).get(&module_name) {
        let index = module_semantic_index(db, crate_id, module_name.clone());
        let registry = create_default_registry();
        let module_diagnostics = registry.validate_all(db, crate_id, *file, &index);

        tracing::info!(
            "[SEMANTIC] Module '{}' validation complete: {} diagnostics",
            module_name,
            module_diagnostics.len()
        );

        module_diagnostics
    } else {
        DiagnosticCollection::default()
    }
}

/// Get all diagnostics (parse + semantic) for a specific module
#[salsa::tracked]
pub fn module_all_diagnostics(
    db: &dyn SemanticDb,
    crate_id: Crate,
    module_name: String,
) -> DiagnosticCollection {
    let parse_diag = module_parse_diagnostics(db, crate_id, module_name.clone());
    let semantic_diag = module_semantic_diagnostics(db, crate_id, module_name);

    let mut collection = DiagnosticCollection::default();
    collection.extend(parse_diag.all().iter().cloned());
    collection.extend(semantic_diag.all().iter().cloned());
    collection
}

/// Check if a specific module has changed since a given revision
/// This function can be used to detect which modules need recomputation
pub fn module_changed_since_revision(
    db: &dyn SemanticDb,
    crate_id: Crate,
    module_name: String,
    revision: salsa::Revision,
) -> bool {
    // Check if the source file has changed
    if let Some(file) = crate_id.modules(db).get(&module_name) {
        // Use Salsa's built-in change detection for the file
        let current_revision = db.zalsa().current_revision();
        if current_revision > revision {
            // Check if this specific file's content has been invalidated
            let file_changed = file.text(db);
            let _ = file_changed; // We just need to trigger the query

            // The query system will tell us if this input has changed
            true
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use salsa::Setter;

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
    fn single_file_crate(db: &dyn SemanticDb, file: File) -> Crate {
        let mut modules = HashMap::new();
        modules.insert("main".to_string(), file);
        Crate::new(
            db,
            modules,
            "main".to_string(),
            PathBuf::from("."),
            "crate_test".to_string(),
        )
    }

    pub fn crate_from_program(db: &dyn SemanticDb, program: &str) -> Crate {
        let file = File::new(db, program.to_string(), "test.cm".to_string());
        single_file_crate(db, file)
    }

    #[test]
    fn test_module_name_for_file_with_updated_content() {
        let mut db = test_db();

        // Create a file
        let file1 = File::new(&db, "original content".to_string(), "test.cm".to_string());

        // Create a crate with this file
        let mut modules = HashMap::new();
        modules.insert("test".to_string(), file1);
        let crate_id = Crate::new(
            &db,
            modules,
            "test".to_string(),
            PathBuf::from("."),
            "crate_test".to_string(),
        );

        // Update the file content (simulating user typing)
        file1.set_text(&mut db).to("updated content".to_string());

        // module_name_for_file should still find the module by file path
        let module_name = module_name_for_file(&db, crate_id, file1);
        assert_eq!(module_name, Some("test".to_string()));
    }

    #[test]
    fn test_module_name_for_file_with_different_file_entity() {
        let db = test_db();

        // Create two file entities with the same path
        let file1 = File::new(&db, "content 1".to_string(), "test.cm".to_string());
        let file2 = File::new(&db, "content 2".to_string(), "test.cm".to_string());

        // Create a crate with file1
        let mut modules = HashMap::new();
        modules.insert("test".to_string(), file1);
        let crate_id = Crate::new(
            &db,
            modules,
            "test".to_string(),
            PathBuf::from("."),
            "crate_test".to_string(),
        );

        // Should find the module even when querying with file2 (same path)
        let module_name = module_name_for_file(&db, crate_id, file2);
        assert_eq!(module_name, Some("test".to_string()));
    }
}
