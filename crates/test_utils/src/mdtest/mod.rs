pub mod config;
pub mod parser;

pub use config::{Location, MdTestConfig, TestMetadata};
pub use parser::{extract_tests, MdTest, ParseError};

use std::path::{Path, PathBuf};

/// Get the path to the mdtest directory
pub fn mdtest_path() -> PathBuf {
    crate::WORKSPACE_ROOT.join("mdtest")
}

/// Discover all markdown test files in the mdtest directory
pub fn discover_markdown_files() -> Vec<PathBuf> {
    let mdtest_dir = mdtest_path();
    let mut files = Vec::new();

    if mdtest_dir.exists() {
        discover_files_recursive(&mdtest_dir, &mut files);
    }

    files
}

fn discover_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_files_recursive(&path, files);
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
}

/// Extract all tests from all markdown files in the mdtest directory
pub fn extract_all_tests() -> Result<Vec<(PathBuf, Vec<MdTest>)>, ParseError> {
    let mut all_tests = Vec::new();

    for md_file in discover_markdown_files() {
        let tests = extract_tests(&md_file)?;
        all_tests.push((md_file, tests));
    }

    Ok(all_tests)
}
