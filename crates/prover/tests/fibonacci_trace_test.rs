use std::fs;

use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerOptions};

/// Compiles a Cairo-M file to a Program
pub fn compile_cairo_file(cairo_file: &str) -> Result<Program, String> {
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

    // Clone the Arc<Program> to get an owned Program
    Ok((*output.program).clone())
}

pub mod fibonacci {
    pub mod trace_memory_generator {

        use cairo_m_prover::adapter::{import_from_runner_artifacts, import_from_runner_output};
        use cairo_m_runner::run_cairo_program;
        use stwo_prover::core::fields::m31::M31;
        use tempfile::TempDir;

        #[test]
        #[allow(clippy::cognitive_complexity)]
        fn test_import_fibonacci() {
            // Create a temporary directory for test fixtures
            let temp_dir = TempDir::new().expect("Failed to create temp directory");

            // Compile the fibonacci program
            let compiled = crate::compile_cairo_file("fibonacci.cm")
                .expect("Failed to compile Cairo-M program");

            // Run the program to generate trace and memory data
            let cairo_result =
                run_cairo_program(&compiled, "fib", &[M31::from(1000)], Default::default())
                    .expect("Failed to run Cairo-M program");

            // Create paths for temporary trace and memory files
            let trace_path = temp_dir.path().join("trace.bin");
            let memory_path = temp_dir.path().join("memory.bin");

            // Write the trace and memory data to temporary files
            cairo_result
                .vm
                .write_binary_trace(&trace_path)
                .expect("Failed to write binary trace");
            cairo_result
                .vm
                .write_binary_memory_trace(&memory_path)
                .expect("Failed to write binary memory trace");

            // Test importing from the generated files
            let from_files = import_from_runner_artifacts(&trace_path, &memory_path)
                .expect("Failed to import from vm output");
            let from_runner_output = import_from_runner_output(&cairo_result)
                .expect("Failed to import from runner output");

            // Compare the results
            assert_eq!(from_files, from_runner_output);
        }
    }
}
