use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

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

    // Support paths
    let project_path = Path::new(name);
    let project_name = project_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    if project_path.exists() {
        anyhow::bail!("Directory '{}' already exists", project_path.display());
    }

    // Create all parent directories if needed
    fs::create_dir_all(project_path).with_context(|| {
        format!(
            "Failed to create project directory '{}'",
            project_path.display()
        )
    })?;

    // Create directory structure
    fs::create_dir(project_path.join("src")).context("Failed to create src directory")?;
    fs::create_dir(project_path.join("tests")).context("Failed to create tests directory")?;
    fs::create_dir(project_path.join(".cargo")).context("Failed to create .cargo directory")?;

    // Write template files
    write_cargo_toml(project_path, project_name)?;
    write_cairom_toml(project_path, project_name)?;
    write_gitignore(project_path)?;
    write_rust_toolchain(project_path)?;
    write_cargo_config(project_path)?;
    write_readme(project_path, project_name)?;
    write_lib_rs(project_path)?;
    write_fibonacci_cm(project_path)?;
    write_integration_test(project_path)?;

    println!(
        "✅ Created new Cairo-M project '{}'",
        project_path.display()
    );
    println!("\nTo get started:");
    println!("  cd {}", project_path.display());
    println!("  cargo test");

    Ok(())
}

fn write_cairom_toml(project_path: &Path, name: &str) -> Result<()> {
    let template = include_str!("../templates/cairom.toml");
    let content = template.replace("{{name}}", name);
    fs::write(project_path.join("cairom.toml"), content).context("Failed to write cairom.toml")?;
    Ok(())
}

fn write_cargo_toml(project_path: &Path, name: &str) -> Result<()> {
    let template = include_str!("../templates/Cargo.toml");
    let content = template.replace("cairo-m-template", name);
    fs::write(project_path.join("Cargo.toml"), content).context("Failed to write Cargo.toml")?;
    Ok(())
}

fn write_gitignore(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/gitignore");
    fs::write(project_path.join(".gitignore"), content).context("Failed to write .gitignore")?;
    Ok(())
}

fn write_rust_toolchain(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/rust-toolchain.toml");
    fs::write(project_path.join("rust-toolchain.toml"), content)
        .context("Failed to write rust-toolchain.toml")?;
    Ok(())
}

fn write_cargo_config(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/cargo-config.toml");
    fs::write(project_path.join(".cargo/config.toml"), content)
        .context("Failed to write .cargo/config.toml")?;
    Ok(())
}

fn write_readme(project_path: &Path, name: &str) -> Result<()> {
    let template = include_str!("../templates/README.md");
    let content = template.replace("{{name}}", name);
    fs::write(project_path.join("README.md"), content).context("Failed to write README.md")?;
    Ok(())
}

fn write_lib_rs(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/lib.rs");
    fs::write(project_path.join("src/lib.rs"), content).context("Failed to write src/lib.rs")?;
    Ok(())
}

fn write_fibonacci_cm(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/fibonacci.cm");
    fs::write(project_path.join("src/fibonacci.cm"), content)
        .context("Failed to write src/fibonacci.cm")?;
    Ok(())
}

fn write_integration_test(project_path: &Path) -> Result<()> {
    let content = include_str!("../templates/integration_test.rs");
    fs::write(project_path.join("tests/integration_test.rs"), content)
        .context("Failed to write tests/integration_test.rs")?;
    Ok(())
}
