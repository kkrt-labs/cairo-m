use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use cairo_m_compiler_semantic::db::{project_parse_diagnostics, project_validate_semantics};
use crossbeam_channel::{Receiver, Sender};
use tokio::sync::mpsc::UnboundedSender;
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

    /// Clear all diagnostics
    Clear,

    /// Health check ping
    HealthCheck,

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

/// Controller for computing diagnostics in a background thread
pub struct DiagnosticsController {
    pub sender: Sender<DiagnosticsRequest>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DiagnosticsController {
    /// Create a new diagnostics controller
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: Arc<ProjectDiagnostics>,
        project_model: Arc<ProjectModel>,
        response_sender: UnboundedSender<DiagnosticsResponse>,
    ) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let error_response_sender = response_sender.clone();
        let handle = thread::Builder::new()
            .name("diagnostics-controller".to_string())
            .spawn(move || {
                // Catch panics to log them and notify the client
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Self::worker_thread(
                        db,
                        diagnostics_state,
                        project_model,
                        receiver,
                        response_sender,
                    );
                }));

                if let Err(e) = result {
                    error!("DiagnosticsController worker thread panicked: {:?}", e);

                    // Send an error response to notify the main thread
                    // Use a special URI to indicate thread failure
                    let error_uri =
                        tower_lsp::lsp_types::Url::parse("file:///thread-error/diagnostics")
                            .expect("Valid error URI");

                    let _ = error_response_sender.send(DiagnosticsResponse {
                        uri: error_uri,
                        version: None,
                        diagnostics: vec![], // Empty diagnostics indicate error
                    });
                }
            })
            .expect("Failed to spawn DiagnosticsController thread");

        Self {
            sender,
            handle: Some(handle),
        }
    }

    /// Send a diagnostics request
    pub fn request(
        &self,
        request: DiagnosticsRequest,
    ) -> Result<(), crossbeam_channel::SendError<DiagnosticsRequest>> {
        self.sender.send(request)
    }

    /// Worker thread that processes diagnostic requests
    fn worker_thread(
        db: Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: Arc<ProjectDiagnostics>,
        project_model: Arc<ProjectModel>,
        receiver: Receiver<DiagnosticsRequest>,
        response_sender: UnboundedSender<DiagnosticsResponse>,
    ) {
        info!("DiagnosticsController worker thread started");

        for request in receiver {
            info!("Received request: {:?}", request);
            match request {
                DiagnosticsRequest::FileChanged { uri, version } => {
                    info!("FileChainged: Processing diagnostics for file: {}", uri);
                    Self::compute_file_diagnostics(
                        &db,
                        &diagnostics_state,
                        &project_model,
                        uri,
                        version,
                        &response_sender,
                    );
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
                    );
                    debug!("Project diagnostics completed in {:?}", start.elapsed());
                }

                DiagnosticsRequest::Clear => {
                    info!("Clearing all diagnostics");
                    diagnostics_state.clear();
                }

                DiagnosticsRequest::HealthCheck => {
                    debug!("Health check received - diagnostics controller is alive");
                    // Send a health check response using a special URI
                    let health_uri =
                        tower_lsp::lsp_types::Url::parse("file:///health-check/diagnostics")
                            .expect("Valid health check URI");

                    let _ = response_sender.send(DiagnosticsResponse {
                        uri: health_uri,
                        version: None,
                        diagnostics: vec![],
                    });
                }

                DiagnosticsRequest::Shutdown => {
                    info!("DiagnosticsController shutting down");
                    break;
                }
            }
        }

        info!("DiagnosticsController worker thread stopped");
    }

    /// Compute diagnostics for a single file
    fn compute_file_diagnostics(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &ProjectDiagnostics,
        project_model: &ProjectModel,
        uri: Url,
        version: Option<i32>,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
    ) {
        debug!("Computing diagnostics for file: {}", uri);

        // Try to get the ProjectCrate for this file
        if let Some(project_crate) = project_model.get_project_crate_for_file(&uri) {
            info!("Found project crate for file, running project diagnostics");
            // We have a project, run full project diagnostics
            Self::compute_project_diagnostics(
                db,
                diagnostics_state,
                project_crate,
                response_sender,
                version,
            );
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

    /// Compute diagnostics for an entire project
    fn compute_project_diagnostics(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &ProjectDiagnostics,
        project_crate: ProjectCrate,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
    ) {
        // Wrap the entire operation in catch_unwind to prevent panics from poisoning the mutex
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Extract necessary data with minimal lock time
            let (files_with_content, parser_diagnostics, semantic_diagnostics) = {
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

                // Check if there are fatal parser errors (syntax errors that would break semantic analysis)
                let has_fatal_errors = parser_diagnostics.all().iter().any(|d| {
                    matches!(
                        d.severity,
                        cairo_m_compiler_diagnostics::DiagnosticSeverity::Error
                    )
                });

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
            }; // Lock is released here

            // Helper function to convert diagnostics
            fn get_uri_from_path_str(path_str: &str) -> Result<Url, String> {
                if path_str.starts_with("file://") {
                    Url::parse(path_str).map_err(|e| format!("Failed to parse URI: {}", e))
                } else {
                    Url::from_file_path(path_str)
                        .map_err(|_| format!("Failed to convert path to URI: {}", path_str))
                }
            }

            // Now process diagnostics without holding the lock
            let mut diagnostics_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
                std::collections::HashMap::new();

            // Initialize with empty diagnostics for all files
            for file_path in files_with_content.keys() {
                if let Ok(uri) = get_uri_from_path_str(&file_path.to_string_lossy()) {
                    diagnostics_by_file.insert(uri, vec![]);
                }
            }

            let total_diagnostics =
                parser_diagnostics.all().len() + semantic_diagnostics.all().len();
            debug!(
                "Converting {} total diagnostics to LSP format ({} parser + {} semantic)",
                total_diagnostics,
                parser_diagnostics.all().len(),
                semantic_diagnostics.all().len()
            );

            // Convert parser diagnostics first
            for cairo_diag in parser_diagnostics.all() {
                let uri = match get_uri_from_path_str(&cairo_diag.file_path) {
                    Ok(uri) => uri,
                    Err(e) => {
                        debug!("Warning: {}", e);
                        continue;
                    }
                };

                debug!("Processing parser diagnostic for URI: {}", uri);

                // Find the source file content
                let path_buf = if cairo_diag.file_path.starts_with("file://") {
                    match uri.to_file_path() {
                        Ok(path) => path,
                        Err(_) => {
                            debug!("Warning: Failed to convert URI to file path: {}", uri);
                            continue;
                        }
                    }
                } else {
                    PathBuf::from(&cairo_diag.file_path)
                };

                if let Some(content) = files_with_content.get(&path_buf) {
                    let lsp_diag = convert_cairo_diagnostic(content, cairo_diag);
                    debug!(
                        "Converted parser diagnostic: {:?} -> LSP range {:?}",
                        cairo_diag.message, lsp_diag.range
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

            // Convert semantic diagnostics
            for cairo_diag in semantic_diagnostics.all() {
                let uri = match get_uri_from_path_str(&cairo_diag.file_path) {
                    Ok(uri) => uri,
                    Err(e) => {
                        debug!("Warning: {}", e);
                        continue;
                    }
                };

                debug!("Processing semantic diagnostic for URI: {}", uri);

                let path_buf = if cairo_diag.file_path.starts_with("file://") {
                    match uri.to_file_path() {
                        Ok(path) => path,
                        Err(_) => {
                            debug!("Warning: Failed to convert URI to file path: {}", uri);
                            continue;
                        }
                    }
                } else {
                    PathBuf::from(&cairo_diag.file_path)
                };

                if let Some(content) = files_with_content.get(&path_buf) {
                    let lsp_diag = convert_cairo_diagnostic(content, cairo_diag);
                    debug!(
                        "Converted semantic diagnostic: {:?} -> LSP range {:?}",
                        cairo_diag.message, lsp_diag.range
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

            // Update state and send responses
            for (uri, diagnostics) in diagnostics_by_file {
                diagnostics_state.set_diagnostics(&uri, diagnostics.clone());

                let _ = response_sender.send(DiagnosticsResponse {
                    uri,
                    version, // Use the passed version
                    diagnostics,
                });
            }
        }));

        // Handle panic in diagnostics computation
        if let Err(panic_payload) = result {
            error!("Panic in diagnostics computation: {:?}", panic_payload);
            error!(
                "This indicates a bug in the compiler or semantic analysis - the mutex should not be poisoned anymore"
            );
        }
    }
}

impl Drop for DiagnosticsController {
    fn drop(&mut self) {
        // Send shutdown signal
        let _ = self.sender.send(DiagnosticsRequest::Shutdown);

        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
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
