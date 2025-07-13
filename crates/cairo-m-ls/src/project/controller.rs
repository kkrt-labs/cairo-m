use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use super::manifest::ProjectManifestPath;
use super::model::CrateInfo;

#[derive(Debug)]
pub enum ProjectUpdateRequest {
    /// Request to update project for a given file path
    UpdateForFile { file_path: PathBuf },
}

#[derive(Debug)]
pub enum ProjectUpdate {
    /// A project with manifest was found
    Project {
        crate_info: CrateInfo,
        files: Vec<PathBuf>,
    },
    /// No project manifest found, treat as standalone file
    Standalone(PathBuf),
    /// Thread error occurred
    ThreadError(String),
}

/// Cache entry for a loaded manifest
#[derive(Debug, Clone)]
struct ManifestCacheEntry {
    /// The loaded crate info
    crate_info: CrateInfo,
    /// Discovered files in the project
    files: Vec<PathBuf>,
    /// When this entry was last accessed
    last_accessed: Instant,
}

pub struct ProjectController {
    sender: UnboundedSender<ProjectUpdateRequest>,
    handle: Option<JoinHandle<()>>,
}

impl ProjectController {
    pub fn new(response_sender: UnboundedSender<ProjectUpdate>) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        // Create shared manifest cache
        let manifest_cache: Arc<Mutex<HashMap<PathBuf, ManifestCacheEntry>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let handle = tokio::spawn(async move {
            info!("ProjectController worker task started");

            // Cache expiration time (5 minutes)
            const CACHE_EXPIRY: Duration = Duration::from_secs(300);

            // Counter for periodic cleanup
            let mut request_count = 0u32;

            while let Some(request) = receiver.recv().await {
                request_count += 1;

                // Periodic cache cleanup every 10 requests
                if request_count % 10 == 0 {
                    let mut cache = manifest_cache.lock().unwrap();
                    let before_size = cache.len();
                    cache.retain(|_, entry| entry.last_accessed.elapsed() < CACHE_EXPIRY);
                    let after_size = cache.len();
                    if before_size > after_size {
                        debug!(
                            "Periodic cache cleanup: removed {} stale entries",
                            before_size - after_size
                        );
                    }
                }

                // Process the request on the blocking thread pool since it involves file I/O
                let response_sender_clone = response_sender.clone();
                let manifest_cache_clone = Arc::clone(&manifest_cache);
                tokio::task::spawn_blocking(move || {
                    Self::process_request(request, response_sender_clone, manifest_cache_clone);
                })
                .await
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to spawn blocking task for project controller: {:?}",
                        e
                    );
                    let error_msg = format!("ProjectController task failed: {:?}", e);
                    let _ = response_sender.send(ProjectUpdate::ThreadError(error_msg));
                });
            }

            // Clean up expired cache entries on shutdown
            if let Ok(mut cache) = manifest_cache.lock() {
                cache.retain(|_, entry| entry.last_accessed.elapsed() < CACHE_EXPIRY);
                debug!(
                    "Cleaned up manifest cache, {} entries remaining",
                    cache.len()
                );
            }

            info!("ProjectController worker task shutting down");
        });

        Self {
            sender,
            handle: Some(handle),
        }
    }

    pub fn update(
        &self,
        request: ProjectUpdateRequest,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<ProjectUpdateRequest>> {
        self.sender.send(request)
    }

    fn process_request(
        request: ProjectUpdateRequest,
        response_sender: UnboundedSender<ProjectUpdate>,
        manifest_cache: Arc<Mutex<HashMap<PathBuf, ManifestCacheEntry>>>,
    ) {
        match request {
            ProjectUpdateRequest::UpdateForFile { file_path } => {
                info!(
                    "Processing project update request for: {}",
                    file_path.display()
                );

                match ProjectManifestPath::discover(&file_path) {
                    Some(manifest) => {
                        info!("Found project manifest: {:?}", manifest);

                        // Check cache first
                        let manifest_path = manifest.path().to_path_buf();
                        const CACHE_EXPIRY: Duration = Duration::from_secs(300);
                        let cache_hit = {
                            let cache = manifest_cache.lock().unwrap();
                            cache.get(&manifest_path).and_then(|entry| {
                                if entry.last_accessed.elapsed() < CACHE_EXPIRY {
                                    debug!("Cache hit for manifest: {:?}", manifest_path);
                                    Some(entry.clone())
                                } else {
                                    debug!("Cache expired for manifest: {:?}", manifest_path);
                                    None
                                }
                            })
                        };

                        let result = match cache_hit {
                            Some(entry) => {
                                // Update last accessed time
                                let mut cache = manifest_cache.lock().unwrap();
                                if let Some(cached_entry) = cache.get_mut(&manifest_path) {
                                    cached_entry.last_accessed = Instant::now();
                                }
                                // Use cached file list
                                Ok((entry.crate_info, entry.files))
                            }
                            None => {
                                // Load project and update cache
                                match Self::load_project(manifest) {
                                    Ok((crate_info, files)) => {
                                        let mut cache = manifest_cache.lock().unwrap();
                                        cache.insert(
                                            manifest_path.clone(),
                                            ManifestCacheEntry {
                                                crate_info: crate_info.clone(),
                                                files: files.clone(),
                                                last_accessed: Instant::now(),
                                            },
                                        );
                                        debug!("Cached manifest: {:?}", manifest_path);
                                        Ok((crate_info, files))
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                        };

                        match result {
                            Ok((crate_info, files)) => {
                                if let Err(e) = response_sender
                                    .send(ProjectUpdate::Project { crate_info, files })
                                {
                                    error!("Failed to send project update: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to load project: {}", e);
                                // Treat as standalone on error
                                if let Err(e) =
                                    response_sender.send(ProjectUpdate::Standalone(file_path))
                                {
                                    error!("Failed to send standalone update: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        info!("No project manifest found, treating as standalone file");
                        if let Err(e) = response_sender.send(ProjectUpdate::Standalone(file_path)) {
                            error!("Failed to send standalone update: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn load_project(manifest: ProjectManifestPath) -> Result<(CrateInfo, Vec<PathBuf>), String> {
        match manifest {
            ProjectManifestPath::CairoM(manifest_path) => {
                let project_root = manifest_path
                    .parent()
                    .ok_or_else(|| "Invalid manifest path".to_string())?;

                let name = project_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();

                let crate_info = CrateInfo {
                    name,
                    root: project_root.to_path_buf(),
                };

                // Discover project files
                let config = cairo_m_compiler::project_discovery::ProjectDiscoveryConfig::default();
                let discovered = cairo_m_compiler::project_discovery::discover_project_files(
                    project_root,
                    &config,
                )
                .map_err(|e| format!("Failed to discover project files: {}", e))?;

                Ok((crate_info, discovered.files))
            }
        }
    }
}

impl Drop for ProjectController {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // Dropping the sender closes the channel, which will cause the task to exit
            handle.abort();
        }
    }
}
