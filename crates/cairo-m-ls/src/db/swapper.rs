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

        // Step 1: Extract essential state with minimal lock time
        let (open_files, all_crates) = {
            let old_db = match db.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    debug!("Failed to lock database for swap");
                    return;
                }
            };

            let mut open_files = Vec::new();
            let crates = project_model.all_crates();

            // Quickly extract file contents
            for crate_obj in &crates {
                for (path, source_file) in &crate_obj.files {
                    let content = source_file.text(&*old_db).to_string();
                    let file_path = source_file.file_path(&*old_db).to_string();
                    open_files.push((path.clone(), file_path, content));
                }
            }

            (open_files, crates)
        }; // Lock is released here

        // Step 2: Build the new database without holding any locks
        let mut new_db = AnalysisDatabase::new();

        // Re-create source files in the new database
        let mut file_map = std::collections::HashMap::new();
        for (_path, file_path, content) in open_files {
            let source_file =
                cairo_m_compiler_parser::SourceFile::new(&new_db, content, file_path.clone());
            file_map.insert(file_path, source_file);
        }

        // Re-apply all project crates
        // TODO: Update this to work with the new load_crate signature that requires file paths and get_source_file closure
        // for crate_obj in all_crates {
        //     if let Err(e) = project_model.load_crate(crate_obj.info, &mut new_db) {
        //         debug!("Failed to reload crate during swap: {}", e);
        //     }
        // }

        // Step 3: Perform the atomic swap with minimal lock time
        match db.lock() {
            Ok(mut old_db) => {
                *old_db = new_db;
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
