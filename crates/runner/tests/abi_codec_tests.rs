use cairo_m_common::{CairoMValue, InputValue};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};

/// Tests for the ABI encoding and decoding of values when calling entrypoints.
/// These tests verify the proper handling of:
/// - Fixed-size arrays (call ABI: passed as pointers, materialized inline)
/// - Nested structures with arrays
/// - Complex types through the VM call interface

#[test]
fn test_fixed_size_array_encoding_decoding() {
    let source = r#"
        fn sum_array(arr: [felt; 3]) -> felt {
            return arr[0] + arr[1] + arr[2];
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    // Create array input
    let args = vec![InputValue::List(vec![
        InputValue::Number(10),
        InputValue::Number(20),
        InputValue::Number(30),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "sum_array",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    // Verify the sum is correct
    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 60, "Expected sum of 60"),
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_nested_arrays_in_structs() {
    let source = r#"
        struct Data {
            values: [felt; 2],
            flag: bool,
        }

        fn process_data(data: Data) -> felt {
            if data.flag {
                return data.values[0] + data.values[1];
            } else {
                return data.values[0] - data.values[1];
            }
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    // Create struct with array field
    let args = vec![InputValue::Struct(vec![
        InputValue::List(vec![InputValue::Number(100), InputValue::Number(25)]),
        InputValue::Bool(true),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "process_data",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => {
            assert_eq!(v.0, 125, "Expected sum of 125");
        }
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_array_of_u32() {
    let source = r#"
        fn sum_u32_array(arr: [u32; 4]) -> u32 {
            return arr[0] + arr[1] + arr[2] + arr[3];
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    // Create array of u32 values
    let args = vec![InputValue::List(vec![
        InputValue::Number(1000),
        InputValue::Number(2000),
        InputValue::Number(3000),
        InputValue::Number(4000),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "sum_u32_array",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::U32(v) => assert_eq!(*v, 10000, "Expected sum of 10000"),
        _ => panic!("Expected U32 return value"),
    }
}

#[test]
fn test_tuple_with_array() {
    let source = r#"
        fn process_tuple(data: ([felt; 2], bool)) -> felt {
            let (arr, flag) = data;
            if flag {
                return arr[0] * arr[1];
            } else {
                return arr[0] + arr[1];
            }
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    // Create tuple with array and bool
    let args = vec![InputValue::List(vec![
        InputValue::List(vec![InputValue::Number(7), InputValue::Number(8)]),
        InputValue::Bool(true),
    ])];

    let result = run_cairo_program(
        &compiled.program,
        "process_tuple",
        &args,
        RunnerOptions::default(),
    );

    match result {
        Ok(output) => match &output.return_values[0] {
            CairoMValue::Felt(v) => {
                assert_eq!(v.0, 56, "Expected product of 56");
            }
            _ => panic!("Expected Felt return value"),
        },
        Err(e) => {
            panic!("Failed to run program: {:?}", e);
        }
    }
}

#[test]
fn test_return_array() {
    let source = r#"
        fn create_array() -> [felt; 3] {
            return [10, 20, 30];
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![];
    let output = run_cairo_program(
        &compiled.program,
        "create_array",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Array(arr) => {
            assert_eq!(arr.len(), 3, "Expected array of length 3");
            match &arr[0] {
                CairoMValue::Felt(v) => assert_eq!(v.0, 10),
                _ => panic!("Expected Felt in array"),
            }
            match &arr[1] {
                CairoMValue::Felt(v) => assert_eq!(v.0, 20),
                _ => panic!("Expected Felt in array"),
            }
            match &arr[2] {
                CairoMValue::Felt(v) => assert_eq!(v.0, 30),
                _ => panic!("Expected Felt in array"),
            }
        }
        _ => panic!("Expected Array return value"),
    }
}

#[test]
fn test_complex_nested_structure() {
    let source = r#"
        struct Inner {
            data: [felt; 2],
        }

        struct Outer {
            inner: Inner,
            count: felt,
        }

        fn process_nested(outer: Outer) -> felt {
            return outer.inner.data[0] + outer.inner.data[1] + outer.count;
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    // Create nested structure with arrays
    let args = vec![InputValue::Struct(vec![
        InputValue::Struct(vec![InputValue::List(vec![
            InputValue::Number(50),
            InputValue::Number(75),
        ])]),
        InputValue::Number(25),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "process_nested",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => {
            assert_eq!(v.0, 150, "Expected sum of 150");
        }
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_multiple_array_parameters() {
    let source = r#"
        fn combine_arrays(a: [felt; 2], b: [felt; 2]) -> felt {
            return a[0] + a[1] + b[0] + b[1];
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![
        InputValue::List(vec![InputValue::Number(1), InputValue::Number(2)]),
        InputValue::List(vec![InputValue::Number(3), InputValue::Number(4)]),
    ];

    let output = run_cairo_program(
        &compiled.program,
        "combine_arrays",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 10, "Expected sum of 10"),
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_array_of_bools() {
    let source = r#"
        fn count_true(flags: [bool; 5]) -> felt {
            let count = 0;
            if flags[0] { count = count + 1; }
            if flags[1] { count = count + 1; }
            if flags[2] { count = count + 1; }
            if flags[3] { count = count + 1; }
            if flags[4] { count = count + 1; }
            return count;
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![InputValue::List(vec![
        InputValue::Bool(true),
        InputValue::Bool(false),
        InputValue::Bool(true),
        InputValue::Bool(true),
        InputValue::Bool(false),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "count_true",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 3, "Expected count of 3"),
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_return_struct_with_array() {
    let source = r#"
        struct Result {
            values: [felt; 2],
            sum: felt,
        }

        fn compute_result(a: felt, b: felt) -> Result {
            return Result {
                values: [a, b],
                sum: a + b,
            };
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![InputValue::Number(15), InputValue::Number(25)];
    let output = run_cairo_program(
        &compiled.program,
        "compute_result",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Struct(fields) => {
            assert_eq!(fields.len(), 2, "Expected 2 fields");
            assert_eq!(fields[0].0, "values");
            match &fields[0].1 {
                CairoMValue::Array(arr) => {
                    assert_eq!(arr.len(), 2);
                    match &arr[0] {
                        CairoMValue::Felt(v) => assert_eq!(v.0, 15),
                        _ => panic!("Expected Felt in array"),
                    }
                    match &arr[1] {
                        CairoMValue::Felt(v) => assert_eq!(v.0, 25),
                        _ => panic!("Expected Felt in array"),
                    }
                }
                _ => panic!("Expected Array field"),
            }
            assert_eq!(fields[1].0, "sum");
            match &fields[1].1 {
                CairoMValue::Felt(v) => assert_eq!(v.0, 40),
                _ => panic!("Expected Felt field"),
            }
        }
        _ => panic!("Expected Struct return value"),
    }
}

#[test]
fn test_empty_array() {
    let source = r#"
        fn process_empty(arr: [felt; 0]) -> felt {
            return 42;
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![InputValue::List(vec![])];
    let output = run_cairo_program(
        &compiled.program,
        "process_empty",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 42, "Expected 42"),
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_array_of_tuples() {
    let source = r#"
        fn sum_pairs(pairs: [(felt, felt); 2]) -> felt {
            let (a, b) = pairs[0];
            let (c, d) = pairs[1];
            return a + b + c + d;
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![InputValue::List(vec![
        InputValue::List(vec![InputValue::Number(1), InputValue::Number(2)]),
        InputValue::List(vec![InputValue::Number(3), InputValue::Number(4)]),
    ])];

    let output = run_cairo_program(
        &compiled.program,
        "sum_pairs",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 10, "Expected sum of 10"),
        _ => panic!("Expected Felt return value"),
    }
}

#[test]
fn test_large_array() {
    let source = r#"
        fn sum_large(arr: [felt; 10]) -> felt {
            return arr[0] + arr[1] + arr[2] + arr[3] + arr[4] +
            arr[5] + arr[6] + arr[7] + arr[8] + arr[9];
        }
    "#;

    let compiled = compile_cairo(
        source.to_string(),
        "test.cm".to_string(),
        CompilerOptions::default(),
    )
    .expect("Failed to compile");

    let args = vec![InputValue::List((1..=10).map(InputValue::Number).collect())];

    let output = run_cairo_program(
        &compiled.program,
        "sum_large",
        &args,
        RunnerOptions::default(),
    )
    .expect("Failed to run program");

    match &output.return_values[0] {
        CairoMValue::Felt(v) => assert_eq!(v.0, 55, "Expected sum of 55"),
        _ => panic!("Expected Felt return value"),
    }
}
