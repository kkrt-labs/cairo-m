use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "cargo-cairo-m",
    bin_name = "cargo-cairo-m",
    version,
    about = "Tool for creating and managing Cairo-M projects"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Cairo-M project
    Init {
        /// Name of the project to create
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => init_project(&name),
    }
}

fn init_project(name: &str) -> Result<()> {
    // Validate project name
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }

    // Create project directory
    let project_path = Path::new(name);
    if project_path.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    fs::create_dir(project_path)
        .with_context(|| format!("Failed to create project directory '{}'", name))?;

    // Create directory structure
    fs::create_dir(project_path.join("src")).context("Failed to create src directory")?;
    fs::create_dir(project_path.join("tests")).context("Failed to create tests directory")?;
    fs::create_dir(project_path.join(".cargo")).context("Failed to create .cargo directory")?;

    // Write template files
    write_cargo_toml(project_path, name)?;
    write_gitignore(project_path)?;
    write_rust_toolchain(project_path)?;
    write_cargo_config(project_path)?;
    write_readme(project_path, name)?;
    write_lib_rs(project_path)?;
    write_fibonacci_cm(project_path)?;
    write_integration_test(project_path)?;

    println!("✅ Created new Cairo-M project '{}'", name);
    println!("\nTo get started:");
    println!("  cd {}", name);
    println!("  cargo test");

    Ok(())
}

fn write_cargo_toml(project_path: &Path, name: &str) -> Result<()> {
    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]

[dev-dependencies]
cairo-m-common = {{ git = "https://github.com/kkrt-labs/cairo-m" }}
cairo-m-runner = {{ git = "https://github.com/kkrt-labs/cairo-m" }}
cairo-m-compiler = {{ git = "https://github.com/kkrt-labs/cairo-m" }}
anyhow = "1.0"
"#,
        name
    );

    fs::write(project_path.join("Cargo.toml"), content).context("Failed to write Cargo.toml")?;
    Ok(())
}

fn write_gitignore(project_path: &Path) -> Result<()> {
    let content = r#"/target
/Cargo.lock
**/*.rs.bk
*.pdb
.DS_Store
"#;

    fs::write(project_path.join(".gitignore"), content).context("Failed to write .gitignore")?;
    Ok(())
}

fn write_rust_toolchain(project_path: &Path) -> Result<()> {
    let content = r#"[toolchain]
channel = "nightly-2025-04-06"
"#;

    fs::write(project_path.join("rust-toolchain.toml"), content)
        .context("Failed to write rust-toolchain.toml")?;
    Ok(())
}

fn write_cargo_config(project_path: &Path) -> Result<()> {
    let content = r#"[target.'cfg(target_os = "macos")']
rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld", "-C", "target-cpu=native"]

[target.'cfg(not(target_os = "macos"))']
rustflags = ["-C", "target-cpu=native"]
"#;

    fs::write(project_path.join(".cargo/config.toml"), content)
        .context("Failed to write .cargo/config.toml")?;
    Ok(())
}

fn write_readme(project_path: &Path, name: &str) -> Result<()> {
    let content = format!(
        r#"# {}

A Cairo-M project with integrated Rust testing.

## Project Structure

- `src/` - Cairo-M source files
- `tests/` - Rust integration tests
- `Cargo.toml` - Rust project configuration

## Prerequisites

### macOS Users
You need to have LLVM installed:
```bash
brew install llvm
```

## Common Commands

### Run all tests
```bash
cargo test
```

### Run a specific test
```bash
cargo test test_fibonacci
```

### Show test output
```bash
cargo test -- --nocapture
```

Note: The required RUSTFLAGS are automatically configured in `.cargo/config.toml`

## Adding New Cairo-M Files

1. Create a new `.cm` file in the `src/` directory
2. Write a corresponding test in `tests/`
3. Use `run_cairo_program` to execute your Cairo-M code
4. Compare results with Rust reference implementations

## Resources

- [Cairo-M Documentation](https://github.com/kkrt-labs/cairo-m)
- [Cairo-M Language Reference](https://github.com/kkrt-labs/cairo-m/docs)
"#,
        name
    );

    fs::write(project_path.join("README.md"), content).context("Failed to write README.md")?;
    Ok(())
}

fn write_lib_rs(project_path: &Path) -> Result<()> {
    let content = "// This file is required by Cargo but can remain empty\n";

    fs::write(project_path.join("src/lib.rs"), content).context("Failed to write src/lib.rs")?;
    Ok(())
}

fn write_fibonacci_cm(project_path: &Path) -> Result<()> {
    let content = r#"fn fibonacci(n: felt) -> felt {
    if (n == 0) {
        return 0;
    }
    if (n == 1) {
        return 1;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
"#;

    fs::write(project_path.join("src/fibonacci.cm"), content)
        .context("Failed to write src/fibonacci.cm")?;
    Ok(())
}

fn write_integration_test(project_path: &Path) -> Result<()> {
    let content = r#"use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use cairo_m_common::{InputValue, CairoMValue};

// Rust reference implementation
fn fibonacci_rust(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    fibonacci_rust(n - 1) + fibonacci_rust(n - 2)
}

#[test]
fn test_fibonacci() -> anyhow::Result<()> {
    // Compile the Cairo-M source
    let source = std::fs::read_to_string("src/fibonacci.cm")?;
    let output = compile_cairo(
        source,
        "fibonacci.cm".to_string(),
        CompilerOptions::default()
    )?;

    // Test multiple values
    for n in 0..=10 {
        // Prepare arguments
        let args = vec![InputValue::Number(n as i64)];

        // Run the Cairo-M program
        let result = run_cairo_program(
            &output.program,
            "fibonacci",
            &args,
            RunnerOptions::default()
        )?;

        // Get the Cairo-M result
        let cairo_result = match &result.return_values[0] {
            CairoMValue::Felt(value) => value.0 as u32,
            _ => panic!("Expected Felt return value"),
        };

        // Compare with Rust implementation
        let rust_result = fibonacci_rust(n);
        assert_eq!(
            cairo_result, rust_result,
            "Mismatch for fibonacci({n}): Cairo-M returned {cairo_result}, Rust returned {rust_result}"
        );
    }

    Ok(())
}
"#;

    fs::write(project_path.join("tests/integration_test.rs"), content)
        .context("Failed to write tests/integration_test.rs")?;
    Ok(())
}
