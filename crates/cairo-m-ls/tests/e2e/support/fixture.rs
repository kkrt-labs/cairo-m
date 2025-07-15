use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A test fixture that manages temporary file system for tests
#[derive(Debug)]
pub struct Fixture {
    /// The temporary directory that will be cleaned up when dropped
    temp_dir: TempDir,
}

impl Fixture {
    /// Create a new fixture with a temporary directory
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        Self { temp_dir }
    }

    /// Add a file to the fixture with the given path and content
    pub fn add_file(&self, path: impl AsRef<Path>, content: impl AsRef<str>) {
        let mut path = path.as_ref().to_path_buf();

        // Default to .cm extension for Cairo-M files if no extension specified
        if path.extension().is_none() && !path.to_string_lossy().contains("cairom.toml") {
            path.set_extension("cm");
        }

        let full_path = self.temp_dir.path().join(&path);

        // Create parent directories if necessary
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        std::fs::write(&full_path, content.as_ref()).expect("Failed to write file");
    }

    /// Add a cairom.toml file with default project configuration
    pub fn add_cairom_toml(&self, project_name: &str) {
        self.add_cairom_toml_with_entry_point(project_name, None);
    }

    /// Add a cairom.toml file with project configuration and optional entry point
    pub fn add_cairom_toml_with_entry_point(&self, project_name: &str, entry_point: Option<&str>) {
        let mut content = format!(
            r#"
name = "{}"
version = "0.1.0"
"#,
            project_name
        );

        if let Some(entry_point) = entry_point {
            content.push_str(&format!("entry_point = \"{}\"\n", entry_point));
        }

        self.add_file("cairom.toml", content);

        // Create src directory by default as cairo-m-project expects it
        std::fs::create_dir_all(self.temp_dir.path().join("src"))
            .expect("Failed to create src directory");
    }

    /// Get the root path of the temporary directory
    pub fn root_path(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Get the root URL of the temporary directory
    pub fn root_url(&self) -> lsp_types::Url {
        lsp_types::Url::from_file_path(self.root_path())
            .expect("Failed to create URL from root path")
    }

    /// Get the URL for a specific file in the fixture
    pub fn file_url(&self, path: impl AsRef<Path>) -> lsp_types::Url {
        let full_path = self.temp_dir.path().join(path.as_ref());
        lsp_types::Url::from_file_path(full_path).expect("Failed to create URL from file path")
    }

    /// Read the content of a file in the fixture
    pub fn read_file(&self, path: impl AsRef<Path>) -> String {
        let full_path = self.temp_dir.path().join(path.as_ref());
        std::fs::read_to_string(full_path).expect("Failed to read file")
    }

    /// Check if a file exists in the fixture
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        let full_path = self.temp_dir.path().join(path.as_ref());
        full_path.exists()
    }

    /// List all files in the fixture (recursively)
    pub fn list_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_files_recursive(self.temp_dir.path(), &mut files);
        files
    }

    fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Get relative path from temp_dir root
                    if let Ok(relative) = path.strip_prefix(self.temp_dir.path()) {
                        files.push(relative.to_path_buf());
                    }
                } else if path.is_dir() {
                    self.collect_files_recursive(&path, files);
                }
            }
        }
    }
}

impl Clone for Fixture {
    fn clone(&self) -> Self {
        let new_fixture = Self::new();

        // Copy all files from the original fixture to the new one
        let files = self.list_files();
        for file_path in files {
            let content = self.read_file(&file_path);
            new_fixture.add_file(&file_path, content);
        }

        new_fixture
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_basic_operations() {
        let fixture = Fixture::new();

        // Test adding a file with explicit extension
        fixture.add_file("test.cm", "func main() {}");
        assert!(fixture.file_exists("test.cm"));
        assert_eq!(fixture.read_file("test.cm"), "func main() {}");

        // Test adding a file without extension (should default to .cm)
        fixture.add_file("test2", "func test() {}");
        assert!(fixture.file_exists("test2.cm"));
        assert_eq!(fixture.read_file("test2.cm"), "func test() {}");

        // Test adding a cairom.toml file
        fixture.add_cairom_toml("test_project");
        assert!(fixture.file_exists("cairom.toml"));
        assert!(fixture.read_file("cairom.toml").contains("test_project"));

        // Test adding a file in a subdirectory
        fixture.add_file("src/lib.cm", "func lib_func() {}");
        assert!(fixture.file_exists("src/lib.cm"));

        // Test root path and URLs
        let root_path = fixture.root_path();
        assert!(root_path.exists());

        let root_url = fixture.root_url();
        assert!(root_url.as_str().starts_with("file://"));

        let file_url = fixture.file_url("test.cm");
        assert!(file_url.as_str().ends_with("/test.cm"));

        // Test listing files
        let files = fixture.list_files();
        assert_eq!(files.len(), 4);
        assert!(files.contains(&PathBuf::from("test.cm")));
        assert!(files.contains(&PathBuf::from("test2.cm")));
        assert!(files.contains(&PathBuf::from("cairom.toml")));
        assert!(files.contains(&PathBuf::from("src/lib.cm")));
    }

    #[test]
    fn test_fixture_cleanup() {
        let temp_path;
        {
            let fixture = Fixture::new();
            temp_path = fixture.root_path();
            assert!(temp_path.exists());
            // Fixture will be dropped here
        }
        // Temporary directory should be cleaned up
        assert!(!temp_path.exists());
    }
}
