// Common test utilities for mdtest runners

use cairo_m_common::program::AbiSlot;
use cairo_m_common::CairoMSerialize;
use cairo_m_compiler::{compile_cairo, CompilerError, CompilerOptions};
use cairo_m_runner::run_cairo_program;
use cairo_m_test_utils::mdtest;
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::process::Command;
use stwo_prover::core::fields::m31::M31;

/// Lazily extract all tests once and store them in a static HashMap
static ALL_TESTS: Lazy<HashMap<String, mdtest::MdTest>> = Lazy::new(|| {
    mdtest::extract_all_tests()
        .expect("Failed to extract tests")
        .into_iter()
        .flat_map(|(_path, tests)| tests)
        .map(|test| (test.name.clone(), test))
        .collect()
});

/// Get a test by name from the pre-loaded test cache
pub fn get_test_by_name(name: &str) -> &mdtest::MdTest {
    ALL_TESTS
        .get(name)
        .unwrap_or_else(|| panic!("Test '{}' not found", name))
}

/// Run a differential test for a single mdtest case
pub fn run_mdtest_diff(test: &mdtest::MdTest) -> Result<(), String> {
    // Compile Cairo-M code
    let compiler_options = CompilerOptions::default();
    let safe_name = sanitize_test_name(&test.name);

    let compiled = compile_cairo(
        test.cairo_source.clone(),
        format!("{}.cm", safe_name),
        compiler_options,
    )
    .map_err(|e| match e {
        CompilerError::ParseErrors(errors) | CompilerError::SemanticErrors(errors) => {
            let mut error_str = String::new();
            for error in errors {
                error_str.push_str(&error.display_with_source(&test.cairo_source));
            }
            error_str
        }
        CompilerError::MirGenerationFailed | CompilerError::CodeGenerationFailed(_) => {
            format!("Compilation failed: {:?}", e)
        }
    })?;

    // Find the entry point function
    let entry_point = find_test_function(&test.cairo_source);

    // Get function signature
    let entrypoint_info = compiled
        .program
        .get_entrypoint(&entry_point)
        .ok_or_else(|| format!("Entrypoint '{}' not found", entry_point))?;

    // Generate deterministic test arguments
    let mut rng = StdRng::seed_from_u64(42);
    let args = generate_random_args(&entrypoint_info.params, &mut rng);

    // Configure runner
    let runner_options = test
        .config
        .as_ref()
        .map(|c| cairo_m_runner::RunnerOptions {
            max_steps: c.mdtest.max_steps,
        })
        .unwrap_or_default();

    // Execute Cairo-M program
    let cairo_result = run_cairo_program(&compiled.program, &entry_point, &args, runner_options)
        .map_err(|e| format!("Runtime error: {:?}", e))?;

    // Format output
    let cairo_output = format_output(&cairo_result.return_values, &entrypoint_info.returns);

    // Check if we expect a runtime error
    if let Some(expected_error) = &test.metadata.expected_error {
        return Err(format!(
            "Expected compilation error '{}', but compilation succeeded",
            expected_error
        ));
    }

    // Check expected output if specified
    if let Some(expected) = &test.metadata.expected_output {
        if cairo_output != *expected {
            return Err(format!(
                "Output mismatch! Expected: {}, Got: {}",
                expected, cairo_output
            ));
        }
        return Ok(());
    }

    // Run differential testing with Rust
    let converted_rust;
    let rust_source = if let Some(rust) = &test.rust_source {
        rust.as_str()
    } else {
        converted_rust = convert_cairo_to_rust(&test.cairo_source);
        &converted_rust
    };

    let rust_output = run_rust_differential(
        rust_source,
        &entry_point,
        &args,
        &entrypoint_info.params,
        &entrypoint_info.returns,
    )?;

    // Parse rust output true / false to 1 / 0
    let rust_output = rust_output.replace("true", "1").replace("false", "0");

    // The rust output is an i32 (if returning from a felt, can be a negative value) or a u32 (if returning a u32) that we want to convert to M31.
    let rust_m31 = match rust_output.parse::<i32>() {
        Ok(val_i32) => M31::from(val_i32),
        Err(_) => rust_output.parse::<u32>().map(M31::from).unwrap(),
    };
    // The cairo output is at most a u32.
    let cairo_m31 = match cairo_output.parse::<u32>() {
        Ok(val_u32) => M31::from(val_u32),
        Err(_) => cairo_output.parse::<i32>().map(M31::from).unwrap(),
    };

    if rust_m31 != cairo_m31 {
        return Err(format!(
            "Differential test failed! Cairo-M: {}, Rust: {}",
            cairo_output, rust_output
        ));
    }

    Ok(())
}

fn sanitize_test_name(name: &str) -> String {
    name.replace(" - ", "_").replace(" ", "_").replace("/", "_")
}

fn find_test_function(cairo_source: &str) -> String {
    // Convention: Look for a function named 'test_main' or 'main' that returns a value
    // Preference order: test_main > main > first function with return value

    let mut test_main_fn = None;
    let mut main_fn = None;
    let mut first_returning_fn = None;

    for line in cairo_source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") && trimmed.contains("->") {
            if let Some(name) = trimmed
                .strip_prefix("fn ")
                .and_then(|s| s.split('(').next())
            {
                let fn_name = name.trim();

                // Check for our preferred names
                if fn_name == "test_main" {
                    test_main_fn = Some(fn_name.to_string());
                    break; // test_main has highest priority
                } else if fn_name == "main" {
                    main_fn = Some(fn_name.to_string());
                } else if first_returning_fn.is_none() {
                    first_returning_fn = Some(fn_name.to_string());
                }
            }
        }
    }

    test_main_fn
        .or(main_fn)
        .or(first_returning_fn)
        .unwrap_or_else(|| "main".to_string())
}

fn generate_random_args(params: &[AbiSlot], rng: &mut StdRng) -> Vec<M31> {
    let mut args = Vec::new();

    for param in params {
        let value: u32 = rng.gen_range(0..2u32.pow(31) - 1);
        if param.slots == 1 {
            // Generate a random felt value
            M31::from(value).encode(&mut args);
        } else {
            // Generate a random u32 value
            value.encode(&mut args);
        }
    }

    args
}

fn format_output(values: &[M31], return_types: &[AbiSlot]) -> String {
    if values.is_empty() {
        return "void".to_string();
    }

    if return_types.len() == 1 && return_types[0].slots == 1 {
        // Single felt return
        values[0].0.to_string()
    } else if return_types.len() == 1 && return_types[0].slots == 2 {
        // Single u32 return
        let (value, _) = u32::decode(values, 0);
        value.to_string()
    } else {
        // Multiple returns or complex type
        format!("{:?}", values)
    }
}

fn convert_cairo_to_rust(cairo_source: &str) -> String {
    use regex::Regex;

    let mut result = cairo_source
        .replace("-> felt", "-> i32")
        .replace("felt", "i32");

    // Make all variables mutable by default using regex
    let re = Regex::new(r"\blet\s+([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap();
    result = re.replace_all(&result, "let mut $1").to_string();

    result
}

fn run_rust_differential(
    rust_source: &str,
    entry_point: &str,
    args: &[M31],
    params: &[AbiSlot],
    _return_types: &[AbiSlot],
) -> Result<String, String> {
    // Create wrapper that calls the function with the same arguments
    let rust_args = format_rust_args(args, params);
    let wrapped_code = format!(
        r#"
{}

fn main() {{
    let result = {}({});
    println!("{{}}", result);
}}
"#,
        rust_source, entry_point, rust_args
    );

    run_rust_code(&wrapped_code)
}

fn format_rust_args(args: &[M31], params: &[AbiSlot]) -> String {
    let mut formatted = Vec::new();
    let mut arg_idx = 0;

    for param in params {
        if param.slots == 1 {
            // Single slot - felt/i32
            if arg_idx < args.len() {
                formatted.push(args[arg_idx].0.to_string());
                arg_idx += 1;
            }
        } else if param.slots == 2 {
            // Two slots - u32
            if arg_idx + 1 < args.len() {
                let (value, _) = u32::decode(args, arg_idx);
                formatted.push(value.to_string());
                arg_idx += 2;
            }
        }
    }

    formatted.join(", ")
}

fn run_rust_code(rust_source: &str) -> Result<String, String> {
    // Create a temporary directory
    let temp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let rust_file = temp_dir.path().join("test.rs");
    std::fs::write(&rust_file, rust_source)
        .map_err(|e| format!("Failed to write Rust file: {}", e))?;

    // Compile the Rust code
    let output = Command::new("rustc")
        .arg(&rust_file)
        .arg("-o")
        .arg(temp_dir.path().join("test"))
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Rust compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Run the compiled binary
    let output = Command::new(temp_dir.path().join("test"))
        .output()
        .map_err(|e| format!("Failed to run Rust binary: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Rust execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
