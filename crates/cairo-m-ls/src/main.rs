#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use cairo_m_compiler::db::CompilerDatabase;
use cairo_m_compiler_diagnostics::{
    Diagnostic as CairoDiagnostic, DiagnosticSeverity as CairoSeverity,
};
use cairo_m_compiler_parser::{SourceFile, Upcast};
use cairo_m_compiler_semantic::db::{module_semantic_index, project_validate_semantics};
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::definition_semantic_type;
use cairo_m_compiler_semantic::types::{TypeData, TypeId};
use cairo_m_compiler_semantic::{DefinitionKind, Project, SemanticDb};
use dashmap::DashMap;
use salsa::Setter;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use walkdir::WalkDir;

/// Cached project discovery information
#[derive(Debug, Clone)]
struct ProjectCache {
    /// List of .cm files found in the project
    files: Vec<PathBuf>,
    /// Entry point file (main.cm if found, otherwise first file)
    entry_point: PathBuf,
    /// When this cache was last updated
    last_updated: SystemTime,
}

/// LSP Backend for Cairo-M
///
/// Uses Mutex for the database to ensure thread-safe access in async context.
/// The `source_files` map stores the Salsa input for each file, enabling
/// incremental compilation on file changes.
///
/// Project-aware: Maintains a mapping from project roots to shared project instances,
/// allowing proper cross-file analysis.
struct Backend {
    client: Client,
    db: Arc<Mutex<CompilerDatabase>>,
    source_files: Arc<DashMap<Url, SourceFile>>,
    /// Map from project root path to shared project instances
    projects: Arc<DashMap<PathBuf, Project>>,
    /// Cache for project file discovery to avoid re-scanning
    project_caches: Arc<DashMap<PathBuf, ProjectCache>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            db: Arc::new(Mutex::new(CompilerDatabase::default())),
            source_files: Arc::new(DashMap::new()),
            projects: Arc::new(DashMap::new()),
            project_caches: Arc::new(DashMap::new()),
        }
    }

    /// Find the project root for a given file URI.
    ///
    /// Traverses parent directories looking for a project marker like .git or cairom.toml.
    /// Returns the project root if found, otherwise the file's parent directory.
    fn find_project_root(&self, file_uri: &Url) -> Option<PathBuf> {
        let file_path = file_uri.to_file_path().ok()?;
        let mut current_dir = file_path.parent()?;

        loop {
            // Check for .git directory
            if current_dir.join(".git").exists() {
                return Some(current_dir.to_path_buf());
            }

            // Check for cairom.toml (future project config file)
            if current_dir.join("cairom.toml").exists() {
                return Some(current_dir.to_path_buf());
            }

            // Move to parent directory
            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => break,
            }
        }

        // If no project markers found, use the file's parent directory
        file_path.parent().map(|p| p.to_path_buf())
    }

    /// Discover all .cm files in a project directory recursively
    ///
    /// Returns a cached result if available and recent, otherwise performs a fresh scan.
    fn discover_project_files(&self, project_root: &PathBuf) -> Option<ProjectCache> {
        // Check if we have a recent cache
        if let Some(cache) = self.project_caches.get(project_root) {
            // Cache is valid for 30 seconds to avoid excessive re-scanning
            if let Ok(elapsed) = cache.last_updated.elapsed() {
                if elapsed.as_secs() < 30 {
                    return Some(cache.clone());
                }
            }
        }

        // Perform fresh discovery
        let mut cm_files = Vec::new();
        let mut main_file = None;

        // Walk the directory recursively looking for .cm files
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("cm") {
                cm_files.push(path.to_path_buf());

                // Check if this is main.cm
                if path.file_name().and_then(|name| name.to_str()) == Some("main.cm") {
                    main_file = Some(path.to_path_buf());
                }
            }
        }

        if cm_files.is_empty() {
            return None;
        }

        // Sort files for deterministic ordering
        cm_files.sort();

        // Determine entry point: prefer main.cm, fallback to first file
        let entry_point = main_file.unwrap_or_else(|| cm_files[0].clone());

        let cache = ProjectCache {
            files: cm_files,
            entry_point,
            last_updated: SystemTime::now(),
        };

        // Store in cache
        self.project_caches
            .insert(project_root.clone(), cache.clone());

        Some(cache)
    }

    /// Get or create a project for the given file URI.
    ///
    /// This implements project discovery and shared project state with caching.
    /// Files within the same project root will share the same Project instance.
    fn get_or_create_project(&self, file_uri: &Url) -> Option<Project> {
        let project_root = self.find_project_root(file_uri)?;

        // Check if we already have a project for this root
        if let Some(project) = self.projects.get(&project_root) {
            return Some(*project.value());
        }

        // Discover project files using cached recursive search
        let project_cache = self.discover_project_files(&project_root)?;
        let mut modules = HashMap::new();

        // Process all discovered .cm files
        for file_path in &project_cache.files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                let source_file = self.safe_db_access(|db| {
                    SourceFile::new(db, content, file_path.display().to_string())
                })?;

                // Use filename without extension as module name
                let module_name = file_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                modules.insert(module_name, source_file);

                // Also store in source_files map for LSP operations
                if let Ok(file_uri) = Url::from_file_path(file_path) {
                    self.source_files.insert(file_uri, source_file);
                }
            }
        }

        if modules.is_empty() {
            return None;
        }

        // Determine entry point based on discovered files
        let entry_point_name = project_cache
            .entry_point
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("main")
            .to_string();

        // Ensure the entry point exists in modules, fallback to first module if not
        #[allow(clippy::map_entry)]
        let main_module = if modules.contains_key(&entry_point_name) {
            entry_point_name
        } else {
            modules.keys().next()?.clone()
        };

        let project = self.safe_db_access(|db| Project::new(db, modules, main_module))?;
        self.projects.insert(project_root, project);
        Some(project)
    }

    /// Safely access the database, returning None if mutex is poisoned
    fn safe_db_access<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&cairo_m_compiler::db::CompilerDatabase) -> R,
    {
        self.db.lock().ok().map(|db| f(&db))
    }

    /// Safely access the mutable database, returning None if mutex is poisoned
    fn safe_db_access_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut cairo_m_compiler::db::CompilerDatabase) -> R,
    {
        self.db.lock().ok().map(|mut db| f(&mut db))
    }

    /// Convert Cairo diagnostic to LSP diagnostic
    fn convert_diagnostic(&self, source: &str, diag: &CairoDiagnostic) -> Diagnostic {
        let severity = match diag.severity {
            CairoSeverity::Error => DiagnosticSeverity::ERROR,
            CairoSeverity::Warning => DiagnosticSeverity::WARNING,
            CairoSeverity::Info => DiagnosticSeverity::INFORMATION,
            CairoSeverity::Hint => DiagnosticSeverity::HINT,
        };

        // Convert byte offsets to line/column positions
        let start_pos = self.offset_to_position(source, diag.span.start);
        let end_pos = self.offset_to_position(source, diag.span.end);

        let range = Range {
            start: start_pos,
            end: end_pos,
        };

        Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("cairo-m".to_string()),
            message: diag.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Convert byte offset to LSP Position
    fn offset_to_position(&self, source: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;

        for (i, ch) in source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    /// Convert LSP Position to byte offset
    fn position_to_offset(&self, source: &str, position: Position) -> usize {
        let mut current_line = 0;
        let mut current_character = 0;

        for (i, ch) in source.char_indices() {
            if current_line == position.line as usize
                && current_character == position.character as usize
            {
                return i;
            }

            if ch == '\n' {
                current_line += 1;
                current_character = 0;

                // If we're past the target line, the position doesn't exist
                if current_line > position.line as usize {
                    return source.len();
                }
            } else {
                current_character += 1;
            }
        }

        source.len()
    }

    /// Format a type for display in hover information
    fn format_type(db: &dyn SemanticDb, type_id: TypeId) -> String {
        match type_id.data(db) {
            TypeData::Felt => "felt".to_string(),
            TypeData::Pointer(inner) => format!("{}*", Self::format_type(db, inner)),
            TypeData::Tuple(types) => {
                if types.is_empty() {
                    "()".to_string()
                } else {
                    let formatted_types: Vec<String> =
                        types.iter().map(|t| Self::format_type(db, *t)).collect();
                    format!("({})", formatted_types.join(", "))
                }
            }
            TypeData::Function(_sig_id) => {
                // For now, just show "function" - we'd need to query the signature data
                "function".to_string()
            }
            TypeData::Struct(struct_id) => struct_id.name(db),
            TypeData::Unknown => "?".to_string(),
            TypeData::Error => "error".to_string(),
        }
    }

    /// Run semantic validation and publish diagnostics.
    ///
    /// Uses project-aware analysis to provide accurate cross-file diagnostics.
    async fn run_diagnostics(&self, uri: Url, version: Option<i32>) {
        let project = match self.get_or_create_project(&uri) {
            Some(project) => project,
            None => return, // Silently fail if project can't be created
        };

        let source = match self.source_files.get(&uri).map(|s| *s.value()) {
            Some(source) => source,
            None => return, // Silently fail if source file not found
        };

        let diagnostics = match self.safe_db_access(|db| {
            let content = source.text(db).to_owned();
            let semantic_diagnostics = project_validate_semantics(db, project);
            (content, semantic_diagnostics)
        }) {
            Some((content, semantic_diagnostics)) => semantic_diagnostics
                .iter()
                .map(|d| self.convert_diagnostic(&content, d))
                .collect(),
            None => {
                // Log error to client if database access fails
                self.client
                    .log_message(
                        MessageType::ERROR,
                        "Database access failed during diagnostics",
                    )
                    .await;
                return;
            }
        };

        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Cairo-M language server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Create a new SourceFile for the opened file. This requires a mutable
        // database lock because it allocates a new entity.
        let source = match self
            .safe_db_access(|db| SourceFile::new(db, content, path.display().to_string()))
        {
            Some(source) => source,
            None => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        "Failed to create source file due to database error",
                    )
                    .await;
                return;
            }
        };

        self.source_files.insert(uri.clone(), source);
        self.run_diagnostics(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next() {
            // Update the SourceFile with new content. This is the key to
            // incremental compilation.
            if let Some(source) = self.source_files.get(&uri).map(|s| *s.value()) {
                match self.safe_db_access_mut(|db| {
                    source.set_text(db).to(change.text);
                }) {
                    Some(_) => {
                        self.run_diagnostics(uri, Some(version)).await;
                    }
                    None => {
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                "Failed to update file content due to database error",
                            )
                            .await;
                    }
                }
            }
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get project for cross-file analysis
        let project = match self.get_or_create_project(&uri) {
            Some(project) => project,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map, do not create a new one.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let definition_location = self.safe_db_access(|db| {
            let content = source.text(db);
            let offset = self.position_to_offset(content, position);

            // Determine which module this file belongs to in the project
            let file_path = uri.to_file_path().ok();
            let module_name = file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())?;

            // Run semantic analysis query. Salsa will compute it incrementally.
            let index = module_semantic_index(db.upcast(), project, module_name);

            // Find the identifier at the cursor position.
            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((usage_idx, _)) = identifier_usage {
                // Get the definition for this usage.
                if let Some(definition) = index.get_use_definition(usage_idx) {
                    // Convert definition span to LSP location.
                    let start_pos = self.offset_to_position(content, definition.name_span.start);
                    let end_pos = self.offset_to_position(content, definition.name_span.end);

                    Some(Location {
                        uri,
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                    })
                } else {
                    None
                }
            } else {
                None
            }
        });

        Ok(definition_location
            .flatten()
            .map(GotoDefinitionResponse::Scalar))
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get project for cross-file analysis
        let project = match self.get_or_create_project(&uri) {
            Some(project) => project,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let hover_info = self.safe_db_access(|db| {
            let content = source.text(db);
            let offset = self.position_to_offset(content, position);

            // Determine which module this file belongs to in the project
            let file_path = uri.to_file_path().ok();
            let module_name = file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())?;

            // Run semantic analysis query incrementally.
            let index = module_semantic_index(db.upcast(), project, module_name);

            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((_usage_idx, usage)) = identifier_usage {
                if let Some((def_idx, _)) =
                    index.resolve_name_to_definition(&usage.name, usage.scope_id)
                {
                    let def_id = DefinitionId::new(db, source, def_idx);
                    let type_id = definition_semantic_type(db.upcast(), project, def_id);
                    let type_str = Self::format_type(db.upcast(), type_id);

                    let hover_text = format!("```cairo-m\n{}: {}\n```", usage.name, type_str);

                    Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_text,
                        }),
                        range: None,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        });

        Ok(hover_info.flatten())
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get project for cross-file analysis
        let project = match self.get_or_create_project(&uri) {
            Some(project) => project,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let completion_items = match self.safe_db_access(|db| {
            let content = source.text(db);
            let offset = self.position_to_offset(content, position);

            // Determine which module this file belongs to in the project
            let file_path = uri.to_file_path().ok();
            let module_name = file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())?;

            let index = module_semantic_index(db.upcast(), project, module_name);

            // Find the scope at the cursor position.
            // TODO: This is inefficient. A better approach would be to have a query
            // or a method on SemanticIndex to find the narrowest scope at an offset.
            let current_scope = {
                let mut best_scope = index.root_scope();
                let mut smallest_span_size = usize::MAX;

                for (scope_id, _) in index.scopes() {
                    for (_, def) in index.definitions_in_scope(scope_id) {
                        if def.full_span.start <= offset && offset <= def.full_span.end {
                            let span_size = def.full_span.end - def.full_span.start;
                            if span_size < smallest_span_size {
                                smallest_span_size = span_size;
                                best_scope = Some(scope_id);
                            }
                        }
                    }
                }

                best_scope.or_else(|| index.root_scope())?
            };

            let mut items = Vec::new();
            let mut seen_names = std::collections::HashSet::new();
            let mut scope = Some(current_scope);

            while let Some(scope_id) = scope {
                if let Some(place_table) = index.place_table(scope_id) {
                    for (_, place) in place_table.places() {
                        let name = place.name.clone();
                        if seen_names.insert(name.clone()) {
                            if let Some((def_idx, def)) =
                                index.resolve_name_to_definition(&name, scope_id)
                            {
                                let type_str = {
                                    let def_id = DefinitionId::new(db, source, def_idx);
                                    let type_id =
                                        definition_semantic_type(db.upcast(), project, def_id);
                                    Self::format_type(db.upcast(), type_id)
                                };

                                let kind = match def.kind {
                                    DefinitionKind::Function(_) => CompletionItemKind::FUNCTION,
                                    DefinitionKind::Parameter(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Local(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Let(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Const(_) => CompletionItemKind::CONSTANT,
                                    DefinitionKind::Struct(_) => CompletionItemKind::STRUCT,
                                    DefinitionKind::Use(_) => CompletionItemKind::MODULE,
                                    DefinitionKind::Namespace(_) => CompletionItemKind::MODULE,
                                    DefinitionKind::LoopVariable(_) => CompletionItemKind::VARIABLE,
                                };

                                items.push(CompletionItem {
                                    label: name,
                                    kind: Some(kind),
                                    detail: Some(type_str),
                                    documentation: None,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                scope = index.scope(scope_id).and_then(|s| s.parent);
            }

            // Add keywords
            let keywords = vec![
                ("if", CompletionItemKind::KEYWORD),
                ("else", CompletionItemKind::KEYWORD),
                ("while", CompletionItemKind::KEYWORD),
                ("loop", CompletionItemKind::KEYWORD),
                ("for", CompletionItemKind::KEYWORD),
                ("break", CompletionItemKind::KEYWORD),
                ("continue", CompletionItemKind::KEYWORD),
                ("return", CompletionItemKind::KEYWORD),
                ("let", CompletionItemKind::KEYWORD),
                ("local", CompletionItemKind::KEYWORD),
                ("const", CompletionItemKind::KEYWORD),
                ("func", CompletionItemKind::KEYWORD),
                ("struct", CompletionItemKind::KEYWORD),
                ("true", CompletionItemKind::KEYWORD),
                ("false", CompletionItemKind::KEYWORD),
                ("felt", CompletionItemKind::KEYWORD),
            ];

            for (keyword, kind) in keywords {
                items.push(CompletionItem {
                    label: keyword.to_string(),
                    kind: Some(kind),
                    ..Default::default()
                });
            }

            Some(items)
        }) {
            Some(Some(items)) => items,
            _ => Vec::new(), // Return empty completion list if database access fails
        };

        Ok(Some(CompletionResponse::Array(completion_items)))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
