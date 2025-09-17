use cairo_m_common::{CairoMValue, InputValue};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};

fn run_with_options(
    source: &str,
    entry: &str,
    args: &[InputValue],
    options: CompilerOptions,
) -> Vec<CairoMValue> {
    let output = compile_cairo(source.to_string(), "opt_equiv.cm".to_string(), options)
        .expect("Compilation failed");

    let run = run_cairo_program(&output.program, entry, args, RunnerOptions::default())
        .expect("Program run failed");
    run.return_values
}

#[test]
fn opt_equiv_points_array_mutation() {
    // Repro from the report: array of struct with a mutation, then sum reads.
    let src = r#"
    struct Point { x: u32, y: u32 }

    fn test_main() -> u32 {
        let points: [Point; 3] = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }, Point { x: 5, y: 6 }];
        points[0].x = 10;
        return points[0].x + points[1].y + points[2].x;
    }
    "#;

    let no_opt = run_with_options(src, "test_main", &[], CompilerOptions::no_opts());
    let std_opt = run_with_options(src, "test_main", &[], CompilerOptions::default());

    assert_eq!(no_opt, std_opt, "Return values differ between opt levels");

    // Also assert expected value for a sanity check: 10 + 4 + 5 = 19
    assert_eq!(no_opt.len(), 1);
    match &no_opt[0] {
        CairoMValue::U32(v) => assert_eq!(*v, 19),
        other => panic!("Expected U32 return, got {:?}", other),
    }
}

#[test]
fn opt_equiv_nested_updates_tuple_in_struct() {
    // Nested tuple update inside a struct, accessed via array indexing.
    let src = r#"
    struct S { t: (u32, u32) }
    struct O { s: S }

    fn test_main() -> u32 {
        let arr: [O; 1] = [O { s: S { t: (1, 2) } }];
        arr[0].s.t.1 = 13;
        return arr[0].s.t.1;
    }
    "#;

    let no_opt = run_with_options(src, "test_main", &[], CompilerOptions::no_opts());
    let std_opt = run_with_options(src, "test_main", &[], CompilerOptions::default());
    assert_eq!(no_opt, std_opt, "Return values differ between opt levels");
    assert_eq!(no_opt.len(), 1);
    match &no_opt[0] {
        CairoMValue::U32(v) => assert_eq!(*v, 13),
        other => panic!("Expected U32 return, got {:?}", other),
    }
}

#[test]
fn opt_equiv_deep_nested_struct_chain() {
    // Deep nested struct field mutation and readback.
    let src = r#"
    struct C { c: u32 }
    struct B { b: C }
    struct A { a: B }

    fn test_main() -> u32 {
        let arr: [A; 1] = [A { a: B { b: C { c: 0 } } }];
        arr[0].a.b.c = 42;
        return arr[0].a.b.c;
    }
    "#;

    let no_opt = run_with_options(src, "test_main", &[], CompilerOptions::no_opts());
    let std_opt = run_with_options(src, "test_main", &[], CompilerOptions::default());
    assert_eq!(no_opt, std_opt, "Return values differ between opt levels");
    assert_eq!(no_opt.len(), 1);
    match &no_opt[0] {
        CairoMValue::U32(v) => assert_eq!(*v, 42),
        other => panic!("Expected U32 return, got {:?}", other),
    }
}
