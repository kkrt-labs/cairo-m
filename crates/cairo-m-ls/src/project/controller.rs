use std::path::PathBuf;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use tracing::{error, info};

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
    Project(CrateInfo),
    /// No project manifest found, treat as standalone file
    Standalone(PathBuf),
}

pub struct ProjectController {
    sender: Sender<ProjectUpdateRequest>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ProjectController {
    pub fn new(response_sender: Sender<ProjectUpdate>) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            Self::worker_thread(receiver, response_sender);
        });

        ProjectController {
            sender,
            handle: Some(handle),
        }
    }

    pub fn update(
        &self,
        request: ProjectUpdateRequest,
    ) -> Result<(), crossbeam_channel::SendError<ProjectUpdateRequest>> {
        self.sender.send(request)
    }

    fn worker_thread(
        receiver: Receiver<ProjectUpdateRequest>,
        response_sender: Sender<ProjectUpdate>,
    ) {
        info!("ProjectController worker thread started");

        for request in receiver {
            match request {
                ProjectUpdateRequest::UpdateForFile { file_path } => {
                    info!(
                        "Processing project update request for: {}",
                        file_path.display()
                    );

                    match ProjectManifestPath::discover(&file_path) {
                        Some(manifest) => {
                            info!("Found project manifest: {:?}", manifest);

                            match Self::load_project(manifest) {
                                Ok(crate_info) => {
                                    if let Err(e) =
                                        response_sender.send(ProjectUpdate::Project(crate_info))
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
                            if let Err(e) =
                                response_sender.send(ProjectUpdate::Standalone(file_path))
                            {
                                error!("Failed to send standalone update: {}", e);
                            }
                        }
                    }
                }
            }
        }

        info!("ProjectController worker thread shutting down");
    }

    fn load_project(manifest: ProjectManifestPath) -> Result<CrateInfo, String> {
        match manifest {
            ProjectManifestPath::CairoM(manifest_path) => {
                let project_root = manifest_path
                    .parent()
                    .ok_or_else(|| "Invalid manifest path".to_string())?;

                // For now, we'll create a simple CrateInfo
                // This will be expanded to actually parse cairom.toml
                let name = project_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();

                Ok(CrateInfo {
                    name,
                    root: project_root.to_path_buf(),
                    manifest_path,
                })
            }
        }
    }
}

impl Drop for ProjectController {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            drop(self.sender.clone()); // Close the channel
            let _ = handle.join();
        }
    }
}
