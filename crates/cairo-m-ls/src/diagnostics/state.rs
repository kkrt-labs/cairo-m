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
        ProjectDiagnostics {
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

    /// Get diagnostics for a specific file
    pub fn get_diagnostics(&self, uri: &Url) -> Option<Vec<Diagnostic>> {
        let map = self.diagnostics.read().unwrap();
        map.get(uri).cloned()
    }

    /// Get all diagnostics grouped by file
    pub fn get_all_diagnostics(&self) -> HashMap<Url, Vec<Diagnostic>> {
        let map = self.diagnostics.read().unwrap();
        map.clone()
    }

    /// Clear diagnostics for a specific file
    pub fn clear_file(&self, uri: &Url) {
        debug!("Clearing diagnostics for {}", uri);

        let mut map = self.diagnostics.write().unwrap();
        map.remove(uri);
    }

    /// Clear all diagnostics
    pub fn clear(&self) {
        debug!("Clearing all diagnostics");

        let mut map = self.diagnostics.write().unwrap();
        map.clear();
    }

    /// Get the total number of diagnostics across all files
    pub fn total_count(&self) -> usize {
        let map = self.diagnostics.read().unwrap();
        map.values().map(|v| v.len()).sum()
    }

    /// Get the number of files with diagnostics
    pub fn file_count(&self) -> usize {
        let map = self.diagnostics.read().unwrap();
        map.len()
    }
}

impl Default for ProjectDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}
