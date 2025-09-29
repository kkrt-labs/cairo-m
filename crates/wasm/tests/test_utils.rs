#![allow(dead_code)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

// Track which .wasm files have been built
static BUILT_FILES: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

// Track which Rust projects have been built
static BUILT_RUST_PROJECTS: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

/// This function is called by every test that uses a .wat file.
/// It builds the corresponding .wasm file the first time it is called
/// and then skips subsequent calls.
pub fn ensure_wasm_file_built(wat_file: &str) {
    let mut built_files = BUILT_FILES.lock().unwrap();

    if !built_files.contains(wat_file) {
        let path = PathBuf::from(wat_file);
        let wasm_path = path.with_extension("wasm");

        if !wasm_path.exists() {
            build_wasm(&path);
        }

        built_files.insert(wat_file.to_string());
    }
}

/// This function is called by every test that uses a Rust project.
/// It builds the project the first time it is called
/// and then skips subsequent calls.
pub fn ensure_rust_wasm_built(project_path: &str) {
    let mut built_projects = BUILT_RUST_PROJECTS.lock().unwrap();

    if !built_projects.contains(project_path) {
        build_wasm_from_rust(&PathBuf::from(project_path));
        built_projects.insert(project_path.to_string());
    }
}

pub fn build_wasm(path: &PathBuf) {
    assert!(path.exists(), "Target file does not exist: {path:?}",);
    let output = Command::new("wat2wasm")
        .arg(path)
        .arg("-o")
        .arg(path.with_extension("wasm").to_str().unwrap())
        .output()
        .expect("Failed to run wat2wasm");

    if !output.status.success() {
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(), "wat2wasm failed for {path:?}",);
}

pub fn build_wasm_from_rust(path: &PathBuf) {
    assert!(path.exists(), "Target directory does not exist: {path:?}",);

    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .current_dir(path)
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(), "cargo build failed for {path:?}",);
}
