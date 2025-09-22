use anyhow::{bail, Context, Result};
use crossterm::{
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    QueueableCommand,
};
use serde::Deserialize;
use std::{
    env::set_current_dir,
    fs::{self, create_dir},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    embedded::EMBEDDED_FILES, exercise::RunnableExercise, info_file::InfoFile,
    term::press_enter_prompt,
};

#[derive(Deserialize)]
struct CargoLocateProject {
    root: PathBuf,
}

pub fn init() -> Result<()> {
    let CairoMlings_dir = Path::new("CairoMlings");
    if CairoMlings_dir.exists() {
        bail!(CairoMlings_DIR_ALREADY_EXISTS_ERR);
    }

    let locate_project_output = Command::new("cargo")
        .arg("locate-project")
        .arg("-q")
        .arg("--workspace")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .context(
            "Failed to run the command `cargo locate-project …`\n\
             Did you already install Rust?\n\
             Try running `cargo --version` to diagnose the problem.",
        )?;

    if !Command::new("cargo")
        .arg("clippy")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to run the command `cargo clippy --version`")?
        .success()
    {
        bail!(
            "Clippy, the official Rust linter, is missing.\n\
             Please install it first before initializing CairoMlings."
        )
    }

    let mut stdout = io::stdout().lock();
    let mut init_git = true;

    if locate_project_output.status.success() {
        if Path::new("exercises").exists() && Path::new("solutions").exists() {
            bail!(IN_INITIALIZED_DIR_ERR);
        }

        let workspace_manifest =
            serde_json::de::from_slice::<CargoLocateProject>(&locate_project_output.stdout)
                .context(
                    "Failed to read the field `root` from the output of `cargo locate-project …`",
                )?
                .root;

        let workspace_manifest_content = fs::read_to_string(&workspace_manifest)
            .with_context(|| format!("Failed to read the file {}", workspace_manifest.display()))?;
        if !workspace_manifest_content.contains("[workspace]\n")
            && !workspace_manifest_content.contains("workspace.")
        {
            bail!(
                "The current directory is already part of a Cargo project.\n\
                 Please initialize CairoMlings in a different directory"
            );
        }

        stdout.write_all(b"This command will create the directory `CairoMlings/` as a member of this Cargo workspace.\n\
                           Press ENTER to continue ")?;
        press_enter_prompt(&mut stdout)?;

        // Make sure "CairoMlings" is added to `workspace.members` by making
        // Cargo initialize a new project.
        let status = Command::new("cargo")
            .arg("new")
            .arg("-q")
            .arg("--vcs")
            .arg("none")
            .arg("CairoMlings")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()?;
        if !status.success() {
            bail!(
                "Failed to initialize a new Cargo workspace member.\n\
                 Please initialize CairoMlings in a different directory"
            );
        }

        stdout.write_all(b"The directory `CairoMlings` has been added to `workspace.members` in the `Cargo.toml` file of this Cargo workspace.\n")?;
        fs::remove_dir_all("CairoMlings")
            .context("Failed to remove the temporary directory `CairoMlings/`")?;
        init_git = false;
    } else {
        stdout.write_all(b"This command will create the directory `CairoMlings/` which will contain the exercises.\n\
                           Press ENTER to continue ")?;
        press_enter_prompt(&mut stdout)?;
    }

    create_dir(CairoMlings_dir).context("Failed to create the `CairoMlings/` directory")?;
    set_current_dir(CairoMlings_dir)
        .context("Failed to change the current directory to `CairoMlings/`")?;

    let info_file = InfoFile::parse()?;
    EMBEDDED_FILES
        .init_exercises_dir(&info_file.exercises)
        .context("Failed to initialize the `CairoMlings/exercises` directory")?;

    create_dir("solutions").context("Failed to create the `solutions/` directory")?;
    fs::write(
        "solutions/README.md",
        include_bytes!("../solutions/README.md"),
    )
    .context("Failed to create the file CairoMlings/solutions/README.md")?;
    for dir in EMBEDDED_FILES.exercise_dirs {
        let mut dir_path = String::with_capacity(10 + dir.name.len());
        dir_path.push_str("solutions/");
        dir_path.push_str(dir.name);
        create_dir(&dir_path)
            .with_context(|| format!("Failed to create the directory {dir_path}"))?;
    }
    for exercise_info in &info_file.exercises {
        let solution_path = exercise_info.sol_path();
        fs::write(&solution_path, INIT_SOLUTION_FILE)
            .with_context(|| format!("Failed to create the file {solution_path}"))?;
    }

    // let current_cargo_toml = include_str!("../dev-Cargo.toml");
    // // Skip the first line (comment).
    // let newline_ind = current_cargo_toml
    //     .as_bytes()
    //     .iter()
    //     .position(|c| *c == b'\n')
    //     .context("The embedded `Cargo.toml` is empty or contains only one line")?;
    // let current_cargo_toml = current_cargo_toml
    //     .get(newline_ind + 1..)
    //     .context("The embedded `Cargo.toml` contains only one line")?;
    // let updated_cargo_toml = updated_cargo_toml(&info_file.exercises, current_cargo_toml, b"")
    // .context("Failed to generate `Cargo.toml`")?;
    let updated_cargo_toml = r#"[package]
name = "exercises"
edition = "2024"
# Don't publish the exercises on crates.io!
publish = false

bin = "src/lib.rs"
"#;
    fs::write("Cargo.toml", updated_cargo_toml)
        .context("Failed to create the file `CairoMlings/Cargo.toml`")?;

    create_dir("src").context("Failed to create the `src/` directory")?;
    fs::write("src/lib.rs", r#"
    fn main() {
        println!("Hello, world!");
    }
    "#)
        .context("Failed to create the file `CairoMlings/src/lib.rs`")?;

    fs::write("rust-analyzer.toml", RUST_ANALYZER_TOML)
        .context("Failed to create the file `CairoMlings/rust-analyzer.toml`")?;

    fs::write(".gitignore", GITIGNORE)
        .context("Failed to create the file `CairoMlings/.gitignore`")?;

    create_dir(".vscode").context("Failed to create the directory `CairoMlings/.vscode`")?;
    fs::write(".vscode/extensions.json", VS_CODE_EXTENSIONS_JSON)
        .context("Failed to create the file `CairoMlings/.vscode/extensions.json`")?;

    if init_git {
        // Ignore any Git error because Git initialization is not required.
        let _ = Command::new("git")
            .arg("init")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    stdout.queue(SetForegroundColor(Color::Green))?;
    stdout.write_all("Initialization done ✓".as_bytes())?;
    stdout.queue(ResetColor)?;
    stdout.write_all(b"\n\n")?;

    stdout.queue(SetAttribute(Attribute::Bold))?;
    stdout.write_all(POST_INIT_MSG)?;
    stdout.queue(ResetColor)?;

    Ok(())
}

const INIT_SOLUTION_FILE: &[u8] = b"fn main() {
    // DON'T EDIT THIS SOLUTION FILE!
    // It will be automatically filled after you finish the exercise.
}
";

pub const RUST_ANALYZER_TOML: &[u8] = br#"check.command = "clippy"
check.extraArgs = ["--profile", "test"]
cargo.targetDir = true
"#;

const GITIGNORE: &[u8] = b"Cargo.lock
target/
.vscode/
";

pub const VS_CODE_EXTENSIONS_JSON: &[u8] = br#"{"recommendations":["rust-lang.rust-analyzer"]}"#;

const IN_INITIALIZED_DIR_ERR: &str = "It looks like CairoMlings is already initialized in this directory.

If you already initialized CairoMlings, run the command `CairoMlings` for instructions on getting started with the exercises.
Otherwise, please run `CairoMlings init` again in a different directory.";

const CairoMlings_DIR_ALREADY_EXISTS_ERR: &str =
    "A directory with the name `CairoMlings` already exists in the current directory.
You probably already initialized CairoMlings.
Run `cd CairoMlings`
Then run `CairoMlings` again";

const POST_INIT_MSG: &[u8] = b"Run `cd CairoMlings` to go into the generated directory.
Then run `CairoMlings` to get started.
";
