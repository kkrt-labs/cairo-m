use std::fs;

use cairo_m_compiler::{compile_cairo, CompiledProgram, CompilerOptions};

/// Compiles a Cairo-M file to a CompiledProgram
pub fn compile_cairo_file(cairo_file: &str) -> Result<CompiledProgram, String> {
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

pub mod fibonacci {
    pub mod trace_memory_generator {
        use cairo_m_runner::run_cairo_program;

        #[test]
        fn dump_trace_memory_fibonacci() {
            let compiled = crate::compile_cairo_file("fibonacci/fibonacci.cm")
                .expect("Failed to compile Cairo-M program");

            let cairo_result = run_cairo_program(&compiled, "main", Default::default())
                .expect("Failed to run Cairo-M program");

            cairo_result
                .vm
                .write_binary_memory_trace("tests/test_data/fibonacci/memory.bin")
                .expect("Failed to write binary memory trace");
            cairo_result
                .vm
                .write_binary_trace("tests/test_data/fibonacci/trace.bin")
                .expect("Failed to write binary trace");
        }
    }
}
