use std::env;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_compile_project() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .expect("Failed to get parent directory")
        .parent()
        .expect("Failed to get workspace root");
    let project_root = workspace_root.join("cairo-m-project");

    let res = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--input")
        .arg(project_root)
        .output()
        .expect("Failed to compile standalone");

    assert!(
        res.status.success(),
        "Failed to compile standalone with error: {}",
        String::from_utf8_lossy(&res.stderr)
    );
}

#[test]
fn test_compile_standalone() {
    let file_to_test = cairo_m_test_utils::fixture_path("functions/fib.cm");
    env::set_var("RUST_LOG", "info");
    let res = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--input")
        .arg(file_to_test)
        .arg("--verbose")
        .output()
        .expect("Failed to compile standalone file");
    println!("{}", String::from_utf8_lossy(&res.stdout));
    assert!(
        res.status.success(),
        "Failed to compile standalone with error: {}",
        String::from_utf8_lossy(&res.stderr)
    );
}
