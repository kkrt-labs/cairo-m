use crate::{assert_parses_err, assert_parses_ok, assert_parses_parameterized};

#[test]
fn function_definitions_parameterized() {
    assert_parses_parameterized! {
        ok: [
            "fn add(a: felt, b: felt) -> felt { return a + b; }",
            "fn get_constant() -> felt { return 42; }",
            "fn print_hello() { let msg = hello; }",
            "fn complex(a: felt, b: felt*, c: (felt, felt)) { }",
            "fn complex_function(a: felt, b: felt*, c: (felt, felt), d: MyStruct, e: MyStruct*) -> (felt, felt) { return (a, b); }",
            "fn test(a: felt, b: felt,) { }",
        ],
        err: [
            "fn (a: felt) -> felt { }",
            "fn test(: felt) { }",
            "fn test() -> felt",
        ]
    }
}

#[test]
fn struct_definitions_parameterized() {
    assert_parses_parameterized! {
        ok: [
            "struct Point { x: felt, y: felt }",
            "struct Unit { }",
            "struct Node { data: felt, next: Node* }",
            r#"
        struct ComplexStruct {
            field1: felt,
            field2: felt*,
            field3: (felt, felt),
            field4: AnotherStruct,
            field5: AnotherStruct*
        }
    "#,
        ],
        err: [
            "struct { x: felt }",
            "struct Point { x, y: felt }",
        ]
    }
}

#[test]
fn namespace_definitions_parameterized() {
    assert_parses_parameterized! {
        ok: [
            "namespace Math { const PI = 314; }",
            "namespace Utils { fn helper() -> felt { return 1; } }",
            "namespace Outer { namespace Inner { const VALUE = 42; } }",
        ]
    }
}

#[test]
fn use_statements_parameterized() {
    assert_parses_parameterized! {
        ok: [
            "use std::math::add;",
            "use std::math::{add, sub};",
        ],
        err: [
            "use std::math::add",
            "use ;",
        ]
    }
}

#[test]
fn toplevel_const_parameterized() {
    assert_parses_parameterized! {
        ok: [
            "const MAX_SIZE = 100;",
            "const COMPUTED = 2 * 3 + 1;",
        ]
    }
}

#[test]
fn invalid_toplevel_parameterized() {
    assert_parses_parameterized! {
        err: [
            "let x = 5;",
            "x = 10;",
            "42;",
            "return 5;",
            "if (true) { x = 1; }",
            "{ let x = 1; }",
        ]
    }
}

#[test]
fn complete_program() {
    assert_parses_ok!(
        r#"
        struct Vector {
            x: felt,
            y: felt
        }

        namespace MathUtils {
            fn magnitude(v: Vector) -> felt {
                return (v.x * v.x + v.y * v.y);
            }

            fn rfib(n: felt) -> felt {
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
        use std::math::sqrt;
        use std::io::print;

        struct Point {
            x: felt,
            y: felt
        }

        fn distance(p1: Point, p2: Point) -> felt {
            let dx: felt = p1.x - p2.x;
            let dy: felt = p1.y - p2.y;
            return sqrt(dx * dx + dy * dy);
        }
    "#
    );
}

#[test]
fn empty_program() {
    assert_parses_ok!("");
}

#[test]
fn whitespace_only() {
    assert_parses_ok!("   \n\t   \n  ");
}

#[test]
fn multiple_syntax_errors() {
    assert_parses_err!(
        r#"
        fn bad1( { }
        fn good() { return 1; }
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

// ===================
// Loop Constructs
// ===================

#[test]
fn function_with_loops() {
    assert_parses_ok!(
        r#"
        fn test_loops() {
            loop {
                let x = 1;
                if (x == 1) {
                    break;
                }
            }

            let counter = 0;
            while (counter != 10) {
                counter = counter + 1;
            }

            for i in range {
                let squared = i * i;
            }
        }
    "#
    );
}
