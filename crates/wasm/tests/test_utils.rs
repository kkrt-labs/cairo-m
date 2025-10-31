use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, LazyLock, Mutex};

use cairo_m_common::Program;
use cairo_m_common::abi_codec::CairoMValue;
use cairo_m_common::program::AbiType;
/// These tests compare the output of the compiled cairo-m with result from the womir interpreter
use cairo_m_compiler_codegen::compile_module;
use cairo_m_compiler_mir::PassManager;
use cairo_m_wasm::loader::BlocklessDagModule;
use cairo_m_wasm::lowering::lower_program_to_mir;
use wasmtime::{Engine, Module};

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

// Track which Rust projects have been built
static BUILT_RUST_PROJECTS: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

/// HashMap of compiled Cairo-M programs
static COMPILED_PROGRAMS: LazyLock<Mutex<HashMap<u64, Arc<Program>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub static WASMTIME_ENGINE: LazyLock<Engine> = LazyLock::new(Engine::default);

/// HashMap of WASMtime modules
static WASMTIME_MODULES: LazyLock<Mutex<HashMap<u64, Module>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Returns a WASMtime module from a binary wasm program
/// If the module has not been built, it will be built and cached.
/// Else, it will be returned from the cache.
pub fn get_or_build_wasmtime_module(bytes: &[u8]) -> Module {
    let key = hash_bytes(bytes);
    let mut cache = WASMTIME_MODULES.lock().unwrap();
    if let Some(m) = cache.get(&key) {
        return m.clone();
    }
    let module = Module::from_binary(&WASMTIME_ENGINE, bytes).unwrap();
    cache.insert(key, module.clone());
    module
}

/// Returns a Cairo-M program from a binary wasm program
/// If the program has not been built, it will be built and cached.
/// Else, it will be returned from the cache.
pub fn get_or_build_cairo_program(bytes: &[u8]) -> Arc<Program> {
    let key = hash_bytes(bytes);
    let mut cache = COMPILED_PROGRAMS.lock().unwrap();
    if let Some(p) = cache.get(&key) {
        return Arc::clone(p);
    }
    let dag_module = BlocklessDagModule::from_bytes(bytes).unwrap();
    let mir_module = lower_program_to_mir(&dag_module, PassManager::standard_pipeline()).unwrap();
    let compiled_module = compile_module(&mir_module).unwrap();
    let arc = Arc::new(compiled_module);
    cache.insert(key, Arc::clone(&arc));
    arc
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

fn build_wasm_from_rust(path: &PathBuf) {
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

/// Convert CairoM return values to u32 following the ABI, mirroring runner tests behavior.
pub fn collect_u32s_by_abi(
    values: &[CairoMValue],
    abi_returns: &[cairo_m_common::program::AbiSlot],
) -> Vec<u32> {
    assert_eq!(
        values.len(),
        abi_returns.len(),
        "Return value count mismatch: got {} but ABI declares {}",
        values.len(),
        abi_returns.len()
    );
    values
        .iter()
        .zip(abi_returns.iter())
        .map(|(v, slot)| match (&slot.ty, v) {
            (AbiType::U32, CairoMValue::U32(n)) => *n,
            (AbiType::Bool, CairoMValue::Bool(b)) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            // For felt returns, WOMIR currently models i32 as u32; not expected in current WASM tests.
            (AbiType::Felt, CairoMValue::Felt(f)) => f.0,
            _ => panic!(
                "Type/value mismatch in return: ABI {:?}, value {:?}",
                slot.ty, v
            ),
        })
        .collect()
}
