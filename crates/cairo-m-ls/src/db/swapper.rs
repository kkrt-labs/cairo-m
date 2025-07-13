use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tower_lsp::lsp_types::Url;
use tracing::{debug, error, info};

use crate::db::AnalysisDatabase;
use crate::project::ProjectModel;

/// Periodically swaps the analysis database to prevent unbounded memory growth.
///
/// This is a critical component for long-running language server sessions,
/// as Salsa accumulates query results over time that can lead to memory issues.
pub struct AnalysisDatabaseSwapper {
    /// Handle to the background thread
    handle: Option<thread::JoinHandle<()>>,
    /// Channel to signal shutdown
    shutdown_tx: crossbeam_channel::Sender<()>,
}

impl AnalysisDatabaseSwapper {
    /// Create a new database swapper that runs on the given interval
    pub fn new(
        db: Arc<Mutex<AnalysisDatabase>>,
        project_model: Arc<ProjectModel>,
        interval: Duration,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(1);

        let handle = thread::Builder::new()
            .name("database-swapper".to_string())
            .spawn(move || {
                // Catch panics to log them
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Self::worker_thread(db, project_model, interval, shutdown_rx);
                }));

                if let Err(e) = result {
                    error!("DatabaseSwapper worker thread panicked: {:?}", e);
                }
            })
            .expect("Failed to spawn DatabaseSwapper thread");

        Self {
            handle: Some(handle),
            shutdown_tx,
        }
    }

    /// Worker thread that performs periodic database swaps
    fn worker_thread(
        db: Arc<Mutex<AnalysisDatabase>>,
        project_model: Arc<ProjectModel>,
        interval: Duration,
        shutdown_rx: crossbeam_channel::Receiver<()>,
    ) {
        info!(
            "AnalysisDatabaseSwapper started with interval: {:?}",
            interval
        );

        loop {
            // Wait for interval or shutdown signal
            match shutdown_rx.recv_timeout(interval) {
                Ok(_) => {
                    info!("AnalysisDatabaseSwapper shutting down");
                    break;
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Time to swap
                    Self::perform_swap(&db, &project_model);
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    info!("AnalysisDatabaseSwapper shutdown channel disconnected");
                    break;
                }
            }
        }
    }

    /// Perform the actual database swap
    fn perform_swap(db: &Arc<Mutex<AnalysisDatabase>>, project_model: &Arc<ProjectModel>) {
        debug!("Starting database swap");
        let start = std::time::Instant::now();

        // Step 1: Snapshot live state with minimal lock time.
        // We get all project definitions and the current text of every file.
        let (all_crates, files_content_map) = {
            let old_db = match db.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    debug!("Failed to lock database for swap snapshot");
                    return;
                }
            };
            let crates = project_model.all_crates();
            let mut content_map = std::collections::HashMap::new();
            for krate in &crates {
                for (path, sf) in &krate.files {
                    if let Ok(uri) = Url::from_file_path(path) {
                        let content = sf.text(&*old_db).to_string();
                        content_map.insert(uri, content);
                    }
                }
            }
            (crates, content_map)
        }; // Lock on old_db is released here.

        // Step 2: Build the new database and project state offline.
        // This part is computationally expensive but doesn't block the language server.
        let mut new_db = AnalysisDatabase::new();
        let mut new_project_crate_ids = std::collections::HashMap::new();

        for krate in all_crates {
            let mut new_files_in_crate = std::collections::HashMap::new();
            for path in krate.files.keys() {
                if let Ok(uri) = Url::from_file_path(path) {
                    if let Some(content) = files_content_map.get(&uri) {
                        let new_sf = cairo_m_compiler_parser::SourceFile::new(
                            &mut new_db,
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
                &mut new_db,
                krate.info.root.clone(),
                main_module_name,
                new_files_in_crate,
            );
            new_project_crate_ids.insert(krate.info.root.clone(), new_project_crate);
        }

        // Step 3: Perform the atomic swap.
        // We acquire the DB lock and, while holding it, swap the database
        // and update the project model to point to the new entities.
        match db.lock() {
            Ok(mut old_db) => {
                *old_db = new_db;
                project_model.replace_project_crate_ids(new_project_crate_ids);
                let elapsed = start.elapsed();
                info!("Database swap completed in {:?}", elapsed);
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
            let _ = handle.join();
        }
    }
}

impl Drop for AnalysisDatabaseSwapper {
    fn drop(&mut self) {
        self.shutdown();
    }
}
