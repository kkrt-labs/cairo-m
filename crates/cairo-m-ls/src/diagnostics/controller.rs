use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cairo_m_compiler_semantic::delta_diagnostics::DeltaDiagnosticsTracker;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use tracing::{debug, error};

use crate::db::{AnalysisDatabase, ProjectCrate, ProjectCrateExt};
use crate::diagnostics::state::ProjectDiagnostics;
use crate::project::ProjectModel;
use crate::utils::{get_path_from_diagnostic, get_uri_from_path_str};

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
    ///
    /// Note: This implementation uses delta-based diagnostics tracking via DeltaDiagnosticsTracker
    /// rather than full recomputation. The delta system leverages Salsa's revision tracking to
    /// only recompute diagnostics for changed modules, providing significant performance improvements.
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: Arc<ProjectDiagnostics>,
        project_model: Arc<ProjectModel>,
        response_sender: UnboundedSender<DiagnosticsResponse>,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let _error_response_sender = response_sender.clone();
        let handle = tokio::spawn(async move {
            // Initialize delta diagnostics tracker for this task
            let mut delta_tracker = DeltaDiagnosticsTracker::new();

            // Process requests until channel is closed
            while let Some(request) = receiver.recv().await {
                debug!("Received request: {:?}", request);
                match request {
                    DiagnosticsRequest::FileChanged { uri, version } => {
                        Self::compute_file_diagnostics_delta(
                            &db,
                            &diagnostics_state,
                            &project_model,
                            uri,
                            version,
                            &response_sender,
                            &mut delta_tracker,
                        )
                        .await;
                        // Signal completion of work
                        let _ = response_sender.send(DiagnosticsResponse {
                            uri: Url::parse("internal://analysis-finished").unwrap(),
                            version: None,
                            diagnostics: vec![],
                        });
                    }

                    DiagnosticsRequest::ProjectChanged { project_crate } => {
                        debug!("Processing diagnostics for entire project using delta tracking");
                        let start = Instant::now();
                        Self::compute_project_diagnostics_delta(
                            &db,
                            &diagnostics_state,
                            project_crate,
                            &response_sender,
                            None,
                            &mut delta_tracker,
                        )
                        .await;
                        debug!("Project diagnostics completed in {:?}", start.elapsed());

                        // Signal completion of work
                        let _ = response_sender.send(DiagnosticsResponse {
                            uri: Url::parse("internal://analysis-finished").unwrap(),
                            version: None,
                            diagnostics: vec![],
                        });
                    }

                    DiagnosticsRequest::Shutdown => {
                        debug!("DiagnosticsController shutting down");
                        break;
                    }
                }
            }
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

    /// Compute diagnostics for a single file using delta tracking
    async fn compute_file_diagnostics_delta(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &Arc<ProjectDiagnostics>,
        project_model: &Arc<ProjectModel>,
        uri: Url,
        version: Option<i32>,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        delta_tracker: &mut DeltaDiagnosticsTracker,
    ) {
        // First try to get the ProjectCrate for this file (async call)
        if let Some(project_crate) = project_model.get_project_crate_for_file(&uri).await {
            // We have a project, run delta project diagnostics
            Self::compute_project_diagnostics_delta(
                db,
                diagnostics_state,
                project_crate,
                response_sender,
                version,
                delta_tracker,
            )
            .await;
        } else {
            debug!(
                "No project crate found for file: {}, clearing diagnostics",
                uri
            );
            // No project found, just clear diagnostics for this file
            diagnostics_state.set_diagnostics(&uri, vec![]).await;

            let _ = response_sender.send(DiagnosticsResponse {
                uri,
                version,
                diagnostics: vec![],
            });
        }
    }

    /// Compute diagnostics for an entire project using delta tracking (async wrapper)
    async fn compute_project_diagnostics_delta(
        db: &Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: &Arc<ProjectDiagnostics>,
        project_crate: ProjectCrate,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
        delta_tracker: &mut DeltaDiagnosticsTracker,
    ) {
        let diagnostics_state_clone = Arc::clone(diagnostics_state);
        let response_sender_clone = response_sender.clone();

        // We need to move the delta tracker computation to a blocking task
        // Since we can't move the mutable reference, we'll do the computation here
        let result = {
            let db_guard = match db.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    debug!("Database was poisoned, recovering from panic");
                    poisoned.into_inner()
                }
            };

            let semantic_crate = project_crate.to_semantic_crate(&*db_guard);

            // Use delta diagnostics tracker to get only changed module diagnostics
            let diagnostics_collection =
                delta_tracker.get_project_diagnostics(&*db_guard, semantic_crate);

            // Get file contents for LSP conversion
            let files = project_crate.files(&*db_guard);
            let mut files_with_content = HashMap::new();
            for (path, source_file) in files {
                let content = source_file.text(&*db_guard).to_string();
                files_with_content.insert(path.clone(), content);
            }
            drop(db_guard);

            (files_with_content, diagnostics_collection)
        };

        // Process the results with unconstrained wrapper to prevent cancellation
        let diagnostics_result = tokio::task::unconstrained(async {
            tokio::task::spawn_blocking(move || {
                Self::compute_project_diagnostics_delta_sync(result.0, result.1)
            })
            .await
            .unwrap_or_else(|e| {
                error!(
                    "Failed to spawn blocking task for delta diagnostics: {:?}",
                    e
                );
                Err("Failed to spawn blocking task".to_string())
            })
        })
        .await;

        // Handle the result and publish diagnostics
        if let Ok(diagnostics_by_file) = diagnostics_result {
            Self::publish_diagnostics(
                diagnostics_by_file,
                &diagnostics_state_clone,
                &response_sender_clone,
                version,
            )
            .await;
        }
    }

    /// Process delta diagnostics results (synchronous version)
    fn compute_project_diagnostics_delta_sync(
        files_with_content: HashMap<PathBuf, String>,
        diagnostics_collection: cairo_m_compiler_diagnostics::DiagnosticCollection,
    ) -> Result<HashMap<Url, Vec<Diagnostic>>, String> {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Convert delta diagnostics to LSP format
            Self::convert_delta_diagnostics_to_lsp(&files_with_content, &diagnostics_collection)
        }));

        match result {
            Ok(diagnostics_by_file) => Ok(diagnostics_by_file),
            Err(panic_payload) => {
                error!(
                    "Panic in delta diagnostics computation: {:?}",
                    panic_payload
                );
                Err("Panic in diagnostics computation".to_string())
            }
        }
    }

    /// Convert delta diagnostics to LSP format
    fn convert_delta_diagnostics_to_lsp(
        files_with_content: &HashMap<PathBuf, String>,
        diagnostics_collection: &cairo_m_compiler_diagnostics::DiagnosticCollection,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        let mut diagnostics_by_file: HashMap<Url, Vec<Diagnostic>> = HashMap::new();

        // Initialize with empty diagnostics for all files
        for file_path in files_with_content.keys() {
            if let Ok(uri) = get_uri_from_path_str(&file_path.to_string_lossy()) {
                diagnostics_by_file.insert(uri, vec![]);
            }
        }

        debug!(
            "Converting {} delta diagnostics to LSP format",
            diagnostics_collection.all().len()
        );

        // Process diagnostics
        Self::process_diagnostic_collection(
            diagnostics_collection,
            files_with_content,
            &mut diagnostics_by_file,
            "delta",
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
            let uri = match get_uri_from_path_str(&cairo_diag.file_path) {
                Ok(uri) => uri,
                Err(e) => {
                    debug!("Warning: {}", e);
                    continue;
                }
            };

            debug!("Processing {} diagnostic for URI: {}", diagnostic_type, uri);

            // Find the source file content
            let path_buf = get_path_from_diagnostic(&uri, &cairo_diag.file_path);
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

    /// Publish diagnostics to the client
    async fn publish_diagnostics(
        diagnostics_by_file: HashMap<Url, Vec<Diagnostic>>,
        diagnostics_state: &ProjectDiagnostics,
        response_sender: &UnboundedSender<DiagnosticsResponse>,
        version: Option<i32>,
    ) {
        for (uri, diagnostics) in diagnostics_by_file {
            diagnostics_state
                .set_diagnostics(&uri, diagnostics.clone())
                .await;

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
