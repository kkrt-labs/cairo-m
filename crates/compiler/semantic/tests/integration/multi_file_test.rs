//! Test for multi-file compilation with cross-module name resolution

use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_parser::SourceFile;
use cairo_m_compiler_semantic::db::{Crate, project_validate_semantics};
use insta::assert_snapshot;

use crate::common::*;

#[test]
fn test_cross_module_function_call() {
    let db = test_db();

    // Create utils module with a function
    let utils_source = r#"
fn add(a: felt, b: felt) -> felt {
    return a + b;
}
"#;

    // Create main module that imports and uses the function
    let main_source = r#"
use utils::add;

fn test() -> felt {
    return add(1, 2);
}
"#;

    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - the import should resolve correctly
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, but got: {:?}",
        diagnostics.all()
    );
}

#[test]
fn test_undefined_import() {
    let db = test_db();

    // Create main module that imports non-existent function
    let main_source = r#"
use utils::nonexistent;

fn test() {
    nonexistent();
}
"#;

    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    // Note: no utils module created
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have errors about undefined module/function
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for undefined import"
    );
}

#[test]
fn test_cross_module_struct_usage() {
    let db = test_db();

    // Create types module with a struct
    let types_source = r#"
struct Point {
    x: felt,
    y: felt
}
"#;

    // Create main module that imports and uses the struct
    let main_source = r#"
use types::Point;

fn test() -> Point {
    return Point { x: 1, y: 2 };
}
"#;

    let types_file = SourceFile::new(&db, types_source.to_string(), "types.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("types".to_string(), types_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - the struct import should resolve correctly
    // Note: We may get warnings about unused imports for types, which is a known limitation
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors, but got: {:?}",
        errors
    );
}

#[test]
fn test_cyclic_imports() {
    let db = test_db();

    // Create module A that imports from B
    let module_a_source = r#"
use module_b::func_b;

fn func_a() {
    func_b();
}
"#;

    // Create module B that imports from A (creating a cycle)
    let module_b_source = r#"
use module_a::func_a;

fn func_b() {
    func_a();
}
"#;

    let module_a_file =
        SourceFile::new(&db, module_a_source.to_string(), "module_a.cm".to_string());
    let module_b_file =
        SourceFile::new(&db, module_b_source.to_string(), "module_b.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("module_a".to_string(), module_a_file);
    modules.insert("module_b".to_string(), module_b_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "module_a".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have error about cyclic imports
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for cyclic imports"
    );

    // Check that the error mentions the cycle
    let has_cycle_error = diagnostics
        .all()
        .iter()
        .any(|d| d.message.contains("Cyclic import") || d.message.contains("cycle"));
    assert!(has_cycle_error, "Expected cyclic import error");
}

#[test]
fn test_multi_file_project_validation() {
    // Test that validates a multi-file project works without panic
    let db = test_db();

    // Create a project with multiple independent modules
    let math_source = r#"
fn multiply(a: felt, b: felt) -> felt {
    return a * b;
}

fn divide(a: felt, b: felt) -> felt {
    return a / b;
}
"#;

    let utils_source = r#"
fn foo(a: felt, b: felt) -> felt {
    // Simplified max function for testing
    return a;
}

fn square(x: felt) -> felt {
    return x * x;
}
"#;

    let main_source = r#"
use math::multiply;
use utils::foo;

fn main() -> felt {
    let x = multiply(5, 6);
    let y = foo(x, 10);
    return y;
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );

    // This should not panic - the panic for multiple modules has been removed
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - all modules are valid and imports resolve correctly
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors, but got: {:?}",
        errors
    );
}

#[test]
fn test_braced_imports() {
    let db = test_db();

    // Create lib module with multiple functions
    let lib_source = r#"
fn a() -> felt {
    return 1;
}

fn b() -> felt {
    return 2;
}

fn c() -> felt {
    return 3;
}
"#;

    // Create main module that imports using braced syntax
    let main_source = r#"
use lib::{a, b};

fn test() -> felt {
    let x = a();
    let y = b();
    return x + y;
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - braced imports should work correctly
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for braced imports, but got: {:?}",
        errors
    );
}

#[test]
fn test_import_name_conflict_with_local() {
    let db = test_db();

    // Create lib module with a function
    let lib_source = r#"
fn my_func() -> felt {
    return 42;
}
"#;

    // Create main module that imports the function AND defines a local with the same name
    let main_source = r#"
use lib::my_func;

fn test() -> felt {
    let my_func = 100;  // Local variable shadows the imported function
    return my_func;     // Should refer to the local variable (100), not imported function
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - local shadowing should work correctly
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for name shadowing, but got: {:?}",
        errors
    );
}

#[test]
fn test_undefined_braced_import_item() {
    let db = test_db();

    // Create lib module with only one function
    let lib_source = r#"
fn a() -> felt {
    return 1;
}
"#;

    // Create main module that tries to import both existing and non-existing functions
    let main_source = r#"
use lib::{a, nonexistent_b};

fn test() -> felt {
    let x = a();
    let y = nonexistent_b();  // This should cause an error
    return x + y;
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have errors about the undefined import
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for undefined braced import item"
    );

    // Take a snapshot of the diagnostics using proper ariadne formatting
    let diagnostic_text = diagnostics
        .all()
        .iter()
        .map(|d| {
            build_diagnostic_message(
                main_source,
                d,
                false, // no color for snapshots
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    assert_snapshot!(diagnostic_text);
}

#[test]
fn test_multiple_import_conflicts() {
    let db = test_db();

    // Create two modules with conflicting function names
    let math_source = r#"
fn calculate() -> felt {
    return 42;
}
"#;

    let utils_source = r#"
fn calculate() -> felt {
    return 100;
}
"#;

    // Create main module that tries to import from both
    let main_source = r#"
use math::calculate;
use utils::calculate;  // This should cause a conflict

fn test() -> felt {
    return calculate();
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Take a snapshot of the diagnostics using proper ariadne formatting
    let diagnostic_text = diagnostics
        .all()
        .iter()
        .map(|d| {
            build_diagnostic_message(
                main_source,
                d,
                false, // no color for snapshots
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    assert_snapshot!(diagnostic_text);
}

#[test]
fn test_braced_import_conflicts() {
    let db = test_db();

    // Create two modules with conflicting function names
    let math_source = r#"
fn calculate() -> felt {
    return 42;
}

fn add() -> felt {
    return 1;
}
"#;

    let utils_source = r#"
fn calculate() -> felt {
    return 100;
}

fn subtract() -> felt {
    return 2;
}
"#;

    // Create main module that imports with conflicts using braced syntax
    let main_source = r#"
use math::{calculate, add};
use utils::{calculate, subtract};  // calculate should cause a conflict

fn test() -> felt {
    return calculate() + add() + subtract();
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have error about duplicate import of calculate
    let duplicate_errors: Vec<_> = diagnostics
        .errors()
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::DuplicateDefinition
            )
        })
        .collect();

    assert!(
        !duplicate_errors.is_empty(),
        "Expected duplicate definition error for conflicting braced imports"
    );

    // Should specifically have an error for 'calculate'
    let has_calculate_conflict = duplicate_errors
        .iter()
        .any(|d| d.message.contains("calculate"));
    assert!(
        has_calculate_conflict,
        "Expected conflict error specifically for 'calculate'"
    );
}

#[test]
fn test_nested_module_imports() {
    let db = test_db();

    // Create a chain of modules: main -> utils -> core
    let core_source = r#"
fn core_function() -> felt {
    return 1;
}
"#;

    let utils_source = r#"
use core::core_function;

fn utils_function() -> felt {
    return core_function() + 1;
}
"#;

    let main_source = r#"
use utils::utils_function;

fn test() -> felt {
    return utils_function();
}
"#;

    let core_file = SourceFile::new(&db, core_source.to_string(), "core.cm".to_string());
    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("core".to_string(), core_file);
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have no errors - nested imports should work correctly
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for nested module imports, but got: {:?}",
        errors
    );
}

#[test]
fn test_self_import_detected() {
    let db = test_db();

    // Create a module that attempts to import from itself
    let main_source = r#"
use main::foo;

fn foo() -> felt {
    return 42;
}

fn test() -> felt {
    return foo();
}
"#;

    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have one error for self-import
    let errors: Vec<_> = diagnostics.errors();
    assert_eq!(
        errors.len(),
        1,
        "Expected one error for self-import, but got: {:?}",
        errors
    );

    // Check the error message - self-imports are detected as cyclic imports
    let error = &errors[0];
    assert!(
        error.message.contains("Cyclic import: main -> main"),
        "Expected cyclic import error message for self-import, but got: {}",
        error.message
    );
}

#[test]
fn test_self_import_with_braced_syntax() {
    let db = test_db();

    // Test self-import with braced syntax
    let utils_source = r#"
use utils::{add, multiply};

fn add(a: felt, b: felt) -> felt {
    return a + b;
}

fn multiply(a: felt, b: felt) -> felt {
    return a * b;
}

fn test() -> felt {
    return add(1, 2);
}
"#;

    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("utils".to_string(), utils_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "utils".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have one error - cyclic import detection catches self-imports
    let errors: Vec<_> = diagnostics.errors();
    assert_eq!(
        errors.len(),
        1,
        "Expected one error for self-import cycle, but got: {:?}",
        errors
    );

    // The error should mention cyclic import
    let error = &errors[0];
    assert!(
        error.message.contains("Cyclic import: utils -> utils"),
        "Expected cyclic import error message for self-import, but got: {}",
        error.message
    );
}

#[test]
fn test_cross_module_type_checking_wrong_argument_types() {
    let db = test_db();

    // Create math module with add function
    let math_source = r#"
fn add(a: felt, b: felt) -> felt {
    return a + b;
}
"#;

    // Create types module with Point struct
    let types_source = r#"
struct Point {
    x: felt,
    y: felt
}
"#;

    // Create main module that calls add with wrong argument type
    let main_source = r#"
use math::add;
use types::Point;

fn test() {
    let p = Point { x: 1, y: 1 };
    add(1, p); // ERROR: second argument should be 'felt', not 'Point'
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let types_file = SourceFile::new(&db, types_source.to_string(), "types.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("types".to_string(), types_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have a TypeMismatch error for the second argument
    let type_errors: Vec<_> = diagnostics
        .errors()
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::TypeMismatch
            )
        })
        .collect();

    assert!(
        !type_errors.is_empty(),
        "Expected TypeMismatch error for wrong argument type"
    );

    // The error should mention the type mismatch
    let has_type_mismatch = type_errors
        .iter()
        .any(|d| d.message.contains("felt") && d.message.contains("Point"));
    assert!(
        has_type_mismatch,
        "Expected error message to mention type mismatch between 'felt' and 'Point'"
    );
}

#[test]
fn test_cross_module_type_checking_wrong_argument_count() {
    let db = test_db();

    // Create math module with add function that takes 2 parameters
    let math_source = r#"
fn add(a: felt, b: felt) -> felt {
    return a + b;
}
"#;

    // Create main module that calls add with wrong number of arguments
    let main_source = r#"
use math::add;

fn test() {
    add(1); // ERROR: Expected 2 arguments, got 1
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have an InvalidFunctionCall error
    let function_errors: Vec<_> = diagnostics
        .errors()
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::InvalidFunctionCall
            )
        })
        .collect();

    assert!(
        !function_errors.is_empty(),
        "Expected InvalidFunctionCall error for wrong argument count"
    );

    // The error should mention arity mismatch
    let has_arity_error = function_errors
        .iter()
        .any(|d| d.message.contains("expects 2 argument") && d.message.contains("1 were provided"));
    assert!(
        has_arity_error,
        "Expected error message to mention arity mismatch. Actual errors: {:?}",
        function_errors
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_cross_module_invalid_struct_instantiation() {
    let db = test_db();

    // Create types module with Point struct
    let types_source = r#"
struct Point {
    x: felt,
    y: felt
}
"#;

    // Create main module that instantiates struct incorrectly
    let main_source = r#"
use types::Point;

fn test() {
    // ERROR: wrong field name 'z' and missing field 'y'
    let p = Point { x: 1, z: 2 };
}
"#;

    let types_file = SourceFile::new(&db, types_source.to_string(), "types.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("types".to_string(), types_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have errors for invalid struct literal
    let has_struct_errors = diagnostics.errors().into_iter().any(|d| {
        matches!(
            d.code,
            cairo_m_compiler_diagnostics::DiagnosticCode::InvalidStructLiteral
                | cairo_m_compiler_diagnostics::DiagnosticCode::InvalidFieldAccess
        )
    });

    assert!(
        has_struct_errors,
        "Expected InvalidStructLiteral or InvalidFieldAccess errors"
    );

    // Should have errors mentioning missing field 'y' and unknown field 'z'
    let all_errors = diagnostics.errors();
    let has_missing_field_error = all_errors.iter().any(|d| d.message.contains("'y'"));
    let has_unknown_field_error = all_errors.iter().any(|d| d.message.contains("'z'"));

    assert!(
        has_missing_field_error,
        "Expected error message to mention missing field 'y'"
    );
    assert!(
        has_unknown_field_error,
        "Expected error message to mention unknown field 'z'"
    );
}

#[test]
fn test_duplicate_import_same_item() {
    let db = test_db();

    // Create math module with add function
    let math_source = r#"
fn add(a: felt, b: felt) -> felt {
    return a + b;
}
"#;

    // Create main module that imports add twice
    let main_source = r#"
use math::add;
use math::add; // ERROR: 'add' is already imported

fn test() {
    add(1, 2);
}
"#;

    let math_file = SourceFile::new(&db, math_source.to_string(), "math.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("math".to_string(), math_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have a DuplicateDefinition error
    let duplicate_errors: Vec<_> = diagnostics
        .errors()
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::DuplicateDefinition
            )
        })
        .collect();

    assert!(
        !duplicate_errors.is_empty(),
        "Expected DuplicateDefinition error for duplicate import"
    );

    // The error should mention 'add' being duplicated
    let has_duplicate_add = duplicate_errors
        .iter()
        .any(|d| d.message.contains("Duplicate definition of 'add'"));
    assert!(
        has_duplicate_add,
        "Expected error message to mention duplicate definition of 'add'"
    );
}

#[test]
#[ignore = "Known issue: conflicts between imports and local definitions are not yet detected"]
fn test_import_conflict_with_top_level_definition() {
    let db = test_db();

    // Create lib module with my_func
    let lib_source = r#"
fn my_func() -> felt {
    return 1;
}
"#;

    // Create main module that imports my_func AND defines its own my_func
    let main_source = r#"
use lib::my_func;

// ERROR: 'my_func' is defined multiple times
fn my_func() -> felt {
    return 2;
}

fn test() -> felt {
    return my_func();
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have a DuplicateDefinition error
    let duplicate_errors: Vec<_> = diagnostics
        .errors()
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::DuplicateDefinition
            )
        })
        .collect();

    assert!(
        !duplicate_errors.is_empty(),
        "Expected DuplicateDefinition error for conflicting import and definition"
    );

    // The error should mention 'my_func' being duplicated
    let has_duplicate_my_func = duplicate_errors
        .iter()
        .any(|d| d.message.contains("Duplicate definition of 'my_func'"));
    assert!(
        has_duplicate_my_func,
        "Expected error message to mention duplicate definition of 'my_func'"
    );
}

#[test]
fn test_unused_imports() {
    let db = test_db();

    // Create lib module with functions and struct
    let lib_source = r#"
fn my_func() {}

struct Point {
    x: felt
}
"#;

    // Create main module that imports but never uses them
    let main_source = r#"
use lib::{my_func, Point}; // Both 'my_func' and 'Point' are unused

fn test() -> felt {
    return 0;
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let crate_id = Crate::new(
        &db,
        modules,
        "main".to_string(),
        PathBuf::from("."),
        "crate_test".to_string(),
    );
    let diagnostics = project_validate_semantics(&db, crate_id);

    // Should have warnings for unused imports
    let warnings: Vec<_> = diagnostics.warnings();
    assert!(!warnings.is_empty(), "Expected warnings for unused imports");

    // Should have UnusedVariable warnings
    let unused_warnings: Vec<_> = warnings
        .into_iter()
        .filter(|d| {
            matches!(
                d.code,
                cairo_m_compiler_diagnostics::DiagnosticCode::UnusedVariable
            )
        })
        .collect();

    assert!(
        unused_warnings.len() >= 2,
        "Expected at least 2 UnusedVariable warnings, got: {}",
        unused_warnings.len()
    );

    // Check that the warnings mention both my_func and Point
    let has_my_func_warning = unused_warnings
        .iter()
        .any(|d| d.message.contains("my_func"));
    let has_point_warning = unused_warnings.iter().any(|d| d.message.contains("Point"));

    assert!(
        has_my_func_warning,
        "Expected warning for unused import 'my_func'"
    );
    assert!(
        has_point_warning,
        "Expected warning for unused import 'Point'"
    );
}
