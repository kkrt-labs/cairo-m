use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cairo_m_compiler_parser::{SourceFile, Upcast};
use cairo_m_compiler_semantic::DefinitionKind;
use cairo_m_compiler_semantic::db::module_semantic_index;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::definition_semantic_type;
use cairo_m_compiler_semantic::types::TypeId;
use dashmap::DashMap;
use salsa::Setter;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::db::{AnalysisDatabase, AnalysisDatabaseSwapper};
use crate::diagnostics::{DiagnosticsController, DiagnosticsRequest, ProjectDiagnostics};
use crate::lsp_ext::{ServerStatus, ServerStatusNotification, ServerStatusParams};
use crate::project::{ProjectController, ProjectModel, ProjectUpdate, ProjectUpdateRequest};

/// LSP Backend for Cairo-M
///
/// Uses Mutex for the database to ensure thread-safe access in async context.
/// The `source_files` map stores the Salsa input for each file, enabling
/// incremental compilation on file changes.
///
/// Project-aware: Maintains a mapping from project roots to shared project instances,
/// allowing proper cross-file analysis.
pub struct Backend {
    client: Client,
    db: Arc<Mutex<AnalysisDatabase>>,
    source_files: Arc<DashMap<Url, SourceFile>>,
    /// Reverse lookup map from file path to URI for O(1) access
    path_to_uri: Arc<DashMap<PathBuf, Url>>,
    /// Project controller for background project management
    project_controller: Option<ProjectController>,
    /// Central project model
    project_model: Arc<ProjectModel>,
    /// Diagnostics controller for background diagnostic computation
    diagnostics_controller: Option<DiagnosticsController>,
    /// Database swapper for memory management
    /// This field is intentionally kept alive to maintain the background swapping task.
    /// The swapper spawns a task that periodically resets the database to prevent
    /// unbounded memory growth. It must be stored to keep the task running.
    #[allow(dead_code)]
    _db_swapper: Option<AnalysisDatabaseSwapper>,
    /// Debounce timers for per-file diagnostics
    debounce_timers: Arc<DashMap<Url, JoinHandle<()>>>,
    /// Debounce delay in milliseconds
    debounce_delay_ms: Arc<AtomicU64>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
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
                    "internal://analysis-finished" => {
                        tracing::debug!("Received analysis-finished signal, sending notification.");
                        client_clone
                            .send_notification::<ServerStatusNotification>(ServerStatusParams {
                                status: ServerStatus::AnalysisFinished,
                            })
                            .await;
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
        let max_file_size = project_model.limits.max_file_size;
        let client_clone2: Client = client.clone();
        let project_model_clone = Arc::clone(&project_model);
        let db_clone = Arc::clone(&db);
        let source_files_clone = Arc::clone(&source_files);
        let path_to_uri = Arc::new(DashMap::new());
        let path_to_uri_clone = Arc::clone(&path_to_uri);
        let diagnostics_state_clone = Arc::clone(&diagnostics_state);
        let diagnostics_sender_clone = diagnostics_controller.sender.clone();

        tokio::spawn(async move {
            while let Some(update) = project_rx.recv().await {
                match update {
                    ProjectUpdate::Project {
                        project,
                        crate_info,
                        files,
                    } => {
                        tracing::info!("Loading project: {:?}", crate_info);
                        // Clone crate_info so we can use it later
                        let crate_info_for_later = crate_info.clone();

                        // Load the project into the model
                        // First, handle database operations in a blocking context
                        let _project_model_clone_for_db = Arc::clone(&project_model_clone);
                        let db_clone_for_db = Arc::clone(&db_clone);
                        let source_files_clone_for_db = Arc::clone(&source_files_clone);
                        let path_to_uri_clone_for_db = Arc::clone(&path_to_uri_clone);

                        // Prepare source files synchronously
                        #[allow(clippy::significant_drop_tightening)]
                        let prepared_files = tokio::task::spawn_blocking(move || {
                            let db = match db_clone_for_db.lock() {
                                Ok(db) => db,
                                Err(e) => {
                                    tracing::error!("Failed to acquire database lock: {:?}", e);
                                    return Err("Failed to acquire database lock".to_string());
                                }
                            };

                            let mut source_files_map = HashMap::new();
                            for path in &files {
                                if let Ok(uri) = Url::from_file_path(path) {
                                    let source_file = if let Some(sf) = source_files_clone_for_db.get(&uri) {
                                        *sf
                                    } else {
                                        // Check file size before reading
                                        if let Ok(metadata) = std::fs::metadata(path) {
                                            let file_size = metadata.len() as usize;
                                            // Default 10MB limit - could be made configurable
                                            if file_size > max_file_size {
                                                tracing::warn!(
                                                    "File {:?} exceeds size limit ({} > {} bytes), skipping",
                                                    path, file_size, max_file_size
                                                );
                                                continue;
                                            }
                                        }

                                        match std::fs::read_to_string(path) {
                                            Ok(content) => {
                                                let sf = cairo_m_compiler_parser::SourceFile::new(
                                                    &*db,
                                                    content,
                                                    uri.to_string(),
                                                );
                                                source_files_clone_for_db.insert(uri.clone(), sf);
                                                path_to_uri_clone_for_db.insert(path.clone(), uri.clone());
                                                sf
                                            }
                                            Err(e) => {
                                                tracing::warn!("Failed to read file {:?}: {}", path, e);
                                                continue;
                                            }
                                        }
                                    };
                                    source_files_map.insert(path.clone(), source_file);
                                }
                            }
                            Ok(source_files_map)
                        })
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("spawn_blocking task failed: {:?}", e);
                            Err("spawn_blocking task failed".to_string())
                        });

                        // Now load the crate asynchronously with prepared data
                        let load_result = match prepared_files {
                            Ok(source_files_map) => {
                                project_model_clone
                                    .load_crate_with_prepared_files_and_project(
                                        crate_info,
                                        source_files_map,
                                        &db_clone,
                                        Some(*project),
                                    )
                                    .await
                            }
                            Err(e) => Err(e),
                        };

                        // Process result after releasing the lock
                        match load_result {
                            Ok(moved_files) => {
                                // Clear diagnostics for files that moved between projects
                                if !moved_files.is_empty() {
                                    diagnostics_state_clone
                                        .clear_for_project(&moved_files)
                                        .await;
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
                                    if let Err(e) = diagnostics_sender_clone
                                        .send(DiagnosticsRequest::ProjectChanged { project_crate })
                                    {
                                        tracing::debug!(
                                            "Failed to send project changed diagnostics request: {}",
                                            e
                                        );
                                    }
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
                        // Load the standalone file
                        // First prepare the source file synchronously
                        let db_clone_for_standalone = Arc::clone(&db_clone);
                        let source_files_clone_for_standalone = Arc::clone(&source_files_clone);
                        let path_to_uri_clone_for_standalone = Arc::clone(&path_to_uri_clone);
                        let file_path_clone = file_path.clone();

                        #[allow(clippy::significant_drop_tightening)]
                        let prepared_file = tokio::task::spawn_blocking(move || {
                            let db = match db_clone_for_standalone.lock() {
                                Ok(db) => db,
                                Err(e) => {
                                    tracing::error!("Failed to acquire database lock: {:?}", e);
                                    return Err("Failed to acquire database lock".to_string());
                                }
                            };

                            if let Ok(uri) = Url::from_file_path(&file_path_clone) {
                                let source_file =
                                    if let Some(sf) = source_files_clone_for_standalone.get(&uri) {
                                        Some(*sf)
                                    } else {
                                        // Check file size before reading
                                        if let Ok(metadata) = std::fs::metadata(&file_path_clone) {
                                            let file_size = metadata.len() as usize;
                                            const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
                                            if file_size > MAX_FILE_SIZE {
                                                tracing::error!(
                                                    "File {:?} exceeds size limit ({} > {} bytes)",
                                                    file_path_clone,
                                                    file_size,
                                                    MAX_FILE_SIZE
                                                );
                                                return Err(format!(
                                                    "File exceeds size limit: {} bytes",
                                                    file_size
                                                ));
                                            }
                                        }

                                        match std::fs::read_to_string(&file_path_clone) {
                                            Ok(content) => {
                                                let sf = cairo_m_compiler_parser::SourceFile::new(
                                                    &*db,
                                                    content,
                                                    uri.to_string(),
                                                );
                                                source_files_clone_for_standalone
                                                    .insert(uri.clone(), sf);
                                                path_to_uri_clone_for_standalone
                                                    .insert(file_path_clone.clone(), uri);
                                                Some(sf)
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to read file {:?}: {}",
                                                    file_path_clone,
                                                    e
                                                );
                                                None
                                            }
                                        }
                                    };
                                Ok(source_file)
                            } else {
                                Err("Invalid file path".to_string())
                            }
                        })
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("spawn_blocking task failed: {:?}", e);
                            Err("spawn_blocking task failed".to_string())
                        });

                        // Now load the standalone file asynchronously
                        let load_result = match prepared_file {
                            Ok(Some(source_file)) => {
                                project_model_clone
                                    .load_standalone_with_prepared_file(
                                        &file_path,
                                        source_file,
                                        &db_clone,
                                    )
                                    .await
                            }
                            Ok(None) => Err("Failed to prepare source file".to_string()),
                            Err(e) => Err(e),
                        };

                        // Process result after releasing the lock
                        match load_result {
                            Ok(moved_files) => {
                                // Clear diagnostics for files that moved
                                if !moved_files.is_empty() {
                                    diagnostics_state_clone
                                        .clear_for_project(&moved_files)
                                        .await;
                                    // Publish empty diagnostics to clear client-side
                                    for uri in moved_files {
                                        client_clone2.publish_diagnostics(uri, vec![], None).await;
                                    }
                                }

                                // Trigger diagnostics for the loaded project
                                if let Some(project_crate) = project_model_clone
                                    .get_project_crate_for_root(&file_path)
                                    .await
                                {
                                    if let Err(e) = diagnostics_sender_clone
                                        .send(DiagnosticsRequest::ProjectChanged { project_crate })
                                    {
                                        tracing::debug!(
                                            "Failed to send project changed diagnostics request: {}",
                                            e
                                        );
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
            path_to_uri,
            project_controller: Some(project_controller),
            project_model,
            diagnostics_controller: Some(diagnostics_controller),
            _db_swapper: Some(db_swapper),
            debounce_timers: Arc::new(DashMap::new()),
            debounce_delay_ms: Arc::new(AtomicU64::new(300)), // Default to 300ms
        }
    }

    /// Safely access the mutable database using blocking lock with spawn_blocking
    ///
    /// Note: This implementation uses `spawn_blocking` rather than `try_lock` with exponential backoff.
    /// While try_lock patterns can reduce contention, spawn_blocking provides better guarantees:
    /// 1. No risk of starvation from repeated lock failures
    /// 2. Fair scheduling through Tokio's blocking thread pool
    /// 3. Automatic yielding of the async runtime thread
    /// 4. Simpler error handling without retry logic complexity
    ///
    /// The blocking thread pool overhead is acceptable for our use case where database
    /// operations are already relatively expensive (parsing, semantic analysis, etc).
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
        let delay_ms = self.debounce_delay_ms.load(Ordering::Relaxed);
        let request_sender = self.diagnostics_controller.as_ref().unwrap().sender.clone();
        let client_clone = self.client.clone();

        // Spawn the debounced task
        let handle = tokio::spawn(async move {
            // Wait for the debounce delay
            sleep(Duration::from_millis(delay_ms)).await;

            // Signal that analysis is starting
            client_clone
                .send_notification::<ServerStatusNotification>(ServerStatusParams {
                    status: ServerStatus::AnalysisStarted,
                })
                .await;

            // Send the diagnostic request (sync)
            tracing::debug!("Sending diagnostic request for URI: {}", uri_clone);
            if let Err(e) = request_sender.send(DiagnosticsRequest::FileChanged {
                uri: uri_clone.clone(),
                version: version_clone,
            }) {
                tracing::debug!("Failed to send file changed diagnostics request: {}", e);
            }
        });

        // Store the new timer handle
        self.debounce_timers.insert(uri, handle);
    }

    /// Run semantic validation and publish diagnostics.
    ///
    /// This now delegates to the DiagnosticsController for background computation.
    async fn run_diagnostics(&self, uri: Url, version: Option<i32>) {
        // Signal that analysis is starting
        self.client
            .send_notification::<ServerStatusNotification>(ServerStatusParams {
                status: ServerStatus::AnalysisStarted,
            })
            .await;
        // Request diagnostics from the controller
        if let Some(controller) = &self.diagnostics_controller {
            if let Err(e) = controller.request(DiagnosticsRequest::FileChanged { uri, version }) {
                tracing::debug!("Failed to send file changed diagnostics request: {}", e);
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Check for initialization options
        if let Some(options) = params.initialization_options {
            if let Some(debounce) = options.get("debounce_ms") {
                if let Some(debounce_value) = debounce.as_u64() {
                    self.debounce_delay_ms
                        .store(debounce_value, Ordering::Relaxed);
                }
            }
            // Note: db_swap_interval_ms would need to be handled during Backend construction
            // as the AnalysisDatabaseSwapper is created there. For testing purposes,
            // we'll need to make this configurable via a different mechanism.
        }
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
        tracing::debug!("Did open: {}", uri);
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

        // Update reverse lookup map
        self.path_to_uri.insert(path.clone(), uri.clone());

        // Request project update from the controller
        if let Some(controller) = &self.project_controller {
            if let Err(e) =
                controller.update(ProjectUpdateRequest::UpdateForFile { file_path: path })
            {
                tracing::debug!("Failed to send project update request: {}", e);
            }
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
                        if let DefinitionKind::Use(use_ref) = &def.kind {
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
                    let type_str = TypeId::format_type(db.upcast(), type_id);

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
                                    TypeId::format_type(db.upcast(), type_id)
                                };

                                let kind = match def.kind {
                                    DefinitionKind::Function(_) => CompletionItemKind::FUNCTION,
                                    DefinitionKind::Parameter(_) => CompletionItemKind::VARIABLE,
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
                ("fn", CompletionItemKind::KEYWORD),
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
