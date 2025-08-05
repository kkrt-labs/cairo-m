// Test harness for MIR generation tests.
// Provides helpers for loading test files, running the lowering pipeline,
// and checking assertions.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cairo_m_compiler_mir::{generate_mir, MirModule, PrettyPrint};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::File;

// Test database that implements all required traits for MIR generation
#[salsa::db]
#[derive(Clone, Default)]
pub struct TestDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TestDatabase {}

#[salsa::db]
impl cairo_m_compiler_parser::Db for TestDatabase {}

#[salsa::db]
impl cairo_m_compiler_semantic::SemanticDb for TestDatabase {}

#[salsa::db]
impl cairo_m_compiler_mir::MirDb for TestDatabase {}

impl Upcast<dyn cairo_m_compiler_parser::Db> for TestDatabase {
    fn upcast(&self) -> &(dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
}

impl Upcast<dyn cairo_m_compiler_semantic::SemanticDb> for TestDatabase {
    fn upcast(&self) -> &(dyn cairo_m_compiler_semantic::SemanticDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_semantic::SemanticDb + 'static) {
        self
    }
}

pub fn test_db() -> TestDatabase {
    TestDatabase::default()
}

/// The result of running MIR generation on a test source.
pub struct MirOutput {
    pub module: Arc<MirModule>,
    pub mir_string: String,
}

/// Runs the full lowering pipeline on a source string.
pub fn check_mir(source: &str) -> MirOutput {
    let db = test_db();
    let file = File::new(&db, source.to_string(), "test.cm".to_string());
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    let module = match generate_mir(&db, crate_id) {
        Ok(module) => module,
        Err(diagnostics) => {
            panic!(
                "MIR generation failed with diagnostics:\n{:#?}",
                diagnostics
            );
        }
    };
    let mir_string = module.pretty_print(0);
    MirOutput { module, mir_string }
}

/// Represents a single test case loaded from a file.
pub struct MirTest {
    pub source: String,
    pub assertions: HashMap<String, String>,
}

impl MirTest {
    /// Loads a test case from a file path and parses assertion comments.
    pub fn load(path_str: &str) -> Self {
        let path = Path::new("tests/").join(path_str);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read test file {path:?}: {e}"));

        let mut assertions = HashMap::new();
        for line in source.lines() {
            if let Some(rest) = line.trim().strip_prefix("//!ASSERT ") {
                if let Some((key, value)) = rest.split_once(':') {
                    assertions.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        Self { source, assertions }
    }

    /// Checks the parsed assertions against the generated MIR output.
    pub fn check_assertions(&self, output: &MirOutput) {
        for (key, value) in &self.assertions {
            let mut value = value.clone();
            if let Some(comment_pos) = value.find("//") {
                value = value[..comment_pos].trim().to_string();
            }
            match key.as_str() {
                "FUNCTION_COUNT" => {
                    let expected: usize =
                        value.parse().expect("Invalid integer for FUNCTION_COUNT");
                    assert_eq!(
                        output.module.function_count(),
                        expected,
                        "Assertion failed: FUNCTION_COUNT"
                    );
                }
                key if key.starts_with("BLOCK_COUNT(") => {
                    // e.g., BLOCK_COUNT(my_func): 3
                    if let Some(func_name) = key
                        .strip_prefix("BLOCK_COUNT(")
                        .and_then(|s| s.strip_suffix(")"))
                    {
                        let expected: usize = value
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid integer for BLOCK_COUNT: {value}"));
                        let func_id = output
                            .module
                            .lookup_function(func_name.trim())
                            .unwrap_or_else(|| {
                                panic!(
                                    "Function '{}' not found for BLOCK_COUNT assertion",
                                    func_name.trim()
                                )
                            });
                        let func = output.module.get_function(func_id).unwrap();
                        assert_eq!(
                            func.block_count(),
                            expected,
                            "Assertion failed: BLOCK_COUNT for function '{}'",
                            func_name.trim()
                        );
                    } else {
                        panic!(
                            "Invalid format for BLOCK_COUNT assertion. Expected 'BLOCK_COUNT(FUNCTION_NAME)'"
                        );
                    }
                }
                "CONTAINS" => {
                    assert!(
                        output.mir_string.contains(&value),
                        "Assertion failed: CONTAINS '{value}'"
                    );
                }
                "NOT_CONTAINS" => {
                    assert!(
                        !output.mir_string.contains(&value),
                        "Assertion failed: NOT_CONTAINS '{value}'"
                    );
                }
                _ => panic!("Unknown assertion key: {key}"),
            }
        }
    }
}
