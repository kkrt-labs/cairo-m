use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cairo_m_project::{Project, discover_project};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

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
        project: Project,
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
    /// The loaded project
    project: Project,
    /// Discovered files in the project
    files: Vec<PathBuf>,
    /// When this entry was last accessed
    last_accessed: Instant,
}

pub struct ProjectController {
    sender: UnboundedSender<ProjectUpdateRequest>,
    handle: Option<JoinHandle<()>>,
    _watcher: Option<RecommendedWatcher>,
}

impl ProjectController {
    pub fn new(response_sender: UnboundedSender<ProjectUpdate>) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        // Create shared manifest cache
        let manifest_cache: Arc<Mutex<HashMap<PathBuf, ManifestCacheEntry>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Set up file system watcher for project manifests
        let watcher_sender = sender.clone();
        let manifest_cache_for_watcher = Arc::clone(&manifest_cache);
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Check if this is a cairom.toml file modification
                    for path in event.paths {
                        if path.file_name().and_then(|n| n.to_str())
                            == Some(cairo_m_project::MANIFEST_FILE_NAME)
                        {
                            // Clear cache entry for this manifest to force reload
                            if let Ok(mut cache) = manifest_cache_for_watcher.lock() {
                                cache.remove(&path);
                            }

                            // Trigger project reload for any file in the project directory
                            if let Some(project_root) = path.parent() {
                                // Try to find any .cm file in the project to trigger reload
                                if let Ok(entries) = std::fs::read_dir(project_root) {
                                    for entry in entries.flatten() {
                                        let entry_path = entry.path();
                                        if entry_path.extension().and_then(|e| e.to_str())
                                            == Some("cm")
                                        {
                                            if let Err(e) = watcher_sender.send(
                                                ProjectUpdateRequest::UpdateForFile {
                                                    file_path: entry_path,
                                                },
                                            ) {
                                                debug!(
                                                    "Failed to send project update request from watcher: {}",
                                                    e
                                                );
                                            }
                                            break; // Only need to trigger once per project
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("File watcher error: {:?}", e);
                }
            }
        });

        // Try to create the watcher and watch the current directory
        let watcher = match watcher {
            Ok(mut w) => {
                // Watch the current working directory recursively for cairom.toml changes
                if let Ok(current_dir) = std::env::current_dir() {
                    if let Err(e) = w.watch(&current_dir, RecursiveMode::Recursive) {
                        warn!(
                            "Failed to watch current directory {:?}: {:?}",
                            current_dir, e
                        );
                    }
                }
                Some(w)
            }
            Err(e) => {
                warn!("Failed to create file watcher: {:?}", e);
                None
            }
        };

        let handle = tokio::spawn(async move {
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
                    drop(cache);
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
                    if let Err(e) =
                        response_sender.send(ProjectUpdate::ThreadError(error_msg.clone()))
                    {
                        error!(
                            "Failed to send thread error message: {} (original error: {})",
                            e, error_msg
                        );
                    }
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
        });

        Self {
            sender,
            handle: Some(handle),
            _watcher: watcher,
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
                match ProjectManifestPath::discover(&file_path) {
                    Some(manifest) => {
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
                                {
                                    let mut cache = manifest_cache.lock().unwrap();
                                    if let Some(cached_entry) = cache.get_mut(&manifest_path) {
                                        cached_entry.last_accessed = Instant::now();
                                    }
                                } // cache dropped here
                                // Use cached file list and convert project to crate info
                                let crate_info = CrateInfo {
                                    name: entry.project.name,
                                    root: entry.project.root_directory,
                                };
                                Ok((crate_info, entry.files))
                            }
                            None => {
                                // Load project and update cache
                                match Self::load_project(manifest) {
                                    Ok((project, files)) => {
                                        debug!(
                                            "Successfully loaded project: {} with {} files",
                                            project.name,
                                            files.len()
                                        );
                                        {
                                            let mut cache = manifest_cache.lock().unwrap();
                                            cache.insert(
                                                manifest_path.clone(),
                                                ManifestCacheEntry {
                                                    project: project.clone(),
                                                    files: files.clone(),
                                                    last_accessed: Instant::now(),
                                                },
                                            );
                                        } // cache dropped here
                                        debug!("Cached manifest: {:?}", manifest_path);
                                        let crate_info = CrateInfo {
                                            name: project.name,
                                            root: project.root_directory,
                                        };
                                        Ok((crate_info, files))
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                        };

                        match result {
                            Ok((crate_info, files)) => {
                                // Get the project from the cache
                                let project = {
                                    let cache = manifest_cache.lock().unwrap();
                                    cache
                                        .get(&manifest_path)
                                        .map(|entry| entry.project.clone())
                                        .expect("Project should be in cache after loading")
                                };

                                if let Err(e) = response_sender.send(ProjectUpdate::Project {
                                    project,
                                    crate_info,
                                    files,
                                }) {
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
                        debug!("No project manifest found, treating as standalone file");
                        if let Err(e) = response_sender.send(ProjectUpdate::Standalone(file_path)) {
                            error!("Failed to send standalone update: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn load_project(manifest: ProjectManifestPath) -> Result<(Project, Vec<PathBuf>), String> {
        match manifest {
            ProjectManifestPath::CairoM(manifest_path) => {
                // Use cairo-m-project's discovery to load project from manifest
                let project_root = manifest_path
                    .parent()
                    .ok_or_else(|| "Invalid manifest path".to_string())?;

                let project = discover_project(project_root)
                    .map_err(|e| format!("Failed to discover project: {}", e))?
                    .ok_or_else(|| "Project discovery returned None".to_string())?;

                // Get project files using cairo-m-project
                let files = project
                    .source_files()
                    .map_err(|e| format!("Failed to get source files: {}", e))?;

                Ok((project, files))
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
