use cairo_m_common::{CairoMValue, InputValue};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use proptest::prelude::*;

// Macro for asserting Cairo-M program results match expected values
macro_rules! assert_cairo_result {
    ($source_path:expr, $entrypoint:expr, $args:expr, $expected:expr) => {{
        // Compile the Cairo-M source
        let source = std::fs::read_to_string($source_path)
            .expect(&format!("Failed to read source file: {}", $source_path));

        let output = compile_cairo(source, "src/".to_string(), CompilerOptions::default())
            .expect(&format!("Failed to compile {}", $source_path));

        // Run the Cairo-M program
        let result = run_cairo_program(
            &output.program,
            $entrypoint,
            &$args,
            RunnerOptions::default(),
        )
        .expect(&format!(
            "Failed to run {} with entry point {}",
            $source_path, $entrypoint
        ));

        // Extract the return value
        assert!(
            !result.return_values.is_empty(),
            "Program {} with entry point {} returned no values",
            $source_path,
            $entrypoint
        );

        let cairo_result = match &result.return_values[0] {
            CairoMValue::Felt(value) => value.0 as u32,
            _ => panic!("Expected Felt return value from {}", $entrypoint),
        };

        // Compare with expected value (convert expected to u32)
        let expected_u32 = $expected as u32;
        assert_eq!(
            cairo_result, expected_u32,
            "Cairo-M program {} with entry point {} returned {} but expected {}",
            $source_path, $entrypoint, cairo_result, expected_u32
        );
    }};
}

// Rust reference implementation (iterative)
fn fibonacci_rust(n: u32) -> u32 {
    let mut current = 0;
    let mut next = 1;
    for _ in 0..n {
        let new_next = current + next;
        current = next;
        next = new_next;
    }
    current
}

proptest! {
    #[test]
    fn test_fibonacci_property(n in 0u32..30) {
        assert_cairo_result!("src/fibonacci.cm", "fibonacci", vec![InputValue::Number(n as i64)], fibonacci_rust(n));
    }
}
