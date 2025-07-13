use std::collections::HashMap;
use std::sync::RwLock;

use tower_lsp::lsp_types::{Diagnostic, Url};
use tracing::debug;

/// Thread-safe storage for project-wide diagnostics
pub struct ProjectDiagnostics {
    /// Map from file URL to diagnostics
    diagnostics: RwLock<HashMap<Url, Vec<Diagnostic>>>,
}

impl ProjectDiagnostics {
    /// Create a new ProjectDiagnostics instance
    pub fn new() -> Self {
        Self {
            diagnostics: RwLock::new(HashMap::new()),
        }
    }

    /// Set diagnostics for a specific file
    pub fn set_diagnostics(&self, uri: &Url, diagnostics: Vec<Diagnostic>) {
        debug!("Setting {} diagnostics for {}", diagnostics.len(), uri);

        let mut map = self.diagnostics.write().unwrap();
        if diagnostics.is_empty() {
            map.remove(uri);
        } else {
            map.insert(uri.clone(), diagnostics);
        }
    }

    /// Clear all diagnostics
    #[allow(dead_code)]
    pub fn clear(&self) {
        debug!("Clearing all diagnostics");

        let mut map = self.diagnostics.write().unwrap();
        map.clear();
    }

    /// Clear diagnostics for all files in a project
    pub fn clear_for_project(&self, project_files: &[Url]) {
        debug!(
            "Clearing diagnostics for {} project files",
            project_files.len()
        );

        let mut map = self.diagnostics.write().unwrap();
        for uri in project_files {
            map.remove(uri);
        }
    }
}

impl Default for ProjectDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}
