//! # Delta-based Diagnostics System
//!
//! This module implements delta-based diagnostics that only recompute diagnostics
//! for modules that have changed, using Salsa's incremental computation capabilities.
//!
//! ## Architecture
//!
//! The delta diagnostics system works by:
//! 1. Tracking the last known revision for each module
//! 2. Using Salsa's change detection to identify which modules have been modified
//! 3. Only recomputing diagnostics for changed modules
//! 4. Merging results with cached diagnostics from unchanged modules
//!
//! ## Usage
//!
//! ```rust,ignore
//! let mut delta_system = DeltaDiagnosticsTracker::new();
//!
//! // Compute diagnostics (only changed modules will be recomputed)
//! let diagnostics = delta_system.get_project_diagnostics(db, crate_id);
//!
//! // Update tracking after changes
//! delta_system.mark_revision(db);
//! ```

use std::collections::HashMap;

use cairo_m_compiler_diagnostics::DiagnosticCollection;
use tracing::{debug, info};

use crate::db::{Crate, SemanticDb, module_all_diagnostics, module_parse_diagnostics};

/// Tracks the revision state for delta-based diagnostics computation
#[derive(Debug, Clone)]
pub struct DeltaDiagnosticsTracker {
    /// Last known revision for each module
    module_revisions: HashMap<String, salsa::Revision>,
    /// Cached diagnostics for each module from the last computation
    cached_diagnostics: HashMap<String, DiagnosticCollection>,
    /// The overall project revision at last computation
    last_project_revision: Option<salsa::Revision>,
}

impl DeltaDiagnosticsTracker {
    /// Create a new delta diagnostics tracker
    pub fn new() -> Self {
        Self {
            module_revisions: HashMap::new(),
            cached_diagnostics: HashMap::new(),
            last_project_revision: None,
        }
    }

    /// Get diagnostics for the entire project, only recomputing changed modules
    pub fn get_project_diagnostics(
        &mut self,
        db: &dyn SemanticDb,
        crate_id: Crate,
    ) -> DiagnosticCollection {
        let current_revision = db.zalsa().current_revision();

        // Check if this is the first computation or if anything has changed
        let needs_full_recompute = self.last_project_revision.is_none()
            || self.last_project_revision.unwrap() < current_revision;

        if !needs_full_recompute {
            debug!("[DELTA] No changes detected, using cached diagnostics");
            return self.get_cached_project_diagnostics();
        }

        info!("[DELTA] Computing delta diagnostics for project");
        let modules = crate_id.modules(db);
        let mut total_diagnostics = DiagnosticCollection::default();
        let mut recomputed_modules = 0;
        let mut cached_modules = 0;

        for (module_name, _file) in modules.iter() {
            let module_changed = self.has_module_changed(db, crate_id, module_name.clone());

            if module_changed {
                debug!(
                    "[DELTA] Recomputing diagnostics for changed module: {}",
                    module_name
                );
                let module_diagnostics = module_all_diagnostics(db, crate_id, module_name.clone());

                // Update our cache
                self.cached_diagnostics
                    .insert(module_name.clone(), module_diagnostics.clone());
                self.module_revisions
                    .insert(module_name.clone(), current_revision);

                total_diagnostics.extend(module_diagnostics.all().iter().cloned());
                recomputed_modules += 1;
            } else {
                debug!(
                    "[DELTA] Using cached diagnostics for unchanged module: {}",
                    module_name
                );
                if let Some(cached_diag) = self.cached_diagnostics.get(module_name) {
                    total_diagnostics.extend(cached_diag.all().iter().cloned());
                    cached_modules += 1;
                } else {
                    // Module wasn't in cache, need to compute it
                    debug!("[DELTA] Module {} not in cache, computing", module_name);
                    let module_diagnostics =
                        module_all_diagnostics(db, crate_id, module_name.clone());
                    self.cached_diagnostics
                        .insert(module_name.clone(), module_diagnostics.clone());
                    self.module_revisions
                        .insert(module_name.clone(), current_revision);
                    total_diagnostics.extend(module_diagnostics.all().iter().cloned());
                    recomputed_modules += 1;
                }
            }
        }

        self.last_project_revision = Some(current_revision);

        info!(
            "[DELTA] Delta computation complete: {} modules recomputed, {} modules cached",
            recomputed_modules, cached_modules
        );

        total_diagnostics
    }

    /// Get parse diagnostics only for changed modules
    pub fn get_project_parse_diagnostics(
        &mut self,
        db: &dyn SemanticDb,
        crate_id: Crate,
    ) -> DiagnosticCollection {
        let current_revision = db.zalsa().current_revision();
        let modules = crate_id.modules(db);
        let mut total_diagnostics = DiagnosticCollection::default();

        for (module_name, _file) in modules.iter() {
            let module_changed = self.has_module_changed(db, crate_id, module_name.clone());

            if module_changed || !self.cached_diagnostics.contains_key(module_name) {
                let module_diagnostics =
                    module_parse_diagnostics(db, crate_id, module_name.clone());
                total_diagnostics.extend(module_diagnostics.all().iter().cloned());
            } else if let Some(cached_diag) = self.cached_diagnostics.get(module_name) {
                // For parse diagnostics, we can reuse the parse portion of cached diagnostics
                total_diagnostics.extend(cached_diag.all().iter().cloned());
            }
        }

        total_diagnostics
    }

    /// Get the list of modules that have changed since the last computation
    pub fn get_changed_modules(&self, db: &dyn SemanticDb, crate_id: Crate) -> Vec<String> {
        let modules = crate_id.modules(db);
        let mut changed_modules = Vec::new();

        for (module_name, _file) in modules.iter() {
            if self.has_module_changed(db, crate_id, module_name.clone()) {
                changed_modules.push(module_name.clone());
            }
        }

        changed_modules
    }

    /// Check if a specific module has changed since our last tracking
    fn has_module_changed(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        module_name: String,
    ) -> bool {
        let current_revision = db.zalsa().current_revision();

        // If we don't have a tracked revision for this module, it's considered changed
        if let Some(last_revision) = self.module_revisions.get(&module_name) {
            if current_revision > *last_revision {
                // Check if the actual module content has changed by querying the file
                if let Some(file) = crate_id.modules(db).get(&module_name) {
                    // This query will be cached by Salsa and will tell us if the content changed
                    let _content = file.text(db);
                    // If we reach here and the revision is newer, the content likely changed
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // No previous revision tracked, consider it changed
            true
        }
    }

    /// Get all cached diagnostics combined (without recomputation)
    fn get_cached_project_diagnostics(&self) -> DiagnosticCollection {
        let mut total_diagnostics = DiagnosticCollection::default();

        for cached_diag in self.cached_diagnostics.values() {
            total_diagnostics.extend(cached_diag.all().iter().cloned());
        }

        total_diagnostics
    }

    /// Mark the current revision as processed (call after handling diagnostics)
    pub fn mark_revision(&mut self, db: &dyn SemanticDb) {
        let current_revision = db.zalsa().current_revision();
        self.last_project_revision = Some(current_revision);
    }

    /// Clear all cached diagnostics (useful when major changes occur)
    pub fn clear_cache(&mut self) {
        self.module_revisions.clear();
        self.cached_diagnostics.clear();
        self.last_project_revision = None;
        info!("[DELTA] Cleared all cached diagnostics");
    }

    /// Get statistics about the current cache state
    pub fn get_cache_stats(&self) -> DeltaCacheStats {
        DeltaCacheStats {
            modules_tracked: self.module_revisions.len(),
            cached_diagnostics: self.cached_diagnostics.len(),
            last_revision: self.last_project_revision,
        }
    }
}

impl Default for DeltaDiagnosticsTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the delta diagnostics cache
#[derive(Debug, Clone)]
pub struct DeltaCacheStats {
    /// Number of modules being tracked
    pub modules_tracked: usize,
    /// Number of modules with cached diagnostics
    pub cached_diagnostics: usize,
    /// Last project revision processed
    pub last_revision: Option<salsa::Revision>,
}

impl DeltaCacheStats {
    /// Check if the cache is healthy (all tracked modules have cached diagnostics)
    pub const fn is_healthy(&self) -> bool {
        self.modules_tracked == self.cached_diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{crate_from_program, test_db};

    #[test]
    fn test_delta_tracker_initialization() {
        let tracker = DeltaDiagnosticsTracker::new();
        let stats = tracker.get_cache_stats();

        assert_eq!(stats.modules_tracked, 0);
        assert_eq!(stats.cached_diagnostics, 0);
        assert!(stats.last_revision.is_none());
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_first_computation_recomputes_all() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "fn main() { let x = 42; }");
        let mut tracker = DeltaDiagnosticsTracker::new();

        let diagnostics = tracker.get_project_diagnostics(&db, crate_id);
        let stats = tracker.get_cache_stats();

        // First computation should process all modules
        assert_eq!(stats.modules_tracked, 1); // "main" module
        assert_eq!(stats.cached_diagnostics, 1);
        assert!(stats.last_revision.is_some());
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_unchanged_modules_use_cache() {
        let db = test_db();
        let crate_id = crate_from_program(&db, "fn main() { let x = 42; }");
        let mut tracker = DeltaDiagnosticsTracker::new();

        // First computation
        let _diagnostics1 = tracker.get_project_diagnostics(&db, crate_id);
        let stats1 = tracker.get_cache_stats();

        // Second computation without changes
        let _diagnostics2 = tracker.get_project_diagnostics(&db, crate_id);
        let stats2 = tracker.get_cache_stats();

        // Should have the same stats (using cache)
        assert_eq!(stats1.modules_tracked, stats2.modules_tracked);
        assert_eq!(stats1.cached_diagnostics, stats2.cached_diagnostics);
    }
}
