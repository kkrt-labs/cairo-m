//! Test that files with identical content but different paths get unique file IDs
//!
//! This addresses the critical bug where content-based hashing could lead to
//! silent definition collisions between files with the same content.

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_mir::{MirDb, generate_mir};
use cairo_m_compiler_parser::Upcast;
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::{File, SemanticDb};

/// Test database that implements all required traits
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

/// Test that files with identical content but different paths generate different file IDs
#[test]
fn test_identical_content_different_paths_unique_file_ids() {
    let db = TestDatabase::default();

    // Same exact source code content
    let identical_source = r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}

func main() -> felt {
    return add(10, 20);
}
"#;

    // Create two files with identical content but different paths
    let file1 = File::new(
        &db,
        identical_source.to_string(),
        "module1/math.cm".to_string(),
    );
    let file2 = File::new(
        &db,
        identical_source.to_string(),
        "module2/math.cm".to_string(),
    );

    // Verify the files have different paths but same content
    assert_eq!(
        file1.text(&db),
        file2.text(&db),
        "Files should have identical content"
    );
    assert_ne!(
        file1.file_path(&db),
        file2.file_path(&db),
        "Files should have different paths"
    );

    // Create two separate single-file crates
    let mut modules1 = HashMap::new();
    modules1.insert("math".to_string(), file1);
    let crate1 = Crate::new(
        &db,
        modules1,
        "math".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    let mut modules2 = HashMap::new();
    modules2.insert("math".to_string(), file2);
    let crate2 = Crate::new(
        &db,
        modules2,
        "math".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // Generate MIR for both crates
    let mir_result1 = generate_mir(&db, crate1);
    let mir_result2 = generate_mir(&db, crate2);

    assert!(
        mir_result1.is_ok(),
        "MIR generation should succeed for crate1"
    );
    assert!(
        mir_result2.is_ok(),
        "MIR generation should succeed for crate2"
    );

    let mir_module1 = mir_result1.unwrap();
    let mir_module2 = mir_result2.unwrap();

    // Both modules should have the same functions
    assert_eq!(mir_module1.function_count(), mir_module2.function_count());
    assert!(mir_module1.lookup_function("add").is_some());
    assert!(mir_module1.lookup_function("main").is_some());
    assert!(mir_module2.lookup_function("add").is_some());
    assert!(mir_module2.lookup_function("main").is_some());

    // The critical test: functions with identical content in different files
    // should have different MirDefinitionIds due to different file_ids
    let add_func1 = mir_module1
        .get_function(mir_module1.lookup_function("add").unwrap())
        .unwrap();
    let add_func2 = mir_module2
        .get_function(mir_module2.lookup_function("add").unwrap())
        .unwrap();
    let main_func1 = mir_module1
        .get_function(mir_module1.lookup_function("main").unwrap())
        .unwrap();
    let main_func2 = mir_module2
        .get_function(mir_module2.lookup_function("main").unwrap())
        .unwrap();

    // Extract file_ids from the functions' MirDefinitionIds in their locals
    // This tests that the file_id generation is now path-based, not content-based
    let add_file_id1 = add_func1.locals.keys().next().map(|def| def.file_id);
    let add_file_id2 = add_func2.locals.keys().next().map(|def| def.file_id);
    let main_file_id1 = main_func1.locals.keys().next().map(|def| def.file_id);
    let main_file_id2 = main_func2.locals.keys().next().map(|def| def.file_id);

    // File IDs should be different even though content is identical
    if let (Some(id1), Some(id2)) = (add_file_id1, add_file_id2) {
        assert_ne!(
            id1, id2,
            "Files with identical content but different paths should have different file_ids"
        );
    }

    if let (Some(id1), Some(id2)) = (main_file_id1, main_file_id2) {
        assert_ne!(
            id1, id2,
            "Files with identical content but different paths should have different file_ids"
        );
    }

    println!(
        "✅ File ID collision bug fixed: identical content, different paths = different file IDs"
    );
}

/// Test that the same file consistently produces the same file ID
#[test]
fn test_same_file_consistent_file_id() {
    let db = TestDatabase::default();

    let source = r#"
func test() -> felt {
    return 42;
}
"#;

    let file = File::new(&db, source.to_string(), "test.cm".to_string());

    // Create crate and generate MIR multiple times
    let mut modules = HashMap::new();
    modules.insert("test".to_string(), file);
    let crate_obj = Crate::new(
        &db,
        modules,
        "test".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    let mir_result1 = generate_mir(&db, crate_obj);
    let mir_result2 = generate_mir(&db, crate_obj);

    assert!(mir_result1.is_ok() && mir_result2.is_ok());

    let mir_module1 = mir_result1.unwrap();
    let mir_module2 = mir_result2.unwrap();

    // Get file IDs from both generations
    let func1 = mir_module1
        .get_function(mir_module1.lookup_function("test").unwrap())
        .unwrap();
    let func2 = mir_module2
        .get_function(mir_module2.lookup_function("test").unwrap())
        .unwrap();

    let file_id1 = func1.locals.keys().next().map(|def| def.file_id);
    let file_id2 = func2.locals.keys().next().map(|def| def.file_id);

    // Same file should produce same file ID consistently
    assert_eq!(
        file_id1, file_id2,
        "Same file should produce consistent file IDs"
    );

    println!("✅ Same file produces consistent file IDs");
}
