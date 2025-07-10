//! Test for multi-file compilation with cross-module name resolution

use std::collections::HashMap;

use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_parser::SourceFile;
use cairo_m_compiler_semantic::db::{Project, project_validate_semantics};
use insta::assert_snapshot;

use crate::test_db;

#[test]
fn test_cross_module_function_call() {
    let db = test_db();

    // Create utils module with a function
    let utils_source = r#"
func add(a: felt, b: felt) -> felt {
    return a + b;
}
"#;

    // Create main module that imports and uses the function
    let main_source = r#"
use utils::add;

func test() -> felt {
    return add(1, 2);
}
"#;

    let utils_file = SourceFile::new(&db, utils_source.to_string(), "utils.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("utils".to_string(), utils_file);
    modules.insert("main".to_string(), main_file);

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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

func test() {
    nonexistent();
}
"#;

    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    // Note: no utils module created
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), main_file);

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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

func test() -> Point {
    return Point { x: 1, y: 2 };
}
"#;

    let types_file = SourceFile::new(&db, types_source.to_string(), "types.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("types".to_string(), types_file);
    modules.insert("main".to_string(), main_file);

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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

func func_a() {
    func_b();
}
"#;

    // Create module B that imports from A (creating a cycle)
    let module_b_source = r#"
use module_a::func_a;

func func_b() {
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

    let project = Project::new(&db, modules, "module_a".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func multiply(a: felt, b: felt) -> felt {
    return a * b;
}

func divide(a: felt, b: felt) -> felt {
    return a / b;
}
"#;

    let utils_source = r#"
func foo(a: felt, b: felt) -> felt {
    // Simplified max function for testing
    return a;
}

func square(x: felt) -> felt {
    return x * x;
}
"#;

    let main_source = r#"
use math::multiply;
use utils::foo;

func main() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());

    // This should not panic - the panic for multiple modules has been removed
    let diagnostics = project_validate_semantics(&db, project);

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
func a() -> felt {
    return 1;
}

func b() -> felt {
    return 2;
}

func c() -> felt {
    return 3;
}
"#;

    // Create main module that imports using braced syntax
    let main_source = r#"
use lib::{a, b};

func test() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func my_func() -> felt {
    return 42;
}
"#;

    // Create main module that imports the function AND defines a local with the same name
    let main_source = r#"
use lib::my_func;

func test() -> felt {
    let my_func = 100;  // Local variable shadows the imported function
    return my_func;     // Should refer to the local variable (100), not imported function
}
"#;

    let lib_file = SourceFile::new(&db, lib_source.to_string(), "lib.cm".to_string());
    let main_file = SourceFile::new(&db, main_source.to_string(), "main.cm".to_string());

    let mut modules = HashMap::new();
    modules.insert("lib".to_string(), lib_file);
    modules.insert("main".to_string(), main_file);

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func a() -> felt {
    return 1;
}
"#;

    // Create main module that tries to import both existing and non-existing functions
    let main_source = r#"
use lib::{a, nonexistent_b};

func test() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func calculate() -> felt {
    return 42;
}
"#;

    let utils_source = r#"
func calculate() -> felt {
    return 100;
}
"#;

    // Create main module that tries to import from both
    let main_source = r#"
use math::calculate;
use utils::calculate;  // This should cause a conflict

func test() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func calculate() -> felt {
    return 42;
}

func add() -> felt {
    return 1;
}
"#;

    let utils_source = r#"
func calculate() -> felt {
    return 100;
}

func subtract() -> felt {
    return 2;
}
"#;

    // Create main module that imports with conflicts using braced syntax
    let main_source = r#"
use math::{calculate, add};
use utils::{calculate, subtract};  // calculate should cause a conflict

func test() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

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
func core_function() -> felt {
    return 1;
}
"#;

    let utils_source = r#"
use core::core_function;

func utils_function() -> felt {
    return core_function() + 1;
}
"#;

    let main_source = r#"
use utils::utils_function;

func test() -> felt {
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

    let project = Project::new(&db, modules, "main".to_string());
    let diagnostics = project_validate_semantics(&db, project);

    // Should have no errors - nested imports should work correctly
    let errors: Vec<_> = diagnostics.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for nested module imports, but got: {:?}",
        errors
    );
}
