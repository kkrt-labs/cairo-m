use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cairo_m_compiler_semantic::db::{project_parse_diagnostics, project_validate_semantics};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use tracing::{debug, error, info};

use crate::db::{AnalysisDatabase, ProjectCrate, ProjectCrateExt};
use crate::diagnostics::state::ProjectDiagnostics;
use crate::project::ProjectModel;

/// Request types for the diagnostics controller
#[derive(Debug, Clone)]
pub enum DiagnosticsRequest {
    /// Recompute diagnostics for a specific file
    FileChanged { uri: Url, version: Option<i32> },

    /// Recompute diagnostics for an entire project
    ProjectChanged { project_crate: ProjectCrate },

    /// Shutdown the controller
    Shutdown,
}

/// Response from diagnostics computation
#[derive(Debug)]
pub struct DiagnosticsResponse {
    pub uri: Url,
    pub version: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Controller for computing diagnostics in a background task
pub struct DiagnosticsController {
    pub sender: UnboundedSender<DiagnosticsRequest>,
    handle: Option<JoinHandle<()>>,
}

impl DiagnosticsController {
    /// Create a new diagnostics controller
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: Arc<ProjectDiagnostics>,
        project_model: Arc<ProjectModel>,
        response_sender: UnboundedSender<DiagnosticsResponse>,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let _error_response_sender = response_sender.clone();
        let handle = tokio::spawn(async move {
            info!("DiagnosticsController worker task started");

            // Process requests until channel is closed
            while let Some(request) = receiver.recv().await {
                info!("Received request: {:?}", request);
                match request {
                    DiagnosticsRequest::FileChanged { uri, version } => {
                        info!("FileChanged: Processing diagnostics for file: {}", uri);
                        Self::compute_file_diagnostics(
                            &db,
                            &diagnostics_state,
                            &project_model,
                            uri,
                            version,
                            &response_sender,
                        )
                        .await;
                    }

                    DiagnosticsRequest::ProjectChanged { project_crate } => {
                        debug!("Processing diagnostics for entire project");
                        let start = Instant::now();
                        Self::compute_project_diagnostics(
                            &db,
                            &diagnostics_state,
                            project_crate,
                            &response_sender,
                            None,
                        )
                        .await;
                        debug!("Project diagnostics completed in {:?}", start.elapsed());
                    }

                    DiagnosticsRequest::Shutdown => {
                        info!("DiagnosticsController shutting down");
                        break;
                    }
                }
            }

            info!("DiagnosticsController worker task stopped");
        });

        Self {
            sender,
            handle: Some(handle),
        }
    }

    /// Send a diagnostics request
    pub fn request(
        &self,
        request: DiagnosticsRequest,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<DiagnosticsRequest>> {
        self.sender.send(request)
    }

    /// Compute diagnostics for a single file
    async fn compute_file_diagnostics(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &Arc<ProjectDiagnostics>,
        project_model: &Arc<ProjectModel>,
        uri: Url,
        version: Option<i32>,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
    ) {
        debug!("Computing diagnostics for file: {}", uri);

        // First try to get the ProjectCrate for this file (async call)
        if let Some(project_crate) = project_model.get_project_crate_for_file(&uri).await {
            info!("Found project crate for file, running project diagnostics");
            // We have a project, run full project diagnostics
            Self::compute_project_diagnostics(
                db,
                diagnostics_state,
                project_crate,
                response_sender,
                version,
            )
            .await;
        } else {
            info!(
                "No project crate found for file: {}, clearing diagnostics",
                uri
            );
            // No project found, just clear diagnostics for this file
            diagnostics_state.set_diagnostics(&uri, vec![]);

            let _ = response_sender.send(DiagnosticsResponse {
                uri,
                version,
                diagnostics: vec![],
            });
        }
    }

    /// Compute diagnostics for an entire project (async wrapper)
    async fn compute_project_diagnostics(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &Arc<ProjectDiagnostics>,
        project_crate: ProjectCrate,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
    ) {
        let db_clone = Arc::clone(db);
        let diagnostics_state_clone = Arc::clone(diagnostics_state);
        let response_sender_clone = response_sender.clone();

        tokio::task::spawn_blocking(move || {
            Self::compute_project_diagnostics_sync(
                &db_clone,
                &diagnostics_state_clone,
                project_crate,
                &response_sender_clone,
                version,
            );
        })
        .await
        .unwrap_or_else(|e| {
            error!(
                "Failed to spawn blocking task for project diagnostics: {:?}",
                e
            );
        });
    }

    /// Compute diagnostics for an entire project (synchronous version)
    fn compute_project_diagnostics_sync(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &Arc<ProjectDiagnostics>,
        project_crate: ProjectCrate,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
    ) {
        // Wrap the entire operation in catch_unwind to prevent panics from poisoning the mutex
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Extract necessary data with minimal lock time
            let (files_with_content, parser_diagnostics, semantic_diagnostics) =
                Self::collect_diagnostics_from_db(db, &project_crate);

            // Convert diagnostics to LSP format
            let diagnostics_by_file = Self::convert_diagnostics_to_lsp(
                &files_with_content,
                &parser_diagnostics,
                &semantic_diagnostics,
            );

            // Update state and send responses
            Self::publish_diagnostics(
                diagnostics_by_file,
                diagnostics_state,
                response_sender,
                version,
            );
        }));

        // Handle panic in diagnostics computation
        if let Err(panic_payload) = result {
            error!("Panic in diagnostics computation: {:?}", panic_payload);
            error!(
                "This indicates a bug in the compiler or semantic analysis - the mutex should not be poisoned anymore"
            );
        }
    }

    /// Collect diagnostics from the database with minimal lock time
    fn collect_diagnostics_from_db(
        db: &Arc<Mutex<AnalysisDatabase>>,
        project_crate: &ProjectCrate,
    ) -> (
        HashMap<PathBuf, String>,
        cairo_m_compiler_diagnostics::DiagnosticCollection,
        cairo_m_compiler_diagnostics::DiagnosticCollection,
    ) {
        let db_guard = match db.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                debug!("Database was poisoned, recovering from panic");
                poisoned.into_inner()
            }
        };

        let files = project_crate.files(&*db_guard);
        debug!(
            "Converting ProjectCrate with {} files for validation",
            files.len()
        );
        for (path, _) in files {
            debug!("  File: {}", path.display());
        }

        // First run parser validation
        let semantic_crate = project_crate.to_semantic_crate(&*db_guard);
        let parser_diagnostics = project_parse_diagnostics(&*db_guard, semantic_crate);

        debug!(
            "Parser validation found {} diagnostics",
            parser_diagnostics.all().len()
        );

        // Check if there are fatal parser errors
        let has_fatal_errors = Self::has_fatal_parser_errors(&parser_diagnostics);

        // Only run semantic validation if no fatal parser errors
        let semantic_diagnostics = if !has_fatal_errors {
            let semantic_crate = project_crate.to_semantic_crate(&*db_guard);
            let collection = project_validate_semantics(&*db_guard, semantic_crate);
            debug!(
                "Semantic validation found {} diagnostics",
                collection.all().len()
            );
            collection
        } else {
            debug!("Skipping semantic validation due to parser errors");
            cairo_m_compiler_diagnostics::DiagnosticCollection::new(Vec::new())
        };

        // Clone file contents while we have the lock
        let files = project_crate.files(&*db_guard);
        let mut files_with_content = HashMap::new();
        for (path, source_file) in files {
            let content = source_file.text(&*db_guard).to_string();
            files_with_content.insert(path.clone(), content);
        }

        (files_with_content, parser_diagnostics, semantic_diagnostics)
    }

    /// Check if there are fatal parser errors that would prevent semantic analysis
    fn has_fatal_parser_errors(
        parser_diagnostics: &cairo_m_compiler_diagnostics::DiagnosticCollection,
    ) -> bool {
        parser_diagnostics.all().iter().any(|d| {
            matches!(
                d.severity,
                cairo_m_compiler_diagnostics::DiagnosticSeverity::Error
            )
        })
    }

    /// Convert Cairo diagnostics to LSP format
    fn convert_diagnostics_to_lsp(
        files_with_content: &HashMap<PathBuf, String>,
        parser_diagnostics: &cairo_m_compiler_diagnostics::DiagnosticCollection,
        semantic_diagnostics: &cairo_m_compiler_diagnostics::DiagnosticCollection,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        let mut diagnostics_by_file: HashMap<Url, Vec<Diagnostic>> = HashMap::new();

        // Initialize with empty diagnostics for all files
        for file_path in files_with_content.keys() {
            if let Ok(uri) = Self::get_uri_from_path_str(&file_path.to_string_lossy()) {
                diagnostics_by_file.insert(uri, vec![]);
            }
        }

        let total_diagnostics = parser_diagnostics.all().len() + semantic_diagnostics.all().len();
        debug!(
            "Converting {} total diagnostics to LSP format ({} parser + {} semantic)",
            total_diagnostics,
            parser_diagnostics.all().len(),
            semantic_diagnostics.all().len()
        );

        // Process parser diagnostics
        Self::process_diagnostic_collection(
            parser_diagnostics,
            files_with_content,
            &mut diagnostics_by_file,
            "parser",
        );

        // Process semantic diagnostics
        Self::process_diagnostic_collection(
            semantic_diagnostics,
            files_with_content,
            &mut diagnostics_by_file,
            "semantic",
        );

        diagnostics_by_file
    }

    /// Process a collection of diagnostics and add them to the diagnostics map
    fn process_diagnostic_collection(
        diagnostics: &cairo_m_compiler_diagnostics::DiagnosticCollection,
        files_with_content: &HashMap<PathBuf, String>,
        diagnostics_by_file: &mut HashMap<Url, Vec<Diagnostic>>,
        diagnostic_type: &str,
    ) {
        for cairo_diag in diagnostics.all() {
            let uri = match Self::get_uri_from_path_str(&cairo_diag.file_path) {
                Ok(uri) => uri,
                Err(e) => {
                    debug!("Warning: {}", e);
                    continue;
                }
            };

            debug!("Processing {} diagnostic for URI: {}", diagnostic_type, uri);

            // Find the source file content
            let path_buf = Self::get_path_from_diagnostic(&uri, &cairo_diag.file_path);
            let path_buf = match path_buf {
                Some(path) => path,
                None => continue,
            };

            if let Some(content) = files_with_content.get(&path_buf) {
                let lsp_diag = convert_cairo_diagnostic(content, cairo_diag);
                debug!(
                    "Converted {} diagnostic: {:?} -> LSP range {:?}",
                    diagnostic_type, cairo_diag.message, lsp_diag.range
                );

                diagnostics_by_file.entry(uri).or_default().push(lsp_diag);
            } else {
                debug!(
                    "Warning: No content found for file path: {} (URI: {})",
                    path_buf.display(),
                    uri
                );
            }
        }
    }

    /// Get PathBuf from diagnostic file path and URI
    fn get_path_from_diagnostic(uri: &Url, file_path: &str) -> Option<PathBuf> {
        if file_path.starts_with("file://") {
            match uri.to_file_path() {
                Ok(path) => Some(path),
                Err(_) => {
                    debug!("Warning: Failed to convert URI to file path: {}", uri);
                    None
                }
            }
        } else {
            Some(PathBuf::from(file_path))
        }
    }

    /// Convert path string to URI
    fn get_uri_from_path_str(path_str: &str) -> Result<Url, String> {
        if path_str.starts_with("file://") {
            Url::parse(path_str).map_err(|e| format!("Failed to parse URI: {}", e))
        } else {
            Url::from_file_path(path_str)
                .map_err(|_| format!("Failed to convert path to URI: {}", path_str))
        }
    }

    /// Publish diagnostics to the client
    fn publish_diagnostics(
        diagnostics_by_file: HashMap<Url, Vec<Diagnostic>>,
        diagnostics_state: &ProjectDiagnostics,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
    ) {
        for (uri, diagnostics) in diagnostics_by_file {
            diagnostics_state.set_diagnostics(&uri, diagnostics.clone());

            let _ = response_sender.send(DiagnosticsResponse {
                uri,
                version,
                diagnostics,
            });
        }
    }
}

impl Drop for DiagnosticsController {
    fn drop(&mut self) {
        // Send shutdown signal
        let _ = self.sender.send(DiagnosticsRequest::Shutdown);

        // Abort the task since we can't await in Drop
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// Convert Cairo diagnostic to LSP diagnostic
pub fn convert_cairo_diagnostic(
    source: &str,
    diag: &cairo_m_compiler_diagnostics::Diagnostic,
) -> Diagnostic {
    use cairo_m_compiler_diagnostics::DiagnosticSeverity as CairoSeverity;

    let severity = match diag.severity {
        CairoSeverity::Error => DiagnosticSeverity::ERROR,
        CairoSeverity::Warning => DiagnosticSeverity::WARNING,
        CairoSeverity::Info => DiagnosticSeverity::INFORMATION,
        CairoSeverity::Hint => DiagnosticSeverity::HINT,
    };

    // Convert byte offsets to line/column positions
    let start_pos = offset_to_position(source, diag.span.start);
    let end_pos = offset_to_position(source, diag.span.end);

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
fn offset_to_position(source: &str, offset: usize) -> Position {
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
