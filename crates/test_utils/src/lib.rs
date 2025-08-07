#![allow(clippy::option_if_let_else)]

pub mod mdtest;

use once_cell::sync::Lazy;
use std::path::PathBuf;

pub(crate) static WORKSPACE_ROOT: Lazy<PathBuf> = Lazy::new(|| {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    loop {
        if current.join("Cargo.toml").exists() {
            let cargo_toml = std::fs::read_to_string(current.join("Cargo.toml"))
                .expect("Failed to read Cargo.toml");
            if cargo_toml.contains("[workspace]") {
                return current;
            }
        }

        current = current
            .parent()
            .expect("Could not find workspace root")
            .to_path_buf();
    }
});

pub fn test_data_path() -> PathBuf {
    WORKSPACE_ROOT.join("test_data")
}

pub fn mdtest_path() -> PathBuf {
    WORKSPACE_ROOT.join("mdtest")
}

/// Get the path to a test fixture file relative to the test_data directory
///
/// ## Arguments
/// * `name` - The relative path to the fixture file (e.g., "arithmetic/add.cm")
///
/// ## Returns
/// The absolute path to the fixture file
pub fn fixture_path(name: &str) -> PathBuf {
    test_data_path().join(name)
}

/// Read the contents of a test fixture file
///
/// ## Arguments
/// * `name` - The relative path to the fixture file (e.g., "arithmetic/add.cm")
///
/// ## Returns
/// The contents of the fixture file as a String
pub fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture '{}': {}", path.display(), e))
}

/// List all fixtures in a given subdirectory
///
/// ## Arguments
/// * `subdir` - The subdirectory to list (e.g., "arithmetic")
///
/// ## Returns
/// A vector of file names (without the directory prefix)
pub fn list_fixtures(subdir: &str) -> Vec<String> {
    let dir_path = WORKSPACE_ROOT.join("test_data").join(subdir);

    std::fs::read_dir(&dir_path)
        .unwrap_or_else(|e| panic!("Failed to read directory '{}': {}", dir_path.display(), e))
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.extension()?.to_str()? == "cm" {
                    path.file_name()?.to_str().map(String::from)
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Check if a fixture exists
///
/// ## Arguments
/// * `name` - The relative path to the fixture file
///
/// ## Returns
/// true if the fixture exists, false otherwise
pub fn fixture_exists(name: &str) -> bool {
    fixture_path(name).exists()
}

/// Discover all Cairo-M test files in the test_data directory
///
/// ## Returns
/// A vector of tuples containing (relative_path, file_stem) for each .cm file
pub fn discover_all_fixtures() -> Vec<(String, String)> {
    let test_data = test_data_path();
    let mut fixtures = Vec::new();
    discover_fixtures_recursive(&test_data, &test_data, &mut fixtures);
    fixtures.sort(); // Ensure consistent ordering
    fixtures
}

fn discover_fixtures_recursive(
    base_path: &std::path::Path,
    current_path: &std::path::Path,
    fixtures: &mut Vec<(String, String)>,
) {
    if let Ok(entries) = std::fs::read_dir(current_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_fixtures_recursive(base_path, &path, fixtures);
            } else if path.extension().and_then(|s| s.to_str()) == Some("cm") {
                // Get relative path from test_data directory
                if let Ok(relative_path) = path.strip_prefix(base_path) {
                    let relative_str = relative_path.to_string_lossy().replace('\\', "/");
                    let file_stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    fixtures.push((relative_str, file_stem));
                }
            }
        }
    }
}

/// Get a categorized list of all fixtures
///
/// ## Returns
/// A map from category (subdirectory) to list of fixture files
pub fn discover_fixtures_by_category() -> std::collections::BTreeMap<String, Vec<String>> {
    let mut categories = std::collections::BTreeMap::new();

    for (path, _stem) in discover_all_fixtures() {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            // Category is the first directory
            let category = parts[0].to_string();
            categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(path);
        } else {
            // Files in root go to "root" category
            categories
                .entry("root".to_string())
                .or_insert_with(Vec::new)
                .push(path);
        }
    }

    categories
}
