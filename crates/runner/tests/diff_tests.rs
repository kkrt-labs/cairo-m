use std::fs;

use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::run_cairo_program;
use stwo_prover::core::fields::m31::M31;

/// Represents a test case for diff-testing
struct DiffTest {
    name: &'static str,
    cairo_file: &'static str,
    entrypoint: &'static str,
    args: Vec<M31>,
    rust_fn: fn() -> u32,
    description: &'static str,
}

/// Macro to create a diff test with minimal boilerplate
macro_rules! diff_test {
    ($name:ident, $cairo_file:expr, $entrypoint:expr, $args:expr, $rust_fn:expr, $description:expr) => {
        #[test]
        fn $name() {
            let test = DiffTest {
                name: stringify!($name),
                cairo_file: $cairo_file,
                entrypoint: $entrypoint,
                args: $args,
                rust_fn: $rust_fn,
                description: $description,
            };
            run_diff_test(test);
        }
    };
    ($name:ident, $cairo_file:expr, $entrypoint:expr, $rust_fn:expr, $description:expr) => {
        #[test]
        fn $name() {
            let test = DiffTest {
                name: stringify!($name),
                cairo_file: $cairo_file,
                entrypoint: $entrypoint,
                args: vec![],
                rust_fn: $rust_fn,
                description: $description,
            };
            run_diff_test(test);
        }
    };
}

/// Compiles a Cairo-M file to a CompiledProgram
fn compile_cairo_file(cairo_file: &str) -> Result<Program, String> {
    let source_path = format!(
        "{}/tests/test_data/{}",
        env!("CARGO_MANIFEST_DIR"),
        cairo_file
    );

    // Read the source file
    let source_text = fs::read_to_string(&source_path)
        .map_err(|e| format!("Failed to read source file '{}': {}", source_path, e))?;

    // Compile using the library API
    let options = CompilerOptions { verbose: false };

    let output = compile_cairo(source_text, source_path, options)
        .map_err(|e| format!("Compilation failed: {}", e))?;

    // Clone the Arc<CompiledProgram> to get an owned CompiledProgram
    Ok((*output.program).clone())
}

/// Runs a diff test comparing Cairo-M and Rust implementations
fn run_diff_test(test: DiffTest) {
    // Compile Cairo-M program
    let program = compile_cairo_file(test.cairo_file).expect("Failed to compile Cairo-M program");

    // Run Cairo-M program using the library API
    println!(
        "Running Cairo-M program: {} with args: {:?}",
        test.entrypoint, test.args
    );
    let cairo_result = run_cairo_program(&program, test.entrypoint, &test.args, Default::default())
        .expect("Failed to run Cairo-M program");

    // Run Rust implementation
    let rust_result = (test.rust_fn)();

    assert!(
        cairo_result.return_values.len() == 1,
        "Expected exactly one return value, got {}",
        cairo_result.return_values.len()
    );
    assert_eq!(
        cairo_result.return_values[0].0, rust_result,
        "Results differ! Cairo-M: {}, Rust: {} \n for test: {} \n {}",
        cairo_result.return_values[0].0, rust_result, test.name, test.description
    );
}

// Test implementations

// 1. Fibonacci (recursive)
fn rust_fib_recursive() -> u32 {
    fn fib(n: u32) -> u32 {
        if n == 0 {
            0
        } else if n == 1 {
            1
        } else {
            (fib(n - 1) + fib(n - 2)) % (1u32 << 31)
        }
    }
    fib(10)
}

diff_test!(
    test_fibonacci_recursive,
    "fibonacci.cm",
    "fib",
    vec![M31::from(10)],
    rust_fib_recursive,
    "Recursive Fibonacci computation for n=10"
);

// 1. Fibonacci (loop)
fn rust_fibonacci_loop() -> u32 {
    fn fib(n: u32) -> u32 {
        let mut a = 0;
        let mut b = 1;
        for _ in 0..n {
            let temp = a;
            a = b;
            b += temp;
        }
        a
    }
    fib(10)
}

diff_test!(
    test_fibonacci_loop,
    "fibonacci_loop.cm",
    "fibonacci_loop",
    vec![M31::from(10)],
    rust_fibonacci_loop,
    "Fibonacci loop computation for n=10"
);

// 2. Factorial (recursive)
fn rust_factorial() -> u32 {
    fn factorial(n: u32) -> u32 {
        if n == 0 {
            1
        } else {
            (n as u64 * factorial(n - 1) as u64 % (1u64 << 31)) as u32
        }
    }
    factorial(10)
}

diff_test!(
    test_factorial,
    "factorial.cm",
    "main",
    rust_factorial,
    "Factorial computation for n=10"
);

// 4. Power (recursive)
fn rust_power() -> u32 {
    fn power(base: u32, exp: u32) -> u32 {
        if exp == 0 {
            1
        } else {
            (base as u64 * power(base, exp - 1) as u64 % (1u64 << 31)) as u32
        }
    }
    power(3, 10)
}

diff_test!(
    test_power,
    "power.cm",
    "power",
    vec![M31::from(3), M31::from(10)],
    rust_power,
    "3^10 computation"
);

// 5. Sum of first N numbers
fn rust_sum_n() -> u32 {
    fn sum_n(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            n + sum_n(n - 1)
        }
    }
    sum_n(10)
}

diff_test!(
    test_sum_n,
    "sum_n.cm",
    "main",
    rust_sum_n,
    "Sum of first 10 natural numbers"
);

// 6. Ackermann function (small values)
fn rust_ackermann() -> u32 {
    fn ackermann(m: u32, n: u32) -> u32 {
        if m == 0 {
            n + 1
        } else if n == 0 {
            ackermann(m - 1, 1)
        } else {
            ackermann(m - 1, ackermann(m, n - 1))
        }
    }
    ackermann(2, 2)
}

diff_test!(
    test_ackermann,
    "ackermann.cm",
    "main",
    rust_ackermann,
    "Ackermann(2,2) computation"
);

// 7. Double factorial (n!!)
fn rust_double_factorial() -> u32 {
    fn double_factorial(n: u32) -> u32 {
        if n == 0 || n == 1 {
            1
        } else {
            (n as u64 * double_factorial(n - 2) as u64 % (1u64 << 31)) as u32
        }
    }
    double_factorial(9) // 9!! = 9*7*5*3*1
}

diff_test!(
    test_double_factorial,
    "double_factorial.cm",
    "main",
    rust_double_factorial,
    "Double factorial of 9"
);

// 8. Nested function calls
const fn rust_nested_calls() -> u32 {
    const fn add(a: u32, b: u32) -> u32 {
        a + b
    }
    const fn mul(a: u32, b: u32) -> u32 {
        (a as u64 * b as u64 % (1u64 << 31)) as u32
    }
    const fn compute(x: u32) -> u32 {
        add(mul(x, 3), mul(x, 5))
    }
    compute(7)
}

diff_test!(
    test_nested_calls,
    "nested_calls.cm",
    "main",
    rust_nested_calls,
    "Nested function calls: compute(7) = 7*3 + 7*5"
);

// 9. Triangular number (using recursion)
fn rust_triangular() -> u32 {
    fn triangular(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            n + triangular(n - 1)
        }
    }
    triangular(100)
}

diff_test!(
    test_triangular,
    "triangular.cm",
    "main",
    rust_triangular,
    "100th triangular number"
);

// 10. Mutual recursion (even/odd check)
fn rust_mutual_recursion() -> u32 {
    fn is_even(n: u32) -> u32 {
        if n == 0 {
            1
        } else {
            is_odd(n - 1)
        }
    }
    fn is_odd(n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            is_even(n - 1)
        }
    }
    is_even(42) * 100 + is_odd(42) // Should be 100 (even=1, odd=0)
}

diff_test!(
    test_mutual_recursion,
    "mutual_recursion.cm",
    "main",
    rust_mutual_recursion,
    "Mutual recursion: even/odd check for 42"
);

// Testing a combination of language constructs
const fn combination() -> u32 {
    let x = 3;
    let y = 13;
    let even_number = 16;
    let eq = x == y;
    let mut mut_val = 1;
    if eq {
        mut_val = mut_val + eq as u32 + 1;
    }
    mut_val = mut_val * mut_val;
    mut_val += even_number / 2;

    let eq2 = x == 3;

    let compound1 = (x != 0) || (((y == 3) as u32 + 2) != 0);
    let compound2 = ((eq as u32) != 2) && (3 != 0);

    mut_val + (eq2 as u32) + random_elements_foo() + 32 + (compound1 as u32) + (compound2 as u32)
}

const fn random_elements_foo() -> u32 {
    32
}

diff_test!(
    test_combination_operations,
    "combination.cm",
    "main",
    combination,
    "combination of language constructs"
);

// Loops

const fn loops_with_break_continue() -> u32 {
    let mut counter = 0;
    let mut i = 0;
    loop {
        counter += 1;
        if counter == 5 {
            continue;
        }
        let mut counter2 = 0;
        loop {
            if counter2 == 10 {
                break;
            }
            counter2 += 1;
            i += 2;
        }
        i += 1;
        if counter == 10 {
            break;
        }
    }
    i
}

diff_test!(
    test_loops_with_break_continue,
    "loops_with_break_continue.cm",
    "loops_with_break_continue",
    loops_with_break_continue,
    "Loops with break and continue"
);

// Nested while loops
const fn nested_while_loops() -> u32 {
    let mut i = 0;
    let mut j = 0;
    while i != 10 {
        while j != 10 {
            j += 1;
        }
        i += 1;
    }
    i
}

diff_test!(
    test_nested_while_loops,
    "nested_while_loops.cm",
    "nested_while_loops",
    nested_while_loops,
    "Nested while loops"
);
