#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

mod db;
mod diagnostics;
mod lsp_tracing;
mod project;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use cairo_m_compiler_parser::{SourceFile, Upcast};
use cairo_m_compiler_semantic::db::module_semantic_index;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::definition_semantic_type;
use cairo_m_compiler_semantic::types::{TypeData, TypeId};
use cairo_m_compiler_semantic::{DefinitionKind, SemanticDb};
use dashmap::DashMap;
use salsa::Setter;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing_subscriber::EnvFilter;

use crate::db::{AnalysisDatabase, AnalysisDatabaseSwapper};
use crate::diagnostics::{DiagnosticsController, DiagnosticsRequest, ProjectDiagnostics};
use crate::project::{ProjectController, ProjectModel, ProjectUpdate, ProjectUpdateRequest};

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
    db: Arc<Mutex<AnalysisDatabase>>,
    source_files: Arc<DashMap<Url, SourceFile>>,
    /// Project controller for background project management
    project_controller: Option<ProjectController>,
    /// Central project model
    project_model: Arc<ProjectModel>,
    /// Diagnostics controller for background diagnostic computation
    diagnostics_controller: Option<DiagnosticsController>,
    /// Database swapper for memory management
    #[allow(dead_code)]
    db_swapper: Option<AnalysisDatabaseSwapper>,
    /// Debounce timers for per-file diagnostics
    debounce_timers: Arc<DashMap<Url, JoinHandle<()>>>,
    /// Debounce delay in milliseconds
    debounce_delay_ms: u64,
}

impl Backend {
    fn new(client: Client) -> Self {
        // Create channel for project updates
        let (project_tx, mut project_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create project controller
        let project_controller = ProjectController::new(project_tx);

        // Create diagnostics state
        let diagnostics_state = Arc::new(ProjectDiagnostics::new());

        // Create channel for diagnostics responses
        let (diag_tx, mut diag_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create shared database
        let db = Arc::new(Mutex::new(AnalysisDatabase::default()));

        // Create project model
        let project_model = Arc::new(ProjectModel::new());

        // Create diagnostics controller
        let diagnostics_controller = DiagnosticsController::new(
            Arc::clone(&db),
            Arc::clone(&diagnostics_state),
            Arc::clone(&project_model),
            diag_tx,
        );

        // Create source files map
        let source_files = Arc::new(DashMap::new());

        // Create database swapper (5 minute interval)
        let db_swapper = AnalysisDatabaseSwapper::new(
            Arc::clone(&db),
            Arc::clone(&project_model),
            std::time::Duration::from_secs(300),
        );

        // Spawn dedicated task for continuous diagnostics monitoring
        let client_clone = client.clone();
        tokio::spawn(async move {
            while let Some(response) = diag_rx.recv().await {
                // Handle special URIs or publish
                match response.uri.as_str() {
                    "file:///thread-error/diagnostics" => {
                        tracing::error!("Diagnostics controller thread has died");
                        client_clone.show_message(MessageType::ERROR, "Diagnostics system has failed - please restart the language server").await;
                    }
                    "file:///health-check/diagnostics" => {
                        tracing::debug!("Diagnostics controller health check received");
                    }
                    _ => {
                        client_clone
                            .publish_diagnostics(
                                response.uri,
                                response.diagnostics,
                                response.version,
                            )
                            .await;
                    }
                }
            }
            tracing::error!("Diagnostics receiver channel closed unexpectedly");
        });

        // Spawn dedicated task for continuous project update monitoring
        let client_clone2 = client.clone();
        let project_model_clone = Arc::clone(&project_model);
        let db_clone = Arc::clone(&db);
        let source_files_clone = Arc::clone(&source_files);
        let diagnostics_state_clone = Arc::clone(&diagnostics_state);
        let diagnostics_sender_clone = diagnostics_controller.sender.clone();

        tokio::spawn(async move {
            tracing::info!("Starting dedicated project update monitor task");

            while let Some(update) = project_rx.recv().await {
                tracing::debug!("Project update monitor received update: {:?}", update);

                match update {
                    ProjectUpdate::Project { crate_info, files } => {
                        tracing::info!("Processing project update: {}", crate_info.name);

                        // Clone crate_info so we can use it later
                        let crate_info_for_later = crate_info.clone();

                        // Load the project into the model using spawn_blocking to avoid holding locks across await
                        let project_model_clone_for_task = Arc::clone(&project_model_clone);
                        let db_clone_for_task = Arc::clone(&db_clone);
                        let source_files_clone_for_task = Arc::clone(&source_files_clone);
                        let load_result = tokio::task::spawn_blocking(move || {
                            match db_clone_for_task.lock() {
                                Ok(mut db) => {
                                    // This is now a sync call inside spawn_blocking
                                    let rt = tokio::runtime::Handle::current();
                                    rt.block_on(project_model_clone_for_task.load_crate(
                                        crate_info,
                                        files,
                                        &mut db,
                                        |db, uri| {
                                            // Try to get existing source file or create new one
                                            if let Some(sf) = source_files_clone_for_task.get(uri) {
                                                Some(*sf)
                                            } else {
                                                let path = uri.to_file_path().ok()?;
                                                let content =
                                                    std::fs::read_to_string(&path).ok()?;
                                                let source_file =
                                                    cairo_m_compiler_parser::SourceFile::new(
                                                        db,
                                                        content,
                                                        uri.to_string(),
                                                    );
                                                source_files_clone_for_task
                                                    .insert(uri.clone(), source_file);
                                                Some(source_file)
                                            }
                                        },
                                    ))
                                }
                                Err(e) => {
                                    tracing::error!("Failed to acquire database lock: {:?}", e);
                                    Err("Failed to acquire database lock".to_string())
                                }
                            }
                        })
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("spawn_blocking task failed: {:?}", e);
                            Err("spawn_blocking task failed".to_string())
                        });

                        // Process result after releasing the lock
                        match load_result {
                            Ok(moved_files) => {
                                // Clear diagnostics for files that moved between projects
                                if !moved_files.is_empty() {
                                    diagnostics_state_clone.clear_for_project(&moved_files);
                                    // Publish empty diagnostics to clear client-side
                                    for uri in moved_files {
                                        client_clone2.publish_diagnostics(uri, vec![], None).await;
                                    }
                                }

                                // Trigger diagnostics for the loaded project
                                if let Some(project_crate) = project_model_clone
                                    .get_project_crate_for_root(&crate_info_for_later.root)
                                    .await
                                {
                                    let _ = diagnostics_sender_clone
                                        .send(DiagnosticsRequest::ProjectChanged { project_crate });
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to load project: {}", e);
                                client_clone2
                                    .show_message(
                                        MessageType::WARNING,
                                        &format!("Failed to load project: {}", e),
                                    )
                                    .await;
                            }
                        }
                    }
                    ProjectUpdate::Standalone(file_path) => {
                        tracing::info!(
                            "Processing standalone file update: {}",
                            file_path.display()
                        );

                        // Load the standalone file using spawn_blocking to avoid holding locks across await
                        let project_model_clone_for_task = Arc::clone(&project_model_clone);
                        let db_clone_for_task = Arc::clone(&db_clone);
                        let source_files_clone_for_task = Arc::clone(&source_files_clone);
                        let load_result = tokio::task::spawn_blocking(move || {
                            match db_clone_for_task.lock() {
                                Ok(mut db) => {
                                    // This is now a sync call inside spawn_blocking
                                    let rt = tokio::runtime::Handle::current();
                                    rt.block_on(project_model_clone_for_task.load_standalone(
                                        file_path,
                                        &mut db,
                                        |db, uri| {
                                            // Try to get existing source file or create new one
                                            if let Some(sf) = source_files_clone_for_task.get(uri) {
                                                Some(*sf)
                                            } else {
                                                let path = uri.to_file_path().ok()?;
                                                let content =
                                                    std::fs::read_to_string(&path).ok()?;
                                                let source_file =
                                                    cairo_m_compiler_parser::SourceFile::new(
                                                        db,
                                                        content,
                                                        uri.to_string(),
                                                    );
                                                source_files_clone_for_task
                                                    .insert(uri.clone(), source_file);
                                                Some(source_file)
                                            }
                                        },
                                    ))
                                }
                                Err(e) => {
                                    tracing::error!("Failed to acquire database lock: {:?}", e);
                                    Err("Failed to acquire database lock".to_string())
                                }
                            }
                        })
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("spawn_blocking task failed: {:?}", e);
                            Err("spawn_blocking task failed".to_string())
                        });

                        // Process result after releasing the lock
                        match load_result {
                            Ok(moved_files) => {
                                // Clear diagnostics for files that moved
                                if !moved_files.is_empty() {
                                    diagnostics_state_clone.clear_for_project(&moved_files);
                                    // Publish empty diagnostics to clear client-side
                                    for uri in moved_files {
                                        client_clone2.publish_diagnostics(uri, vec![], None).await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to load standalone file: {}", e);
                            }
                        }
                    }
                    ProjectUpdate::ThreadError(error_msg) => {
                        tracing::error!("Project controller thread error: {}", error_msg);
                        client_clone2
                            .show_message(
                                MessageType::ERROR,
                                &format!("Project discovery failed: {}", error_msg),
                            )
                            .await;
                    }
                }
            }

            tracing::error!("Project update monitor task exiting - channel closed");
            client_clone2
                .show_message(
                    MessageType::ERROR,
                    "Project controller thread has stopped unexpectedly",
                )
                .await;
        });

        Self {
            client,
            db,
            source_files,
            project_controller: Some(project_controller),
            project_model,
            diagnostics_controller: Some(diagnostics_controller),
            db_swapper: Some(db_swapper),
            debounce_timers: Arc::new(DashMap::new()),
            debounce_delay_ms: 300, // Default to 300ms
        }
    }

    /// Safely access the mutable database using blocking lock with spawn_blocking
    fn safe_db_access_mut<F, R>(&self, f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce(&mut AnalysisDatabase) -> R + Send + 'static,
        R: Send + 'static,
    {
        let db_clone = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut db = db_clone.lock().unwrap_or_else(|poisoned| {
                tracing::error!("Database mutex poisoned - recovering from panic");
                poisoned.into_inner()
            });
            f(&mut db)
        })
    }

    /// Synchronously access the database (for use in sync contexts)
    fn safe_db_access_sync<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&AnalysisDatabase) -> R,
    {
        match self.db.lock() {
            Ok(db) => Some(f(&db)),
            Err(poisoned) => {
                tracing::error!("Database mutex poisoned - recovering from panic");
                let db = poisoned.into_inner();
                Some(f(&db))
            }
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

    /// Helper for URI conversion from path strings that may already be URIs
    fn get_uri_from_path_str(&self, path_str: &str) -> std::result::Result<Url, String> {
        if path_str.starts_with("file://") {
            Url::parse(path_str).map_err(|e| format!("Failed to parse URI: {}", e))
        } else {
            Url::from_file_path(path_str)
                .map_err(|_| format!("Failed to convert path to URI: {}", path_str))
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

    /// Get the semantic crate for a file URL
    async fn get_semantic_crate_for_file(
        &self,
        uri: &Url,
    ) -> Option<cairo_m_compiler_semantic::Crate> {
        // Get the ProjectCrate from the model
        let project_crate = self.project_model.get_project_crate_for_file(uri).await?;

        // Convert to semantic crate
        self.safe_db_access_sync(|db| {
            use crate::db::ProjectCrateExt;
            project_crate.to_semantic_crate(db)
        })
    }

    /// Schedule debounced diagnostics for a file
    fn schedule_debounced_diagnostics(&self, uri: Url, version: Option<i32>) {
        // Cancel any existing timer for this file
        if let Some((_, handle)) = self.debounce_timers.remove(&uri) {
            handle.abort();
        }

        // Clone necessary components for the async task
        let uri_clone = uri.clone();
        let version_clone = version;
        let delay_ms = self.debounce_delay_ms;
        let request_sender = self.diagnostics_controller.as_ref().unwrap().sender.clone();

        // Spawn the debounced task
        let handle = tokio::spawn(async move {
            // Wait for the debounce delay
            sleep(Duration::from_millis(delay_ms)).await;

            // Send the diagnostic request (sync)
            tracing::debug!("Sending diagnostic request for URI: {}", uri_clone);
            let _ = request_sender.send(DiagnosticsRequest::FileChanged {
                uri: uri_clone.clone(),
                version: version_clone,
            });
        });

        // Store the new timer handle
        self.debounce_timers.insert(uri, handle);
    }

    /// Run semantic validation and publish diagnostics.
    ///
    /// This now delegates to the DiagnosticsController for background computation.
    async fn run_diagnostics(&self, uri: Url, version: Option<i32>) {
        // Request diagnostics from the controller
        if let Some(controller) = &self.diagnostics_controller {
            let _ = controller.request(DiagnosticsRequest::FileChanged { uri, version });
        }
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
        tracing::info!("Did open: {}", uri);
        let content = params.text_document.text;
        let version = params.text_document.version;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Create a new SourceFile for the opened file. This requires a mutable
        // database lock because it allocates a new entity.
        let uri_string = uri.to_string();
        let join_handle =
            self.safe_db_access_mut(move |db| SourceFile::new(db, content, uri_string));

        let source = match join_handle.await {
            Ok(source) => source,
            Err(e) => {
                tracing::error!("Failed to create source file: {:?}", e);
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

        // Request project update from the controller
        if let Some(controller) = &self.project_controller {
            let _ = controller.update(ProjectUpdateRequest::UpdateForFile { file_path: path });
        }

        // No need to process project updates here - the dedicated monitor task handles them continuously

        self.run_diagnostics(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next() {
            // Update the SourceFile with new content. This is the key to
            // incremental compilation.
            if let Some(source) = self.source_files.get(&uri).map(|s| *s.value()) {
                let change_text = change.text.clone();
                let join_handle = self.safe_db_access_mut(move |db| {
                    source.set_text(db).to(change_text);
                });

                match join_handle.await {
                    Ok(_) => {
                        // Schedule debounced diagnostics instead of immediate run
                        self.schedule_debounced_diagnostics(uri, Some(version));
                    }
                    Err(e) => {
                        tracing::error!("Failed to update file content: {:?}", e);
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

        // Get crate for cross-file analysis
        let crate_id = match self.get_semantic_crate_for_file(&uri).await {
            Some(crate_id) => crate_id,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map, do not create a new one.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let definition_location = self.safe_db_access_sync(|db| {
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
            let index = module_semantic_index(db.upcast(), crate_id, module_name);

            // First, check if we're clicking on an identifier usage
            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((usage_idx, usage)) = identifier_usage {
                // Try to resolve with imports for cross-module support
                if let Some((_def_idx, def, def_file)) = index.resolve_name_with_imports(
                    db.upcast(),
                    crate_id,
                    source,
                    &usage.name,
                    usage.scope_id,
                ) {
                    // If the definition is in a different file, create a location for that file
                    if def_file != source {
                        // Get the URI for the definition file
                        let def_path = def_file.file_path(db);
                        if let Ok(def_uri) = self.get_uri_from_path_str(def_path) {
                            let def_content = def_file.text(db);
                            let start_pos =
                                self.offset_to_position(def_content, def.name_span.start);
                            let end_pos = self.offset_to_position(def_content, def.name_span.end);

                            return Some(Location {
                                uri: def_uri,
                                range: Range {
                                    start: start_pos,
                                    end: end_pos,
                                },
                            });
                        }
                    }

                    // Same file definition
                    let start_pos = self.offset_to_position(content, def.name_span.start);
                    let end_pos = self.offset_to_position(content, def.name_span.end);

                    Some(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                    })
                } else if let Some(definition) = index.get_use_definition(usage_idx) {
                    // Fallback to local resolution (for backward compatibility)
                    let start_pos = self.offset_to_position(content, definition.name_span.start);
                    let end_pos = self.offset_to_position(content, definition.name_span.end);

                    Some(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                    })
                } else {
                    None
                }
            } else {
                // Check if we're clicking on a module name in a use statement
                // This requires parsing the source to find use statements

                // For now, we'll check all definitions to see if any are use statements at this position
                for (_def_idx, def) in index.all_definitions() {
                    if def.full_span.start <= offset && offset <= def.full_span.end {
                        if let crate::DefinitionKind::Use(use_ref) = &def.kind {
                            // Try to find the module file
                            let _module_path = format!("{}.cm", use_ref.imported_module);

                            // Search for the module file in the crate
                            for (mod_name, mod_file) in crate_id.modules(db).iter() {
                                if mod_name == &use_ref.imported_module {
                                    let mod_path = mod_file.file_path(db);
                                    if let Ok(mod_uri) = self.get_uri_from_path_str(mod_path) {
                                        // Navigate to the beginning of the module file
                                        return Some(Location {
                                            uri: mod_uri,
                                            range: Range {
                                                start: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                                end: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                            },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

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

        // Get crate for cross-file analysis
        let crate_id = match self.get_semantic_crate_for_file(&uri).await {
            Some(crate_id) => crate_id,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let hover_info = self.safe_db_access_sync(|db| {
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
            let index = module_semantic_index(db.upcast(), crate_id, module_name);

            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((_usage_idx, usage)) = identifier_usage {
                // Try to resolve with imports for cross-module support
                if let Some((def_idx, _def, def_file)) = index.resolve_name_with_imports(
                    db.upcast(),
                    crate_id,
                    source,
                    &usage.name,
                    usage.scope_id,
                ) {
                    let def_id = DefinitionId::new(db, def_file, def_idx);
                    let type_id = definition_semantic_type(db.upcast(), crate_id, def_id);
                    let type_str = Self::format_type(db.upcast(), type_id);

                    let mut hover_text = format!("```cairo-m\n{}: {}\n```", usage.name, type_str);

                    // Add module information if it's from a different file
                    if def_file != source {
                        if let Some(module_name) =
                            cairo_m_compiler_semantic::db::module_name_for_file(
                                db.upcast(),
                                crate_id,
                                def_file,
                            )
                        {
                            hover_text.push_str(&format!("\n\n*From module: {}*", module_name));
                        }
                    }

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

        // Get crate for cross-file analysis
        let crate_id = match self.get_semantic_crate_for_file(&uri).await {
            Some(crate_id) => crate_id,
            None => return Ok(None),
        };

        // Retrieve the SourceFile from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let completion_items = match self.safe_db_access_sync(|db| {
            let content = source.text(db);
            let offset = self.position_to_offset(content, position);

            // Determine which module this file belongs to in the project
            let file_path = uri.to_file_path().ok();
            let module_name = file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())?;

            let index = module_semantic_index(db.upcast(), crate_id, module_name);

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
                                        definition_semantic_type(db.upcast(), crate_id, def_id);
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
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let args = std::env::args().collect::<Vec<String>>();
    let _trace_level = args.get(1);

    let (service, socket) = LspService::build(|client| {
        // --- Start of new logging setup ---

        // 1. Create a channel for log messages.
        // The sender is passed to the tracing layer, and the receiver is used in a dedicated task.
        let (log_sender, mut log_receiver) = mpsc::unbounded_channel();

        // 2. Clone the client for the logging task.
        let client_clone = client.clone();

        // 3. Spawn a task to listen for log messages and forward them to the LSP client.
        // This task runs on the main Tokio runtime and can safely call async functions.
        tokio::spawn(async move {
            while let Some((level, message)) = log_receiver.recv().await {
                // tracing::info!("{}", message);
                client_clone.log_message(level, message).await;
            }
        });

        // 4. Create the LspTracingLayer with the sender part of the channel.
        let lsp_layer = lsp_tracing::LspTracingLayer::new(log_sender);

        // --- End of new logging setup ---

        let directives = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .parse_lossy("");

        tracing_subscriber::registry()
            .with(lsp_layer)
            .with(directives)
            .init();

        Backend::new(client)
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
