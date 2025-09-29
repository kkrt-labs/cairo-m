use std::path::PathBuf;
use std::process::Command;

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
