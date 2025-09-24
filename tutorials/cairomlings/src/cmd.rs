use anyhow::{Context, Result, bail};
use cairo_m_common::Program;
use cairo_m_compiler::{compile_cairo, CompilerError, CompilerOptions};
use cairo_m_runner::{RunnerOptions, run_cairo_program};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use serde::Deserialize;
use std::fs;
use std::{
    io::{Read, pipe},
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::term::write_ansi;

/// Run a command with a description for a possible error and append the merged stdout and stderr.
/// The boolean in the returned `Result` is true if the command's exit status is success.
fn run_cmd(mut cmd: Command, description: &str, output: Option<&mut Vec<u8>>) -> Result<bool> {
    let spawn = |mut cmd: Command| {
        // NOTE: The closure drops `cmd` which prevents a pipe deadlock.
        cmd.stdin(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to run the command `{description}`"))
    };

    let mut handle = if let Some(output) = output {
        let (mut reader, writer) = pipe().with_context(|| {
            format!("Failed to create a pipe to run the command `{description}``")
        })?;

        let writer_clone = writer.try_clone().with_context(|| {
            format!("Failed to clone the pipe writer for the command `{description}`")
        })?;

        cmd.stdout(writer_clone).stderr(writer);
        let handle = spawn(cmd)?;

        reader
            .read_to_end(output)
            .with_context(|| format!("Failed to read the output of the command `{description}`"))?;

        output.push(b'\n');

        handle
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        spawn(cmd)?
    };

    handle
        .wait()
        .with_context(|| format!("Failed to wait on the command `{description}` to exit"))
        .map(|status| status.success())
}

// Parses parts of the output of `cargo metadata`.
#[derive(Deserialize)]
struct CargoMetadata {
    target_directory: PathBuf,
}

pub struct CmdRunner {
    target_dir: PathBuf,
}

impl CmdRunner {
    pub fn build() -> Result<Self> {
        // Get the target directory from Cargo.
        let metadata_output = Command::new("cargo")
            .arg("metadata")
            .arg("-q")
            .arg("--format-version")
            .arg("1")
            .arg("--no-deps")
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .context(CARGO_METADATA_ERR)?;

        if !metadata_output.status.success() {
            bail!(
                "The command `cargo metadata …` failed. Are you in the `CairoMlings/` directory?"
            );
        }

        let metadata: CargoMetadata = serde_json::de::from_slice(&metadata_output.stdout)
            .context(
                "Failed to read the field `target_directory` from the output of the command `cargo metadata …`",
            )?;

        Ok(Self {
            target_dir: metadata.target_directory,
        })
    }

    pub fn cairom_compile<'out>(
        &self,
        input_path: &str,
        output: Option<&'out mut Vec<u8>>,
    ) -> Result<Program> {
        let source_text = fs::read_to_string(input_path).context("Failed to read input file")?;
        let compiled = match compile_cairo(
            source_text.clone(),
            input_path.to_string(),
            CompilerOptions::default(),
        ) {
            Ok(compiled) => compiled,
            Err(e) => {
                match e {
                    CompilerError::ParseErrors(errors) | CompilerError::SemanticErrors(errors) => {
                        let mut error_str = String::new();
                        for error in errors {
                            error_str.push_str(&error.display_with_source(&source_text));
                        }
                        return Err(anyhow::anyhow!(error_str));
                    }
                    CompilerError::MirGenerationFailed | CompilerError::CodeGenerationFailed(_) => {
                        return Err(anyhow::anyhow!("Compilation failed: {:?}", e));
                    }
                }
            }
        };

        Ok(compiled.program.as_ref().clone())
    }

    pub fn cargo<'out>(
        &self,
        subcommand: &str,
        bin_name: &str,
        output: Option<&'out mut Vec<u8>>,
    ) -> CargoSubcommand<'out> {
        let mut cmd = Command::new("cargo");
        cmd.arg(subcommand).arg("-q").arg("--bin").arg(bin_name);

        // A hack to make `cargo run` work when developing CairoMlings.
        #[cfg(debug_assertions)]
        cmd.arg("--manifest-path")
            .arg("dev/Cargo.toml")
            .arg("--target-dir")
            .arg(&self.target_dir);

        if output.is_some() {
            cmd.arg("--color").arg("always");
        }

        CargoSubcommand { cmd, output }
    }

    /// The boolean in the returned `Result` is true if the command's exit status is success.
    pub fn run_debug_bin(&self, bin_name: &str, output: Option<&mut Vec<u8>>) -> Result<bool> {
        // 7 = "/debug/".len()
        let mut bin_path =
            PathBuf::with_capacity(self.target_dir.as_os_str().len() + 7 + bin_name.len());
        bin_path.push(&self.target_dir);
        bin_path.push("debug");
        bin_path.push(bin_name);

        run_cmd(Command::new(&bin_path), &bin_path.to_string_lossy(), output)
    }
}

pub struct CargoSubcommand<'out> {
    cmd: Command,
    pub output: Option<&'out mut Vec<u8>>,
}

impl CargoSubcommand<'_> {
    #[inline]
    pub fn args<'arg, I>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = &'arg str>,
    {
        self.cmd.args(args);
        self
    }

    /// The boolean in the returned `Result` is true if the command's exit status is success.
    #[inline]
    pub fn run(self, description: &str) -> Result<bool> {
        run_cmd(self.cmd, description, self.output)
    }
}

const CARGO_METADATA_ERR: &str = "Failed to run the command `cargo metadata …`
Is Cargo installed and available on PATH?
Try running `cargo --version` to diagnose the problem.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_cmd() {
        let mut cmd = Command::new("echo");
        cmd.arg("Hello");

        let mut output = Vec::with_capacity(8);
        run_cmd(cmd, "echo …", Some(&mut output)).unwrap();

        assert_eq!(output, b"Hello\n\n");
    }
}
