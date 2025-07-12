use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use cairo_m_compiler_semantic::db::project_validate_semantics;
use crossbeam_channel::{Receiver, Sender};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use tracing::{debug, info};

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
    sender: Sender<DiagnosticsRequest>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DiagnosticsController {
    /// Create a new diagnostics controller
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        diagnostics_state: Arc<ProjectDiagnostics>,
        project_model: Arc<ProjectModel>,
        response_sender: Sender<DiagnosticsResponse>,
    ) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            Self::worker_thread(
                db,
                diagnostics_state,
                project_model,
                receiver,
                response_sender,
            );
        });

        DiagnosticsController {
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
        response_sender: Sender<DiagnosticsResponse>,
    ) {
        info!("DiagnosticsController worker thread started");

        for request in receiver {
            match request {
                DiagnosticsRequest::FileChanged { uri, version } => {
                    debug!("Processing diagnostics for file: {}", uri);
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
                    );
                    debug!("Project diagnostics completed in {:?}", start.elapsed());
                }

                DiagnosticsRequest::Clear => {
                    info!("Clearing all diagnostics");
                    diagnostics_state.clear();
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
        response_sender: &Sender<DiagnosticsResponse>,
    ) {
        // Try to get the ProjectCrate for this file
        if let Some(project_crate) = project_model.get_project_crate_for_file(&uri) {
            // We have a project, run full project diagnostics
            Self::compute_project_diagnostics(
                db,
                diagnostics_state,
                project_crate,
                response_sender,
            );
        } else {
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
        response_sender: &Sender<DiagnosticsResponse>,
    ) {
        // Extract necessary data with minimal lock time
        let (_semantic_crate, files_with_content, diagnostic_collection) = {
            let db_guard = match db.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    debug!("Failed to lock database for diagnostics");
                    return;
                }
            };

            // Convert to semantic crate for validation
            let semantic_crate = project_crate.to_semantic_crate(&*db_guard);

            // Run semantic validation
            let diagnostic_collection = project_validate_semantics(&*db_guard, semantic_crate);

            // Clone file contents while we have the lock
            let files = project_crate.files(&*db_guard);
            let mut files_with_content = HashMap::new();
            for (path, source_file) in files {
                let content = source_file.text(&*db_guard).to_string();
                files_with_content.insert(path.clone(), content);
            }

            (semantic_crate, files_with_content, diagnostic_collection)
        }; // Lock is released here

        // Now process diagnostics without holding the lock
        let mut diagnostics_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
            std::collections::HashMap::new();

        // Initialize with empty diagnostics for all files
        for file_path in files_with_content.keys() {
            if let Ok(uri) = Url::from_file_path(file_path) {
                diagnostics_by_file.insert(uri, vec![]);
            }
        }

        // Convert and group Cairo diagnostics to LSP diagnostics
        for cairo_diag in diagnostic_collection.all() {
            if let Ok(uri) = Url::from_file_path(&cairo_diag.file_path) {
                // Find the source file content
                if let Some(content) = files_with_content.get(&PathBuf::from(&cairo_diag.file_path))
                {
                    let lsp_diag = convert_cairo_diagnostic(content, cairo_diag);

                    diagnostics_by_file.entry(uri).or_default().push(lsp_diag);
                }
            }
        }

        // Update state and send responses
        for (uri, diagnostics) in diagnostics_by_file {
            diagnostics_state.set_diagnostics(&uri, diagnostics.clone());

            let _ = response_sender.send(DiagnosticsResponse {
                uri,
                version: None, // Project-wide validation doesn't have a specific version
                diagnostics,
            });
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
fn convert_cairo_diagnostic(
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
