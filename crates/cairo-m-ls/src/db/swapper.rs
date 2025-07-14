use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tower_lsp::lsp_types::Url;
use tracing::debug;

use crate::db::AnalysisDatabase;
use crate::project::ProjectModel;
use crate::project::model::Crate;

/// Periodically swaps the analysis database to prevent unbounded memory growth.
///
/// This is a critical component for long-running language server sessions,
/// as Salsa accumulates query results over time that can lead to memory issues.
pub struct AnalysisDatabaseSwapper {
    /// Handle to the background task
    handle: Option<JoinHandle<()>>,
    /// Channel to signal shutdown
    shutdown_tx: mpsc::UnboundedSender<()>,
}

impl AnalysisDatabaseSwapper {
    /// Create a new database swapper that runs on the given interval
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        project_model: Arc<ProjectModel>,
        interval_duration: Duration,
    ) -> Self {
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();

        let handle = tokio::spawn(async move {
            let mut timer = interval(interval_duration);
            timer.tick().await; // Skip the first immediate tick

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        debug!("AnalysisDatabaseSwapper shutting down");
                        break;
                    }
                    _ = timer.tick() => {
                        Self::perform_swap(&db, &project_model).await;
                    }
                }
            }
        });

        Self {
            handle: Some(handle),
            shutdown_tx,
        }
    }

    /// Perform the actual database swap
    async fn perform_swap(db: &Arc<Mutex<AnalysisDatabase>>, project_model: &Arc<ProjectModel>) {
        debug!("Starting database swap");
        let start = std::time::Instant::now();

        // Step 1: Get all crates first (async call outside lock)
        let all_crates = project_model.all_crates().await;

        // Step 2: Snapshot live state with minimal lock time.
        // We get the current text of every file.
        let files_content_map = {
            let mut content_map = std::collections::HashMap::new();
            {
                let old_db = match db.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        debug!("Failed to lock database for swap snapshot");
                        return;
                    }
                };
                for krate in &all_crates {
                    for (path, sf) in &krate.files {
                        if let Ok(uri) = Url::from_file_path(path) {
                            let content = sf.text(&*old_db).to_string();
                            content_map.insert(uri, content);
                        }
                    }
                }
                drop(old_db);
            } // Lock on old_db is released here.
            content_map
        };

        // Step 3: Build the new database and project state offline.
        // This part is computationally expensive but doesn't block the language server.
        let new_db = AnalysisDatabase::new();
        let mut new_project_crate_ids = std::collections::HashMap::new();
        let mut new_crates = std::collections::HashMap::new();

        for krate in all_crates {
            let mut new_files_in_crate = std::collections::HashMap::new();
            for path in krate.files.keys() {
                if let Ok(uri) = Url::from_file_path(path) {
                    if let Some(content) = files_content_map.get(&uri) {
                        let new_sf = cairo_m_compiler_parser::SourceFile::new(
                            &new_db,
                            content.clone(),
                            uri.to_string(),
                        );
                        new_files_in_crate.insert(path.clone(), new_sf);
                    }
                }
            }

            let main_module_name = krate
                .main_file
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("main")
                .to_string();

            let new_project_crate = crate::db::ProjectCrate::new(
                &new_db,
                krate.info.root.clone(),
                main_module_name,
                new_files_in_crate.clone(),
            );
            new_project_crate_ids.insert(krate.info.root.clone(), new_project_crate);

            // Create new Crate object with fresh SourceFile IDs
            let new_crate = Crate {
                info: krate.info.clone(),
                main_file: krate.main_file.clone(),
                files: new_files_in_crate,
            };
            new_crates.insert(krate.info.root.clone(), new_crate);
        }

        // Step 4: Update the project model first (async call)
        // CRITICAL: Must update both project_crate_ids AND crates to avoid stale Salsa IDs
        project_model
            .replace_project_crate_ids(new_project_crate_ids)
            .await;
        project_model.replace_crates(new_crates).await;

        // Step 5: Perform the atomic database swap.
        // We acquire the DB lock and swap the database.
        match db.lock() {
            Ok(mut old_db) => {
                *old_db = new_db;
                let elapsed = start.elapsed();
                debug!("Database swap completed in {:?}", elapsed);
            }
            Err(_) => {
                debug!("Failed to lock database for final swap");
            }
        }
    }

    /// Request shutdown of the swapper
    pub fn shutdown(&mut self) {
        let _ = self.shutdown_tx.send(());

        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl Drop for AnalysisDatabaseSwapper {
    fn drop(&mut self) {
        self.shutdown();
    }
}
