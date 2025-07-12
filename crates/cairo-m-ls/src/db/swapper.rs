use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tracing::{debug, info};

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

        let handle = thread::spawn(move || {
            Self::worker_thread(db, project_model, interval, shutdown_rx);
        });

        AnalysisDatabaseSwapper {
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

        // Create new database
        let new_db = AnalysisDatabase::new();

        // Lock the old database to extract essential state
        let essential_state = match db.lock() {
            Ok(old_db) => {
                // Extract open files and their contents
                let mut open_files = Vec::new();

                // Get all project crates
                let crates = project_model.all_crates();

                // For each crate, get its files
                for crate_obj in crates {
                    for (path, source_file) in &crate_obj.files {
                        let content = source_file.text(&*old_db);
                        open_files.push((
                            path.clone(),
                            source_file.file_path(&*old_db).to_string(),
                            content.to_string(),
                        ));
                    }
                }

                Some(open_files)
            }
            Err(_) => {
                debug!("Failed to lock database for swap");
                return;
            }
        };

        if let Some(files) = essential_state {
            // Apply essential state to new database
            let mut new_db_mut = new_db;

            // Re-create source files
            for (_path, file_path, content) in files {
                let _ = cairo_m_compiler_parser::SourceFile::new(&new_db_mut, file_path, content);
            }

            // Re-apply all project crates
            for crate_obj in project_model.all_crates() {
                if let Err(e) = project_model.load_crate(crate_obj.info, &mut new_db_mut) {
                    debug!("Failed to reload crate during swap: {}", e);
                }
            }

            // Perform the atomic swap
            match db.lock() {
                Ok(mut old_db) => {
                    *old_db = new_db_mut;
                    let elapsed = start.elapsed();
                    info!("Database swap completed in {:?}", elapsed);
                }
                Err(_) => {
                    debug!("Failed to lock database for final swap");
                }
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
