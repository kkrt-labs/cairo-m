//! Multi-file semantic validation tests using parameterized test infrastructure

use crate::{assert_semantic_parameterized, multi_file};

#[test]
fn test_cross_module_imports() {
    assert_semantic_parameterized! {
        ok: [
            // Valid import and usage
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use utils::add;\nfn test() -> felt { return add(1, 2); }"),
                    ("utils.cm", "fn add(a: felt, b: felt) -> felt { return a + b; }"),
                ]
            ),
            // Braced imports
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use lib::{a, b};\nfn test() -> felt { return a() + b(); }"),
                    ("lib.cm", "fn a() -> felt { return 1; }\nfn b() -> felt { return 2; }"),
                ]
            ),
            // Import from nested modules
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use lib::helper;\nfn run() -> felt { return helper(42); }"),
                    ("lib.cm", "use nested::utils::double;\nfn helper(x: felt) -> felt { return double(x); }"),
                    ("nested/utils.cm", "fn double(x: felt) -> felt { return x * 2; }"),
                ]
            ),
        ],
        err: [
            // Importing a non-existent function
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use utils::nonexistent;\nfn test() { nonexistent(); return; }"),
                    ("utils.cm", "fn add(a: felt, b: felt) -> felt { return a + b; }"),
                ]
            ),
            // Importing from non-existent module
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use missing::add;\nfn test() { add(1, 2); return; }"),
                    ("utils.cm", "fn add(a: felt, b: felt) -> felt { return a + b; }"),
                ]
            ),
        ]
    }
}

#[test]
fn test_cross_module_type_references() {
    assert_semantic_parameterized! {
        ok: [
            // Valid struct usage across modules
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use types::Point;\nfn origin() -> Point { return Point { x: 0, y: 0 }; }"),
                    ("types.cm", "struct Point { x: felt, y: felt }"),
                ]
            ),
            // Function parameter using imported type
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use types::Vector;\nfn magnitude(v: Vector) -> felt { return v.x * v.x + v.y * v.y; }"),
                    ("types.cm", "struct Vector { x: felt, y: felt }"),
                ]
            ),
        ],
        err: [
            // Using undefined type from another module
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use types::Point;\nfn test() -> Rectangle { return Rectangle { width: 10, height: 20 }; }"),
                    ("types.cm", "struct Point { x: felt, y: felt }"),
                ]
            ),
            // Type mismatch in cross-module function call
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use types::Point;\nuse ops::process;\nfn test() { let p = Point { x: 1, y: 2 }; return process(p); }"),
                    ("types.cm", "struct Point { x: felt, y: felt }\nstruct Vector { x: felt, y: felt }"),
                    ("ops.cm", "use types::Vector;\nfn process(v: Vector) { return; }"),
                ]
            ),
        ]
    }
}

#[test]
fn test_circular_dependencies() {
    assert_semantic_parameterized! {
        err: [
            // Direct circular dependency
            multi_file(
                "module_a.cm",
                &[
                    ("module_a.cm", "use module_b::func_b;\nfn func_a() { func_b(); }"),
                    ("module_b.cm", "use module_a::func_a;\nfn func_b() { func_a(); }"),
                ]
            ),
            // Indirect circular dependency through three modules
            multi_file(
                "module_a.cm",
                &[
                    ("module_a.cm", "use module_b::func_b;\nfn func_a() { func_b(); }"),
                    ("module_b.cm", "use module_c::func_c;\nfn func_b() { func_c(); }"),
                    ("module_c.cm", "use module_a::func_a;\nfn func_c() { func_a(); }"),
                ]
            ),
            // Self import
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use main::foo;\nfn foo() {}"),
                ]
            ),
        ]
    }
}

#[test]
fn test_import_conflicts() {
    assert_semantic_parameterized! {
        err: [
            // Duplicate imports from different modules
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use math::calculate;\nuse utils::calculate;\nfn test() { return calculate(); }"),
                    ("math.cm", "fn calculate() { return; }"),
                    ("utils.cm", "fn calculate() { return; }"),
                ]
            ),
            // Duplicate import of the same item
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use math::add;\nuse math::add;\nfn test() { add(1, 2); return; }"),
                    ("math.cm", "fn add(a: felt, b: felt) { return; }"),
                ]
            ),
        ]
    }
}

#[test]
fn test_cross_module_type_checking() {
    assert_semantic_parameterized! {
        err: [
            // Wrong argument count
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use utils::add;\nfn test() -> felt { return add(1); }"),
                    ("utils.cm", "fn add(a: felt, b: felt) -> felt { return a + b; }"),
                ]
            ),
            // Wrong argument type
            multi_file(
                "main.cm",
                &[
                    ("main.cm", "use utils::process;\nfn test() { let x: felt = 5; return process(x); }"),
                    ("utils.cm", "struct Point { x: felt, y: felt }\nfn process(p: Point) { }"),
                ]
            ),
        ]
    }
}
