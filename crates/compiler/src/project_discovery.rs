use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cairo_m_compiler_parser::{DiscoveredCrate, SourceFile};
use walkdir::WalkDir;

/// Configuration for project discovery
#[derive(Debug, Clone)]
pub struct ProjectDiscoveryConfig {
    /// File extensions to consider as source files
    pub extensions: Vec<String>,
    /// Whether to use cached discovery results
    pub use_cache: bool,
    /// Maximum directory depth to search
    pub max_depth: Option<usize>,
}

impl Default for ProjectDiscoveryConfig {
    fn default() -> Self {
        Self {
            extensions: vec!["cm".to_string()],
            use_cache: true,
            max_depth: None,
        }
    }
}

/// Result of project discovery
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub entry_point: PathBuf,
}

/// Find the project root starting from a given path
pub fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    loop {
        // Check for project markers
        if current.join("cairom.toml").exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // Fallback to the directory containing the start path
    if start_path.is_file() {
        start_path.parent().map(|p| p.to_path_buf())
    } else {
        Some(start_path.to_path_buf())
    }
}

/// Discover all source files in a project
pub fn discover_project_files(
    root: &Path,
    config: &ProjectDiscoveryConfig,
) -> Result<DiscoveredProject, String> {
    let mut files = Vec::new();
    let mut main_file = None;

    let walker = if let Some(max_depth) = config.max_depth {
        WalkDir::new(root).max_depth(max_depth)
    } else {
        WalkDir::new(root)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Check if this is a source file
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if config.extensions.contains(&ext.to_string()) {
                files.push(path.to_path_buf());

                let filename = path.file_name().and_then(|n| n.to_str());
                if let Some(filename) = filename {
                    if filename == "main.cm" || filename == "lib.cm" {
                        if main_file.is_some() {
                            return Err("Multiple main or lib files found".to_string());
                        }
                        main_file = Some(path.to_path_buf());
                    }
                }
            }
        }
    }

    if files.is_empty() {
        return Err("No source files found".to_string());
    }

    // Sort for deterministic ordering
    files.sort();

    // Determine entry point
    let entry_point = main_file.unwrap_or_else(|| files[0].clone());

    Ok(DiscoveredProject {
        root: root.to_path_buf(),
        files,
        entry_point,
    })
}

/// Create a DiscoveredCrate from discovered project files
pub fn create_crate_from_discovery(
    db: &dyn cairo_m_compiler_parser::Db,
    discovered: &DiscoveredProject,
) -> Result<DiscoveredCrate, std::io::Error> {
    let mut source_files = Vec::new();

    for file_path in &discovered.files {
        let content = std::fs::read_to_string(file_path)?;
        let source_file = SourceFile::new(db, content, file_path.display().to_string());
        source_files.push(source_file);
    }

    // Extract entry file name from the full path
    let entry_file = discovered
        .entry_point
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main.cm")
        .to_string();

    Ok(DiscoveredCrate::new(
        db,
        discovered.root.display().to_string(),
        entry_file,
        source_files,
    ))
}

/// Create a semantic Project from a DiscoveredCrate
pub fn create_project_from_crate(
    db: &dyn cairo_m_compiler_semantic::SemanticDb,
    crate_data: DiscoveredCrate,
) -> cairo_m_compiler_semantic::Crate {
    let mut modules = HashMap::new();

    for source_file in crate_data.files(db) {
        let file_path = source_file.file_path(db);
        let module_name = Path::new(&file_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string();
        modules.insert(module_name, source_file);
    }

    let entry_path = crate_data.entry_file(db);
    let entry_module_name = Path::new(&entry_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("main")
        .to_string();

    cairo_m_compiler_semantic::Crate::new(db, modules, entry_module_name)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_find_project_root() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("project");
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(project_dir.join("cairom.toml"), "").unwrap();

        let root = find_project_root(src_dir.join("main.cm").as_path()).unwrap();
        // Compare canonicalized paths to handle symlink differences on macOS
        assert_eq!(
            root.canonicalize().unwrap(),
            project_dir.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_discover_project_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(src_dir.join("main.cm"), "func main() {}").unwrap();
        fs::write(src_dir.join("math.cm"), "func add() {}").unwrap();
        fs::write(src_dir.join("README.md"), "# Test").unwrap(); // Should be ignored

        let config = ProjectDiscoveryConfig::default();
        let discovered = discover_project_files(temp_dir.path(), &config).unwrap();

        assert_eq!(discovered.files.len(), 2);
        assert!(discovered.entry_point.ends_with("main.cm"));
    }

    #[test]
    fn test_project_discovery_with_invalid_root() {
        let config = ProjectDiscoveryConfig::default();
        let result = discover_project_files(Path::new("/definitely/does/not/exist"), &config);

        // Should return Ok with empty files since the directory doesn't exist
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No source files found"));
    }

    #[test]
    fn test_find_project_root_with_cairom_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("project");
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(project_dir.join("cairom.toml"), "").unwrap();
        fs::write(src_dir.join("main.cm"), "").unwrap();

        // Test with file path
        let root = find_project_root(&src_dir.join("main.cm")).unwrap();
        assert!(root.ends_with("project"));

        // Test with directory path
        let root2 = find_project_root(&src_dir).unwrap();
        assert!(root2.ends_with("project"));
    }
}
