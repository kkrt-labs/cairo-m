use anyhow::{bail, Context, Error, Result};
use serde::Deserialize;
use std::{fs, io::ErrorKind};

use crate::{embedded::EMBEDDED_FILES, exercise::RunnableExercise};

/// One test case executed via calling the `main` entrypoint.
/// Inputs and outputs are string-encoded using the same syntax accepted by
/// `cairo-m-runner --arguments` (e.g. numbers, bools, tuples `()`, lists `[]`, structs `{}`).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct TestCase {
    /// List of arguments passed positionally to `main`.
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Expected return values from `main` (in order).
    #[serde(default)]
    pub outputs: Vec<String>,
}

/// Deserialized from the `info.toml` file.
#[derive(Deserialize, Debug, Clone)]
pub struct ExerciseInfo {
    /// Exercise's unique name.
    pub name: String,
    /// Exercise's directory name inside the `exercises/` directory.
    pub dir: Option<String>,
    /// Run `cargo test` on the exercise.
    #[serde(default = "default_true")]
    pub test: bool,
    /// Test cases to validate this exercise when `test = true`.
    /// Each case is executed by calling the `main` entrypoint with `inputs`
    /// and comparing the returned values against `outputs`.
    #[serde(default, rename = "test-cases")]
    pub test_cases: Vec<TestCase>,
    /// Deny all Clippy warnings.
    #[serde(default)]
    pub strict_clippy: bool,
    /// The exercise's hint to be shown to the user on request.
    pub hint: String,
    /// The exercise is already solved. Ignore it when checking that all exercises are unsolved.
    #[serde(default)]
    pub skip_check_unsolved: bool,
}
#[inline(always)]
const fn default_true() -> bool {
    true
}

impl ExerciseInfo {
    /// Path to the exercise file starting with the `exercises/` directory.
    pub fn path(&self) -> String {
        let mut path = if let Some(dir) = &self.dir {
            // 14 = 10 + 1 + 3
            // exercises/ + / + .cm
            let mut path = String::with_capacity(14 + dir.len() + self.name.len());
            path.push_str("exercises/");
            path.push_str(dir);
            path.push('/');
            path
        } else {
            // 13 = 10 + 3
            // exercises/ + .cm
            let mut path = String::with_capacity(13 + self.name.len());
            path.push_str("exercises/");
            path
        };

        path.push_str(&self.name);
        path.push_str(".cm");

        path
    }
}

impl RunnableExercise for ExerciseInfo {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn dir(&self) -> Option<&str> {
        self.dir.as_deref()
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

/// The deserialized `info.toml` file.
#[derive(Deserialize)]
pub struct InfoFile {
    /// For possible breaking changes in the future for community exercises.
    pub format_version: u8,
    /// Shown to users when starting with the exercises.
    pub welcome_message: Option<String>,
    /// Shown to users after finishing all exercises.
    pub final_message: Option<String>,
    /// List of all exercises.
    pub exercises: Vec<ExerciseInfo>,
}

impl InfoFile {
    /// Official exercises: Parse the embedded `info.toml` file.
    /// Community exercises: Parse the `info.toml` file in the current directory.
    pub fn parse() -> Result<Self> {
        // Read a local `info.toml` if it exists.
        let slf = match fs::read_to_string("info.toml") {
            Ok(file_content) => toml::de::from_str::<Self>(&file_content)
                .context("Failed to parse the `info.toml` file")?,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return toml::de::from_str(EMBEDDED_FILES.info_file)
                        .context("Failed to parse the embedded `info.toml` file");
                }

                return Err(Error::from(e).context("Failed to read the `info.toml` file"));
            }
        };

        if slf.exercises.is_empty() {
            bail!("{NO_EXERCISES_ERR}");
        }

        Ok(slf)
    }
}

const NO_EXERCISES_ERR: &str = "There are no exercises yet!
Add at least one exercise before testing.";
