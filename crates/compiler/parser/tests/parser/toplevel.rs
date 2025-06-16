use crate::{assert_parses_err, assert_parses_ok};

// ===================
// Functions
// ===================

#[test]
fn simple_function() {
    assert_parses_ok!("func add(a: felt, b: felt) -> felt { return a + b; }");
}

#[test]
fn function_no_params() {
    assert_parses_ok!("func get_constant() -> felt { return 42; }");
}

#[test]
fn function_no_return() {
    assert_parses_ok!("func print_hello() { let msg = hello; }");
}

#[test]
fn function_multiple_params() {
    assert_parses_ok!("func complex(a: felt, b: felt*, c: (felt, felt)) { }");
}

#[test]
fn many_parameters() {
    assert_parses_ok!("func complex_function(a: felt, b: felt*, c: (felt, felt), d: MyStruct, e: MyStruct*) -> (felt, felt) { return (a, b); }");
}

#[test]
fn trailing_comma_function_params() {
    assert_parses_ok!("func test(a: felt, b: felt,) { }");
}

#[test]
fn missing_function_name() {
    assert_parses_err!("func (a: felt) -> felt { }");
}

#[test]
fn invalid_parameter() {
    assert_parses_err!("func test(: felt) { }");
}

#[test]
fn missing_function_body() {
    assert_parses_err!("func test() -> felt");
}

// ===================
// Structs
// ===================

#[test]
fn simple_struct() {
    assert_parses_ok!("struct Point { x: felt, y: felt }");
}

#[test]
fn empty_struct() {
    assert_parses_ok!("struct Unit { }");
}

#[test]
fn struct_with_pointers() {
    assert_parses_ok!("struct Node { data: felt, next: Node* }");
}

#[test]
fn complex_struct() {
    assert_parses_ok!(
        r#"
        struct ComplexStruct {
            field1: felt,
            field2: felt*,
            field3: (felt, felt),
            field4: AnotherStruct,
            field5: AnotherStruct*
        }
    "#
    );
}

#[test]
fn missing_struct_name() {
    assert_parses_err!("struct { x: felt }");
}

#[test]
fn invalid_field_definition() {
    assert_parses_err!("struct Point { x, y: felt }");
}

// ===================
// Namespaces
// ===================

#[test]
fn simple_namespace() {
    assert_parses_ok!("namespace Math { const PI = 314; }");
}

#[test]
fn namespace_with_function() {
    assert_parses_ok!("namespace Utils { func helper() -> felt { return 1; } }");
}

#[test]
fn nested_namespace() {
    assert_parses_ok!("namespace Outer { namespace Inner { const VALUE = 42; } }");
}

// ===================
// Imports
// ===================

#[test]
fn simple_import() {
    assert_parses_ok!("from std.math import add");
}

#[test]
fn import_with_alias() {
    assert_parses_ok!("from std.math import add as plus");
}

#[test]
fn nested_path_import() {
    assert_parses_ok!("from very.deep.module.path import function");
}

#[test]
fn invalid_import_syntax() {
    assert_parses_err!("import std.math");
}

#[test]
fn empty_import_path() {
    assert_parses_err!("from import item");
}

// ===================
// Constants
// ===================

#[test]
fn toplevel_const() {
    assert_parses_ok!("const MAX_SIZE = 100;");
}

#[test]
fn const_with_expression() {
    assert_parses_ok!("const COMPUTED = 2 * 3 + 1;");
}

// ===================
// Invalid Top-Level Items
// ===================

#[test]
fn invalid_toplevel_let() {
    assert_parses_err!("let x = 5;");
}

#[test]
fn invalid_toplevel_local() {
    assert_parses_err!("local x: felt = 42;");
}

#[test]
fn invalid_toplevel_assignment() {
    assert_parses_err!("x = 10;");
}

#[test]
fn invalid_toplevel_expression() {
    assert_parses_err!("42;");
}

#[test]
fn invalid_toplevel_return() {
    assert_parses_err!("return 5;");
}

#[test]
fn invalid_toplevel_if() {
    assert_parses_err!("if (true) { x = 1; }");
}

#[test]
fn invalid_toplevel_block() {
    assert_parses_err!("{ let x = 1; }");
}

// ===================
// Integration Tests
// ===================

#[test]
fn complete_program() {
    assert_parses_ok!(
        r#"
        struct Vector {
            x: felt,
            y: felt
        }

        namespace MathUtils {
            func magnitude(v: Vector) -> felt {
                return (v.x * v.x + v.y * v.y);
            }

            func rfib(n: felt) -> felt {
                if (n == 0) {
                    return 0;
                }
                if (n == 1) {
                    return 1;
                }
                return rfib(n - 1) + rfib(n - 2);
            }
        }

        const TOP_LEVEL_CONST = 100;
    "#
    );
}

#[test]
fn imports_and_functions() {
    assert_parses_ok!(
        r#"
        from std.math import sqrt
        from std.io import print as output

        struct Point {
            x: felt,
            y: felt
        }

        func distance(p1: Point, p2: Point) -> felt {
            local dx: felt = p1.x - p2.x;
            local dy: felt = p1.y - p2.y;
            return sqrt(dx * dx + dy * dy);
        }
    "#
    );
}

// ===================
// Edge Cases
// ===================

#[test]
fn empty_program() {
    assert_parses_ok!("");
}

#[test]
fn whitespace_only() {
    assert_parses_ok!("   \n\t   \n  ");
}

// ===================
// Error Recovery
// ===================

#[test]
fn multiple_syntax_errors() {
    assert_parses_err!(
        r#"
        func bad1( { }
        func good() { return 1; }
        struct bad2 x: felt }
        struct Good { x: felt }
    "#
    );
}

#[test]
fn mixed_valid_invalid() {
    assert_parses_err!(
        r#"
        const GOOD = 1;
        let bad = 42;
        const ALSO_GOOD = 2;
    "#
    );
}
