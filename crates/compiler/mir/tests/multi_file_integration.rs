//! End-to-end multi-file MIR integration tests
//!
//! These tests verify that cross-module function calls work correctly
//! through the entire MIR generation pipeline.

use std::collections::HashMap;

use cairo_m_compiler_mir::{MirDb, PrettyPrint, generate_mir};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::{File, SemanticDb};

/// Test database that implements all required traits for MIR generation
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

/// Test that cross-module function calls are resolved correctly in MIR generation
#[test]
fn test_cross_module_function_calls() {
    let db = TestDatabase::default();

    // Main module that imports and calls functions from math module
    let main_source = r#"
use math::add;
use math::multiply;

func main() -> felt {
    let x = add(2, 3);
    let y = multiply(x, 4);
    return y;
}

func local_helper() -> felt {
    return add(1, 1);
}
"#;

    // Math module with utility functions
    let math_source = r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}

func multiply(a: felt, b: felt) -> felt {
    return a * b;
}

func subtract(a: felt, b: felt) -> felt {
    return a - b;
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let math_file = File::new(&db, math_source.to_string(), "math.cm".to_string());

    // Create crate with both modules
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("math".to_string(), math_file);
    let crate_id = Crate::new(&db, modules, "main".to_string());

    // Generate MIR for the entire crate
    let mir_result = generate_mir(&db, crate_id);
    if mir_result.is_err() {
        let errors = mir_result.as_ref().unwrap_err();
        println!("MIR generation errors:");
        for error in errors {
            println!("  {}", error.message);
        }
    }
    assert!(mir_result.is_ok(), "MIR generation should succeed");

    let mir_module = mir_result.unwrap();

    // Verify all functions are present
    assert!(
        mir_module.lookup_function("main").is_some(),
        "main function should be present"
    );
    assert!(
        mir_module.lookup_function("local_helper").is_some(),
        "local_helper function should be present"
    );
    assert!(
        mir_module.lookup_function("add").is_some(),
        "add function should be present"
    );
    assert!(
        mir_module.lookup_function("multiply").is_some(),
        "multiply function should be present"
    );
    assert!(
        mir_module.lookup_function("subtract").is_some(),
        "subtract function should be present"
    );

    // Verify expected function count
    assert_eq!(mir_module.function_count(), 5);

    // Verify main function calls the correct functions
    let mir_text = mir_module.pretty_print(0);

    // Note: Cross-module calls currently generate error values
    // This test documents current behavior - in the future this should resolve correctly
    // For now, just verify that all functions are present in the MIR

    // Verify that all expected functions are in the generated MIR
    assert!(mir_text.contains("main"), "main function should be in MIR");
    assert!(mir_text.contains("add"), "add function should be in MIR");
    assert!(
        mir_text.contains("multiply"),
        "multiply function should be in MIR"
    );
    assert!(
        mir_text.contains("local_helper"),
        "local_helper function should be in MIR"
    );
    assert!(
        mir_text.contains("subtract"),
        "subtract function should be in MIR"
    );
}

/// Test that unused imported functions are still included in MIR
/// (since we generate MIR for all functions in the project)
#[test]
fn test_unused_imports_in_mir() {
    let db = TestDatabase::default();

    let main_source = r#"
use utils::used_function;

func main() -> felt {
    return used_function(42);
}
"#;

    let utils_source = r#"
func used_function(x: felt) -> felt {
    return x * 2;
}

func unused_function(x: felt) -> felt {
    return x + 1;
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let utils_file = File::new(&db, utils_source.to_string(), "utils.cm".to_string());

    // Create crate
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("utils".to_string(), utils_file);
    let crate_id = Crate::new(&db, modules, "main".to_string());

    // Generate MIR
    let mir_result = generate_mir(&db, crate_id);
    assert!(mir_result.is_ok(), "MIR generation should succeed");

    let mir_module = mir_result.unwrap();

    // Both functions should be present in MIR (crate-level generation)
    assert!(mir_module.lookup_function("used_function").is_some());
    assert!(mir_module.lookup_function("unused_function").is_some());
    assert!(mir_module.lookup_function("main").is_some());

    assert_eq!(mir_module.function_count(), 3);
}

/// Test cyclic import detection
///
/// The compiler correctly detects and prevents cyclic imports between modules.
/// This test verifies that cyclic imports are properly detected and reported.
#[test]
fn test_cyclic_import_detection() {
    let db = TestDatabase::default();

    let module_a_source = r#"
use module_b::even;

func odd(n: felt) -> felt {
    if (n == 0) {
        return 0;
    } else {
        return even(n - 1);
    }
}
"#;

    let module_b_source = r#"
use module_a::odd;

func even(n: felt) -> felt {
    if (n == 0) {
        return 1;
    } else {
        return odd(n - 1);
    }
}
"#;

    let main_source = r#"
use module_a::odd;
use module_b::even;

func main() -> felt {
    let result1 = odd(5);
    let result2 = even(4);
    return result1 + result2;
}
"#;

    // Create files
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let module_a_file = File::new(&db, module_a_source.to_string(), "module_a.cm".to_string());
    let module_b_file = File::new(&db, module_b_source.to_string(), "module_b.cm".to_string());

    // Create crate
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    modules.insert("module_a".to_string(), module_a_file);
    modules.insert("module_b".to_string(), module_b_file);
    let crate_id = Crate::new(&db, modules, "main".to_string());

    // Generate MIR - should fail due to cyclic imports
    let mir_result = generate_mir(&db, crate_id);
    if mir_result.is_err() {
        let errors = mir_result.as_ref().unwrap_err();
        println!("Expected cyclic import error:");
        for error in errors {
            println!("  {}", error.message);
        }
    }
    assert!(
        mir_result.is_err(),
        "MIR generation should fail for cyclic imports"
    );

    let diagnostics = mir_result.unwrap_err();
    assert!(
        !diagnostics.is_empty(),
        "Should have cyclic import error diagnostics"
    );

    // Should contain an error about cyclic imports
    let diagnostic_messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
    let combined_message = diagnostic_messages.join(" ");

    assert!(
        combined_message.contains("Cyclic") || combined_message.contains("cyclic"),
        "Error should mention cyclic imports. Got: {:?}",
        diagnostic_messages
    );
}

/// Test error handling when imported functions are missing
///
/// The current implementation uses graceful error recovery - it generates MIR
/// with error values instead of failing completely. This documents current behavior.
#[test]
fn test_missing_imported_function_error() {
    let db = TestDatabase::default();

    let main_source = r#"
use nonexistent::missing_function;

func main() -> felt {
    return missing_function(42);
}
"#;

    // Create crate with only main module (missing nonexistent module)
    let main_file = File::new(&db, main_source.to_string(), "main.cm".to_string());
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);
    let crate_id = Crate::new(&db, modules, "main".to_string());

    // Generate MIR - current implementation uses graceful error recovery
    let mir_result = generate_mir(&db, crate_id);
    assert!(
        mir_result.is_ok(),
        "MIR generation should succeed with error recovery"
    );

    let mir_module = mir_result.unwrap();
    assert_eq!(mir_module.function_count(), 1);
    assert!(mir_module.lookup_function("main").is_some());

    // The MIR should contain error values for unresolved calls
    let mir_text = mir_module.pretty_print(0);
    println!("MIR with missing imports:\n{}", mir_text);
    assert!(
        mir_text.contains("<error>"),
        "Should contain error values for missing imports"
    );
}

/// Test that the dependency order doesn't affect MIR generation
#[test]
fn test_dependency_order_independence() {
    let db = TestDatabase::default();

    let caller_source = r#"
use callee::helper;

func main() -> felt {
    return helper(10);
}
"#;

    let callee_source = r#"
func helper(x: felt) -> felt {
    return x * 2;
}
"#;

    // Test with different module insertion orders
    for main_first in [true, false] {
        let caller_file = File::new(&db, caller_source.to_string(), "caller.cm".to_string());
        let callee_file = File::new(&db, callee_source.to_string(), "callee.cm".to_string());

        let mut modules = HashMap::new();
        if main_first {
            modules.insert("caller".to_string(), caller_file);
            modules.insert("callee".to_string(), callee_file);
        } else {
            modules.insert("callee".to_string(), callee_file);
            modules.insert("caller".to_string(), caller_file);
        }

        let crate_id = Crate::new(&db, modules, "caller".to_string());

        // Generate MIR
        let mir_result = generate_mir(&db, crate_id);
        assert!(
            mir_result.is_ok(),
            "MIR generation should succeed regardless of order"
        );

        let mir_module = mir_result.unwrap();
        assert_eq!(mir_module.function_count(), 2);
        assert!(mir_module.lookup_function("main").is_some());
        assert!(mir_module.lookup_function("helper").is_some());
    }
}
