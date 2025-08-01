//! End-to-end multi-file codegen integration tests
//!
//! These tests verify that cross-module function calls work correctly
//! through the entire compilation pipeline including code generation.

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_codegen::{compile_module, CodegenDb};
use cairo_m_compiler_mir::{generate_mir, MirDb};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::{File, SemanticDb};

/// Test database that implements all required traits for code generation
#[salsa::db]
#[derive(Clone, Default)]
struct TestDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TestDatabase {}

#[salsa::db]
impl cairo_m_compiler_parser::Db for TestDatabase {}

#[salsa::db]
impl SemanticDb for TestDatabase {}

#[salsa::db]
impl MirDb for TestDatabase {}

#[salsa::db]
impl CodegenDb for TestDatabase {}

impl Upcast<dyn cairo_m_compiler_parser::Db> for TestDatabase {
    fn upcast(&self) -> &(dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_parser::Db + 'static) {
        self
    }
}

impl Upcast<dyn SemanticDb> for TestDatabase {
    fn upcast(&self) -> &(dyn SemanticDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn SemanticDb + 'static) {
        self
    }
}

impl Upcast<dyn MirDb> for TestDatabase {
    fn upcast(&self) -> &(dyn MirDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn MirDb + 'static) {
        self
    }
}

/// Test that cross-module function calls generate correct CASM code
#[test]
fn test_cross_module_codegen() {
    let db = TestDatabase::default();

    // Simple modules for codegen testing
    let main_source = r#"
use math::add;

fn main() -> felt {
    let result = add(10, 20);
    return result;
}
"#;

    let math_source = r#"
fn add(a: felt, b: felt) -> felt {
    return a + b;
}

fn multiply(a: felt, b: felt) -> felt {
    return a * b;
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let math_file = File::new(&db, math_source.to_string(), "math.cm".to_string());

    // Create crate with both modules
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("math".to_string(), math_file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Generate MIR for the entire crate
    let mir_result = generate_mir(&db, crate_id);
    assert!(mir_result.is_ok(), "MIR generation should succeed");

    let mir_module = mir_result.unwrap();

    // Verify all functions are present in MIR
    assert!(
        mir_module.lookup_function("main").is_some(),
        "main function should be present"
    );
    assert!(
        mir_module.lookup_function("add").is_some(),
        "add function should be present"
    );
    assert!(
        mir_module.lookup_function("multiply").is_some(),
        "multiply function should be present"
    );
    assert_eq!(mir_module.function_count(), 3);

    // Compile MIR to CASM
    let compilation_result = compile_module(&mir_module);
    assert!(compilation_result.is_ok(), "Code generation should succeed");

    let program = compilation_result.unwrap();

    // Verify that the program contains instructions
    assert!(
        !program.instructions.is_empty(),
        "Generated program should contain instructions"
    );
}

/// Test code generation with multiple functions in different modules
#[test]
fn test_multi_module_functions_codegen() {
    let db = TestDatabase::default();

    let utilities_source = r#"
fn square(x: felt) -> felt {
    return x * x;
}

fn double(x: felt) -> felt {
    return x + x;
}
"#;

    let calculator_source = r#"
use utilities::square;
use utilities::double;

fn compute(x: felt) -> felt {
    let a = square(x);
    let b = double(x);
    return a + b;
}
"#;

    let main_source = r#"
use calculator::compute;

fn main() -> felt {
    return compute(5);
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let calc_file = File::new(
        &db,
        calculator_source.to_string(),
        "calculator.cm".to_string(),
    );
    let utils_file = File::new(
        &db,
        utilities_source.to_string(),
        "utilities.cm".to_string(),
    );

    // Create crate
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("calculator".to_string(), calc_file);
    modules.insert("utilities".to_string(), utils_file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Generate MIR
    let mir_result = generate_mir(&db, crate_id);
    assert!(mir_result.is_ok(), "MIR generation should succeed");

    let mir_module = mir_result.unwrap();

    // Verify all functions are present
    assert!(mir_module.lookup_function("main").is_some());
    assert!(mir_module.lookup_function("compute").is_some());
    assert!(mir_module.lookup_function("square").is_some());
    assert!(mir_module.lookup_function("double").is_some());
    assert_eq!(mir_module.function_count(), 4);

    // Compile to CASM
    let compilation_result = compile_module(&mir_module);
    assert!(compilation_result.is_ok(), "Code generation should succeed");

    let program = compilation_result.unwrap();
    assert!(
        !program.instructions.is_empty(),
        "Generated program should contain instructions"
    );
}

/// Test that unused functions in imported modules are still compiled
/// (since we compile at the crate level)
#[test]
fn test_unused_function_compilation() {
    let db = TestDatabase::default();

    let main_source = r#"
use library::used_function;

fn main() -> felt {
    return used_function();
}
"#;

    let library_source = r#"
fn used_function() -> felt {
    return 42;
}

fn unused_function() -> felt {
    return 99;
}

fn another_unused() -> felt {
    return 123;
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let lib_file = File::new(&db, library_source.to_string(), "library.cm".to_string());

    // Create crate
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("library".to_string(), lib_file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Generate MIR
    let mir_result = generate_mir(&db, crate_id);
    assert!(mir_result.is_ok(), "MIR generation should succeed");

    let mir_module = mir_result.unwrap();

    // All functions should be present (crate-level compilation)
    assert!(mir_module.lookup_function("main").is_some());
    assert!(mir_module.lookup_function("used_function").is_some());
    assert!(mir_module.lookup_function("unused_function").is_some());
    assert!(mir_module.lookup_function("another_unused").is_some());
    assert_eq!(mir_module.function_count(), 4);

    // Compile to CASM
    let compilation_result = compile_module(&mir_module);
    assert!(compilation_result.is_ok(), "Code generation should succeed");

    let program = compilation_result.unwrap();
    assert!(
        !program.instructions.is_empty(),
        "Generated program should contain instructions"
    );
}

/// Test compilation error handling with invalid multi-module setup
#[test]
fn test_compilation_with_missing_imports() {
    let db = TestDatabase::default();

    let main_source = r#"
use missing_module::missing_function;

fn main() -> felt {
    return missing_function(42);
}
"#;

    // Create crate with only main module (missing the imported module)
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Generate MIR - should succeed with error recovery (generates error values)
    let mir_result = generate_mir(&db, crate_id);
    assert!(
        mir_result.is_ok(),
        "MIR generation should succeed with error recovery"
    );

    let mir_module = mir_result.unwrap();
    assert_eq!(mir_module.function_count(), 1);
    assert!(mir_module.lookup_function("main").is_some());

    // Code generation should fail when encountering error values in MIR
    let compilation_result = compile_module(&mir_module);
    assert!(
        compilation_result.is_err(),
        "Code generation should fail when MIR contains error values"
    );
}

/// Test that compilation results are deterministic regardless of module order
#[test]
fn test_deterministic_compilation() {
    let db = TestDatabase::default();

    let module_a_source = r#"
fn func_a() -> felt {
    return 1;
}
"#;

    let module_b_source = r#"
fn func_b() -> felt {
    return 2;
}
"#;

    let main_source = r#"
use module_a::func_a;
use module_b::func_b;

fn main() -> felt {
    return func_a() + func_b();
}
"#;

    // Test with different module insertion orders
    for main_first in [true, false] {
        let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
        let module_a_file = File::new(&db, module_a_source.to_string(), "module_a.cm".to_string());
        let module_b_file = File::new(&db, module_b_source.to_string(), "module_b.cm".to_string());

        let mut modules = HashMap::new();
        if main_first {
            modules.insert("main".to_string(), main_file);
            modules.insert("module_a".to_string(), module_a_file);
            modules.insert("module_b".to_string(), module_b_file);
        } else {
            modules.insert("module_b".to_string(), module_b_file);
            modules.insert("module_a".to_string(), module_a_file);
            modules.insert("main".to_string(), main_file);
        }

        let crate_id = Crate::new(
            &db,
            modules,
            "main".to_string(),
            PathBuf::from("."),
            "crate_test".to_string(),
        );

        // Generate MIR
        let mir_result = generate_mir(&db, crate_id);
        assert!(
            mir_result.is_ok(),
            "MIR generation should succeed regardless of order"
        );

        let mir_module = mir_result.unwrap();
        assert_eq!(mir_module.function_count(), 3);

        // Compile to CASM
        let compilation_result = compile_module(&mir_module);
        assert!(
            compilation_result.is_ok(),
            "Code generation should succeed regardless of order"
        );

        let program = compilation_result.unwrap();
        assert!(
            !program.instructions.is_empty(),
            "Should generate instructions"
        );
    }
}
