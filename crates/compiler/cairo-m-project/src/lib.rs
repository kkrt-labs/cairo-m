mod discovery;
mod manifest;
mod model;

pub use discovery::{discover_project, discover_workspace, find_entry_point};
pub use manifest::{CairoMToml, CrateManifest};
pub use model::{CrateId, Project, SourceLayout, Workspace};

/// The standard Cairo-M manifest filename
pub const MANIFEST_FILE_NAME: &str = "cairom.toml";
