// Common test utilities for mdtest runners

use cairo_m_common::program::{AbiSlot, AbiType};
use cairo_m_common::{CairoMValue, InputValue};
use cairo_m_compiler::{compile_cairo, CompilerError, CompilerOptions};
use cairo_m_runner::run_cairo_program;
use cairo_m_test_utils::mdtest;
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::path::PathBuf;
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

    let compiled = match compile_cairo(
        test.cairo_source.clone(),
        format!("{}.cm", safe_name),
        compiler_options,
    ) {
        Ok(compiled) => compiled,
        Err(e) => {
            if let Some(_expected_error) = &test.metadata.expected_error {
                if e.to_string().contains("compilation") {
                    return Ok(());
                }
                return Ok(());
            }
            match e {
                CompilerError::ParseErrors(errors) | CompilerError::SemanticErrors(errors) => {
                    let mut error_str = String::new();
                    for error in errors {
                        error_str.push_str(&error.display_with_source(&test.cairo_source));
                    }
                    return Err(error_str);
                }
                CompilerError::MirGenerationFailed | CompilerError::CodeGenerationFailed(_) => {
                    return Err(format!("Compilation failed: {:?}", e));
                }
            }
        }
    };

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
    let cairo_output_info =
        match run_cairo_program(&compiled.program, &entry_point, &args, runner_options) {
            Ok(output) => output,
            Err(e) => {
                if let Some(expected_error) = &test.metadata.expected_error {
                    if format!("{:?}", e).contains(expected_error) {
                        return Ok(());
                    } else {
                        return Err(format!(
                            "Expected error to contain: {:?}, got: {:?}",
                            expected_error, e
                        ));
                    }
                }
                return Err(format!("Runtime error: {:?}", e));
            }
        };

    // Format output
    let cairo_output = format_output(&cairo_output_info.return_values, &entrypoint_info.returns);

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
    let (rust_source, rust_entry_point) = if let Some(rust) = &test.rust_source {
        (rust.to_string(), entry_point.clone())
    } else {
        let converted_rust = convert_cairo_to_rust(&test.cairo_source);
        let converted_entrypoint = convert_cairo_to_rust(&entry_point);
        (converted_rust, converted_entrypoint)
    };

    let rust_output = run_rust_differential(
        &rust_source,
        &rust_entry_point,
        &args,
        &entrypoint_info.params,
        &entrypoint_info.returns,
    )?;

    // Parse rust output true / false to 1 / 0
    let rust_output = rust_output.replace("true", "1").replace("false", "0");
    if rust_output == "[]" {
        assert_eq!(rust_output, cairo_output);
        return Ok(());
    }

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
    name.replace(" - ", "_")
        .replace(" ", "_")
        .replace("/", "_")
        .replace("`", "")
        .replace("'", "")
}

fn find_test_function(cairo_source: &str) -> String {
    // Convention: Look for a function named 'test_main' or 'main' that returns a value
    // Preference order: test_main > main > first function with return value

    let mut test_main_fn = None;
    let mut main_fn = None;
    let mut first_returning_fn = None;

    for line in cairo_source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") {
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
        .expect("No function found")
}

fn generate_random_args(params: &[AbiSlot], rng: &mut StdRng) -> Vec<InputValue> {
    let mut args = Vec::new();

    for param in params {
        args.push(generate_random_value(&param.ty, rng, 0));
    }

    args
}

fn generate_random_value(ty: &AbiType, rng: &mut StdRng, depth: u32) -> InputValue {
    // Limit recursion depth to avoid stack overflow
    if depth > 3 {
        panic!("MDTest runner: Recursion depth too high");
    }

    match ty {
        AbiType::Felt => {
            // Random positive value in a safe range
            let value: u32 = rng.gen_range(0..(1u32 << 31) - 1);
            InputValue::Number(value as i64)
        }
        AbiType::U32 => {
            // Random u32 value
            let value: u32 = rng.gen();
            InputValue::Number(value as i64)
        }
        AbiType::Bool => {
            let b: bool = rng.gen::<bool>();
            InputValue::Bool(b)
        }
        AbiType::Unit => InputValue::Unit,
        AbiType::Tuple(types) => {
            let values: Vec<InputValue> = types
                .iter()
                .map(|t| generate_random_value(t, rng, depth + 1))
                .collect();
            InputValue::List(values)
        }
        AbiType::Struct { fields, .. } => {
            let values: Vec<InputValue> = fields
                .iter()
                .map(|(_, t)| generate_random_value(t, rng, depth + 1))
                .collect();
            InputValue::Struct(values)
        }
        AbiType::FixedSizeArray { element, size } => {
            let values: Vec<InputValue> = (0..*size)
                .map(|_| generate_random_value(element, rng, depth + 1))
                .collect();
            InputValue::List(values)
        }
    }
}

fn format_output(values: &[CairoMValue], _return_types: &[AbiSlot]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }

    if values.len() == 1 {
        match &values[0] {
            CairoMValue::U32(v) => v.to_string(),
            CairoMValue::Felt(v) => v.0.to_string(),
            CairoMValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            CairoMValue::Pointer(p) => p.0.to_string(),
            _ => format!("{:?}", values[0]),
        }
    } else {
        // Multiple returns or complex type
        format!("{:?}", values)
    }
}

fn convert_cairo_to_rust(cairo_source: &str) -> String {
    use regex::Regex;

    let mut result = cairo_source.replace("felt", "i64");

    // Make all variables mutable by default using regex
    let re = Regex::new(r"\blet\s+([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap();
    result = re.replace_all(&result, "let mut $1").to_string();

    // Make all array indexes `as usize`
    let re = Regex::new(r"(\w+)\[(\w+|\d+)\]").unwrap();
    result = re.replace_all(&result, "$1[$2 as usize]").to_string();

    // Add a #[derive(Copy, Clone)] to all structs
    let re = Regex::new(r"\bstruct\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\{").unwrap();
    result = re
        .replace_all(&result, "#[derive(Copy, Clone)]\nstruct $1 {")
        .to_string();

    result
}

fn run_rust_differential(
    rust_source: &str,
    entry_point: &str,
    args: &[InputValue],
    params: &[AbiSlot],
    _return_types: &[AbiSlot],
) -> Result<String, String> {
    // Create wrapper that calls the function with the same arguments
    let rust_args = format_rust_args(args, params);

    // Check if the rust source uses M31
    let uses_m31 = rust_source.contains("use stwo_prover::core::fields::m31::M31");

    let wrapped_code = if uses_m31 {
        // For M31 returns, we need to extract the numeric value
        format!(
            r#"
{}

fn main() {{
    let result = {}({});
    // M31 has a .0 field that contains the u32 value
    println!("{{}}", result.0);
}}
"#,
            rust_source, entry_point, rust_args
        )
    } else {
        // Standard wrapper for non-M31 returns
        format!(
            r#"
{}

fn main() {{
    let result = {}({});
    if format!("{{:#?}}", result) != "()" {{
        println!("{{:#?}}", result);
    }} else {{
        println!("[]");
    }}
}}
"#,
            rust_source, entry_point, rust_args
        )
    };

    run_rust_code(&wrapped_code)
}

fn format_rust_args(args: &[InputValue], params: &[AbiSlot]) -> String {
    let mut formatted = Vec::new();

    for (arg, param) in args.iter().zip(params.iter()) {
        formatted.push(format_rust_value(arg, &param.ty));
    }

    formatted.join(", ")
}

fn format_rust_value(value: &InputValue, ty: &AbiType) -> String {
    match (value, ty) {
        (InputValue::Number(n), AbiType::Bool) => {
            if *n != 0 { "true" } else { "false" }.to_string()
        }
        (InputValue::Number(n), _) => n.to_string(),
        (InputValue::Bool(b), _) => if *b { "true" } else { "false" }.to_string(),
        (InputValue::Unit, _) => "()".to_string(),
        (InputValue::List(values), AbiType::Tuple(types)) => {
            let formatted: Vec<String> = values
                .iter()
                .zip(types.iter())
                .map(|(v, t)| format_rust_value(v, t))
                .collect();
            format!("({})", formatted.join(", "))
        }
        (InputValue::Struct(values), AbiType::Struct { fields, name }) => {
            // For Rust, we need to format as a struct literal
            let field_values: Vec<String> = values
                .iter()
                .zip(fields.iter())
                .map(|(v, (fname, fty))| format!("{}: {}", fname, format_rust_value(v, fty)))
                .collect();
            format!("{} {{ {} }}", name, field_values.join(", "))
        }
        (InputValue::List(values), AbiType::FixedSizeArray { element, .. }) => {
            let formatted: Vec<String> = values
                .iter()
                .map(|v| format_rust_value(v, element))
                .collect();
            format!("[{}]", formatted.join(", "))
        }
        _ => panic!(
            "MDTest runner: Type/value mismatch in Rust formatter: {:?} / {:?}",
            value, ty
        ),
    }
}

/// Structure to hold crate dependency information
#[derive(Debug)]
struct CrateDependency {
    name: String,
    rlib_path: PathBuf,
}

/// Build workspace crates and extract their artifact paths using cargo JSON output
fn build_and_discover_crates() -> Result<Vec<CrateDependency>, String> {
    // Build the required crates with JSON output
    let output = Command::new("cargo")
        .args([
            "build",
            "-p",
            "stwo-prover",
            "-p",
            "cairo-m-common",
            "-p",
            "cairo-m-test-utils",
            "--message-format=json",
        ])
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut dependencies = Vec::new();

    // Parse JSON output to find artifact paths
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            // Check if this is a compiler-artifact message
            if json["reason"] == "compiler-artifact" {
                if let Some(target) = json.get("target") {
                    let name = target["name"].as_str().unwrap_or("");

                    // We're interested in our workspace crates
                    if name == "stwo_prover"
                        || name == "cairo_m_common"
                        || name == "cairo_m_test_utils"
                    {
                        if let Some(filenames) = json.get("filenames") {
                            // Find the rlib file in the filenames array
                            for filename in filenames.as_array().unwrap_or(&Vec::new()) {
                                if let Some(path_str) = filename.as_str() {
                                    if path_str.ends_with(".rlib") {
                                        // Use the actual path from the JSON
                                        let path = PathBuf::from(path_str);

                                        // If the file doesn't exist at this path, try the deps directory
                                        let final_path = if path.exists() {
                                            path
                                        } else {
                                            // Try to find it in target/debug/deps
                                            let deps_path = PathBuf::from(format!(
                                                "target/debug/deps/lib{}.rlib",
                                                name
                                            ));
                                            if deps_path.exists() {
                                                deps_path
                                            } else {
                                                // Fall back to the original path
                                                path
                                            }
                                        };

                                        dependencies.push(CrateDependency {
                                            name: name.to_string(),
                                            rlib_path: final_path,
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

fn run_rust_code(rust_source: &str) -> Result<String, String> {
    // Create a temporary directory
    let temp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let rust_file = temp_dir.path().join("test.rs");
    std::fs::write(&rust_file, rust_source)
        .map_err(|e| format!("Failed to write Rust file: {}", e))?;

    // Build dependencies and get their paths
    let dependencies = build_and_discover_crates()?;

    // Build rustc command with dependency information
    let mut rustc_cmd = Command::new("rustc");
    rustc_cmd
        .arg(&rust_file)
        .arg("--edition=2021")
        .arg("--crate-type=bin")
        .arg("-o")
        .arg(temp_dir.path().join("test"));

    // Add dependency directory and extern crates
    if !dependencies.is_empty() {
        // Add both the main directory and deps directory
        if let Some(first_dep) = dependencies.first() {
            if let Some(deps_dir) = first_dep.rlib_path.parent() {
                rustc_cmd
                    .arg("-L")
                    .arg(format!("dependency={}", deps_dir.display()));

                // Also add the deps subdirectory if it exists
                let deps_subdir = deps_dir.join("deps");
                if deps_subdir.exists() {
                    rustc_cmd
                        .arg("-L")
                        .arg(format!("dependency={}", deps_subdir.display()));
                }
            }
        }

        // Add each crate as an extern
        for dep in &dependencies {
            rustc_cmd
                .arg("--extern")
                .arg(format!("{}={}", dep.name, dep.rlib_path.display()));
        }
    }

    // Compile the Rust code
    let output = rustc_cmd
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Rust compilation failed for source:\n{}\nError: {}",
            rust_source,
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
