use anyhow::{Context, Result};
use crossterm::{
    QueueableCommand,
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
};
use std::io::{self, StdoutLock, Write};

use crate::{
    cmd::CmdRunner,
    term::{self, CountedWrite, file_path, terminal_file_link, write_ansi},
};

use crate::info_file::TestCase;
use cairo_m_common::{CairoMValue, InputValue, Program, parse_cli_arg};
use cairo_m_runner::{RunnerOptions, run_cairo_program};
use std::fs;

/// The initial capacity of the output buffer.
pub const OUTPUT_CAPACITY: usize = 1 << 14;

pub fn solution_link_line(
    stdout: &mut StdoutLock,
    solution_path: &str,
    emit_file_links: bool,
) -> io::Result<()> {
    stdout.queue(SetAttribute(Attribute::Bold))?;
    stdout.write_all(b"Solution")?;
    stdout.queue(ResetColor)?;
    stdout.write_all(b" for comparison: ")?;
    file_path(stdout, Color::Cyan, |writer| {
        if emit_file_links && let Some(canonical_path) = term::canonicalize(solution_path) {
            terminal_file_link(writer, solution_path, &canonical_path)
        } else {
            writer.stdout().write_all(solution_path.as_bytes())
        }
    })?;
    stdout.write_all(b"\n")
}

fn run_artifact(
    artifact_output_path: &str,
    output: Option<&mut Vec<u8>>,
    cmd_runner: &CmdRunner,
) -> Result<bool> {
    let file_content = fs::read_to_string(&artifact_output_path)
        .with_context(|| format!("Error reading file '{}'", artifact_output_path))?;

    let compiled_program: Program =
        serde_json::from_str(&file_content).context("Failed to parse compiled program")?;

    match std::panic::catch_unwind(|| {
        run_cairo_program(&compiled_program, "main", &[], RunnerOptions::default())
    }) {
        Ok(Ok(_output)) => Ok(true),
        Ok(Err(e)) => {
            if let Some(buf) = output {
                write_ansi(buf, SetForegroundColor(Color::Red));
                buf.extend_from_slice(format!("Execution failed: {}\n", e).as_bytes());
                write_ansi(buf, ResetColor);
            }
            Ok(false)
        }
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic occurred".to_string()
            };

            if let Some(buf) = output {
                write_ansi(buf, SetForegroundColor(Color::Red));
                buf.extend_from_slice(format!("Panic occurred: {}\n", panic_message).as_bytes());
                write_ansi(buf, ResetColor);
            }
            Ok(false)
        }
    }
}

// Run an exercise binary and append its output to the `output` buffer.
// Compilation must be done before calling this method.
fn run_bin(
    bin_name: &str,
    mut output: Option<&mut Vec<u8>>,
    cmd_runner: &CmdRunner,
) -> Result<bool> {
    if let Some(output) = output.as_deref_mut() {
        write_ansi(output, SetAttribute(Attribute::Underlined));
        output.extend_from_slice(b"Output");
        write_ansi(output, ResetColor);
        output.push(b'\n');
    }

    let success = cmd_runner.run_debug_bin(bin_name, output.as_deref_mut())?;

    if let Some(output) = output
        && !success
    {
        // This output is important to show the user that something went wrong.
        // Otherwise, calling something like `exit(1)` in an exercise without further output
        // leaves the user confused about why the exercise isn't done yet.
        write_ansi(output, SetAttribute(Attribute::Bold));
        write_ansi(output, SetForegroundColor(Color::Red));
        output.extend_from_slice(b"The exercise didn't run successfully (nonzero exit code)");
        write_ansi(output, ResetColor);
        output.push(b'\n');
    }

    Ok(success)
}

/// See `info_file::ExerciseInfo`
pub struct Exercise {
    pub dir: Option<&'static str>,
    pub name: &'static str,
    /// Path of the exercise file starting with the `exercises/` directory.
    pub path: &'static str,
    pub canonical_path: Option<String>,
    pub test: bool,
    pub test_cases: Vec<TestCase>,
    pub strict_clippy: bool,
    pub hint: &'static str,
    pub done: bool,
}

impl Exercise {
    pub fn terminal_file_link<'a>(
        &self,
        writer: &mut impl CountedWrite<'a>,
        emit_file_links: bool,
    ) -> io::Result<()> {
        file_path(writer, Color::Blue, |writer| {
            if emit_file_links && let Some(canonical_path) = self.canonical_path.as_deref() {
                terminal_file_link(writer, self.path, canonical_path)
            } else {
                writer.write_str(self.path)
            }
        })
    }
}

pub trait RunnableExercise {
    fn name(&self) -> &str;
    fn dir(&self) -> Option<&str>;
    fn strict_clippy(&self) -> bool;
    fn test(&self) -> bool;
    fn test_cases(&self) -> &[TestCase];

    // Compile, check and run the exercise or its solution (depending on `bin_name´).
    // The output is written to the `output` buffer after clearing it.
    fn run<const FORCE_STRICT_CLIPPY: bool>(
        &self,
        bin_name: &str,
        mut output: Option<&mut Vec<u8>>,
        cmd_runner: &CmdRunner,
    ) -> Result<bool> {
        if let Some(output) = output.as_deref_mut() {
            output.clear();
        }

        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| std::env::current_dir().unwrap().to_string_lossy().to_string());
        let exercise_path = format!(
            "{workspace_root}/exercises/{}/{}.cm",
            self.dir().unwrap_or(""),
            self.name()
        );
        let program = match cmd_runner.cairom_compile(&exercise_path, output.as_deref_mut()) {
            Ok(program) => program,
            Err(e) => {
                if let Some(buf) = output.as_deref_mut() {
                    write_ansi(buf, SetForegroundColor(Color::Red));
                    buf.extend_from_slice(format!("Compilation failed: {}\n", e).as_bytes());
                    write_ansi(buf, ResetColor);
                }
                return Ok(false);
            }
        };

        // Discard the compiler output because it will be shown again by Clippy.
        if let Some(output) = output.as_deref_mut() {
            output.clear();
        }

        // If tests are enabled, execute configured test-cases by calling `main` with inputs
        // and comparing returned values with expected outputs.
        if self.test() && !self.test_cases().is_empty() {
            // Load compiled program
            // Helper to map CairoMValue to an InputValue-like structure for comparison
            fn cairo_to_input_like(v: &CairoMValue) -> InputValue {
                match v {
                    CairoMValue::Felt(m) => InputValue::Number(m.0 as i64),
                    CairoMValue::Bool(b) => InputValue::Bool(*b),
                    CairoMValue::U32(u) => InputValue::Number(*u as i64),
                    CairoMValue::Pointer(m) => InputValue::Number(m.0 as i64),
                    CairoMValue::Tuple(elems) => {
                        InputValue::List(elems.iter().map(cairo_to_input_like).collect())
                    }
                    CairoMValue::Struct(fields) => {
                        // Compare positionally; ignore field names
                        InputValue::Struct(
                            fields.iter().map(|(_, v)| cairo_to_input_like(v)).collect(),
                        )
                    }
                    CairoMValue::Array(elems) => {
                        InputValue::List(elems.iter().map(cairo_to_input_like).collect())
                    }
                    CairoMValue::Unit => InputValue::Unit,
                }
            }

            // Execute all test cases
            let mut all_ok = true;
            if let Some(buf) = output.as_deref_mut() {
                write_ansi(buf, SetAttribute(Attribute::Underlined));
                buf.extend_from_slice(b"Tests");
                write_ansi(buf, ResetColor);
                buf.push(b'\n');
            }

            for (i, case) in self.test_cases().iter().enumerate() {
                // Parse inputs
                let mut args: Vec<InputValue> = Vec::with_capacity(case.inputs.len());
                let mut parse_err: Option<String> = None;
                for s in &case.inputs {
                    match parse_cli_arg(s) {
                        Ok(v) => args.push(v),
                        Err(e) => {
                            parse_err = Some(format!("Failed to parse input '{}': {}", s, e));
                            break;
                        }
                    }
                }
                if let Some(err) = parse_err {
                    all_ok = false;
                    if let Some(buf) = output.as_deref_mut() {
                        write_ansi(buf, SetForegroundColor(Color::Red));
                        buf.extend_from_slice(format!("Case {}: {}\n", i + 1, err).as_bytes());
                        write_ansi(buf, ResetColor);
                    }
                    continue;
                }

                let run = run_cairo_program(&program, "main", &args, RunnerOptions::default());
                match run {
                    Err(e) => {
                        all_ok = false;
                        if let Some(buf) = output.as_deref_mut() {
                            write_ansi(buf, SetForegroundColor(Color::Red));
                            buf.extend_from_slice(
                                format!("Case {}: execution failed: {}\n", i + 1, e).as_bytes(),
                            );
                            write_ansi(buf, ResetColor);
                        }
                    }
                    Ok(run_output) => {
                        let got: Vec<InputValue> = run_output
                            .return_values
                            .iter()
                            .map(cairo_to_input_like)
                            .collect();

                        // Parse expected outputs
                        let mut expected: Vec<InputValue> = Vec::with_capacity(case.outputs.len());
                        let mut exp_err: Option<String> = None;
                        for s in &case.outputs {
                            match parse_cli_arg(s) {
                                Ok(v) => expected.push(v),
                                Err(e) => {
                                    exp_err = Some(format!(
                                        "Failed to parse expected output '{}': {}",
                                        s, e
                                    ));
                                    break;
                                }
                            }
                        }
                        if let Some(err) = exp_err {
                            all_ok = false;
                            if let Some(buf) = output.as_deref_mut() {
                                write_ansi(buf, SetForegroundColor(Color::Red));
                                buf.extend_from_slice(
                                    format!("Case {}: {}\n", i + 1, err).as_bytes(),
                                );
                                write_ansi(buf, ResetColor);
                            }
                            continue;
                        }

                        let pass = got == expected;
                        if let Some(buf) = output.as_deref_mut() {
                            if pass {
                                write_ansi(buf, SetForegroundColor(Color::Green));
                                buf.extend_from_slice(format!("Case {}: ok\n", i + 1).as_bytes());
                                write_ansi(buf, ResetColor);
                            } else {
                                all_ok = false;
                                write_ansi(buf, SetForegroundColor(Color::Red));
                                buf.extend_from_slice(
                                    format!("Case {}: failed\n", i + 1).as_bytes(),
                                );
                                write_ansi(buf, ResetColor);
                                buf.extend_from_slice(
                                    format!(
                                        "  inputs:   {:?}\n  expected: {:?}\n  got:      {:?}\n",
                                        case.inputs, expected, got
                                    )
                                    .as_bytes(),
                                );
                            }
                        }
                    }
                }
            }

            return Ok(all_ok);
        }

        // let mut clippy_cmd = cmd_runner.cargo("clippy", bin_name, output.as_deref_mut());

        // // `--profile test` is required to also check code with `#[cfg(test)]`.
        // if FORCE_STRICT_CLIPPY || self.strict_clippy() {
        //     clippy_cmd.args(["--profile", "test", "--", "-D", "warnings"]);
        // } else {
        //     clippy_cmd.args(["--profile", "test"]);
        // }

        // let clippy_success = clippy_cmd.run("cargo clippy …")?;
        // let run_success = run_bin(bin_name, output, cmd_runner)?;
        // Ok(clippy_success && run_success)

        match run_cairo_program(&program, "main", &[], RunnerOptions::default()) {
            Ok(_) => Ok(true),
            Err(e) => {
                if let Some(buf) = output.as_deref_mut() {
                    write_ansi(buf, SetForegroundColor(Color::Red));
                    buf.extend_from_slice(format!("Execution failed: {}\n", e).as_bytes());
                    write_ansi(buf, ResetColor);
                }
                Ok(false)
            }
        }
    }

    /// Compile, check and run the exercise.
    /// The output is written to the `output` buffer after clearing it.
    #[inline]
    fn run_exercise(&self, output: Option<&mut Vec<u8>>, cmd_runner: &CmdRunner) -> Result<bool> {
        self.run::<false>(self.name(), output, cmd_runner)
    }

    /// Compile, check and run the exercise's solution.
    /// The output is written to the `output` buffer after clearing it.
    fn run_solution(&self, output: Option<&mut Vec<u8>>, cmd_runner: &CmdRunner) -> Result<bool> {
        let name = self.name();
        let mut bin_name = String::with_capacity(name.len() + 4);
        bin_name.push_str(name);
        bin_name.push_str("_sol");

        self.run::<true>(&bin_name, output, cmd_runner)
    }

    fn sol_path(&self) -> String {
        let name = self.name();

        let mut path = if let Some(dir) = self.dir() {
            // 14 = 10 + 1 + 3
            // solutions/ + / + .cm
            let mut path = String::with_capacity(14 + dir.len() + name.len());
            path.push_str("solutions/");
            path.push_str(dir);
            path.push('/');
            path
        } else {
            // 13 = 10 + 3
            // solutions/ + .cm
            let mut path = String::with_capacity(13 + name.len());
            path.push_str("solutions/");
            path
        };

        path.push_str(name);
        path.push_str(".cm");

        path
    }
}

impl RunnableExercise for Exercise {
    #[inline]
    fn name(&self) -> &str {
        self.name
    }

    #[inline]
    fn dir(&self) -> Option<&str> {
        self.dir
    }

    #[inline]
    fn strict_clippy(&self) -> bool {
        self.strict_clippy
    }

    #[inline]
    fn test(&self) -> bool {
        self.test
    }

    #[inline]
    fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }
}
