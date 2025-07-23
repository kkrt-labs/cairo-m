#![allow(clippy::option_if_let_else)]

mod discovery;
mod manifest;
mod model;

pub use discovery::{discover_project, discover_workspace, find_project_manifest};
pub use manifest::ProjectManifest;
pub use model::{Project, ProjectId, SourceLayout, Workspace};

/// The standard Cairo-M manifest filename
pub const MANIFEST_FILE_NAME: &str = "cairom.toml";
