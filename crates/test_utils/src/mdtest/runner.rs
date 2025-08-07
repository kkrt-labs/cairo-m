use super::parser::{extract_tests, MdTest};
use std::path::Path;

type ProcessorType<'a> = Box<dyn Fn(&str, &str) -> Result<String, String> + 'a>;

/// Generic mdtest runner that can be configured for different compilation phases
pub struct MdTestRunner<'a> {
    /// Name of the compilation phase (e.g., "MIR", "Codegen")
    pub phase_name: &'a str,
    /// Function to process source code and return result string
    pub processor: ProcessorType<'a>,
    /// Whether to include parent directory in snapshot suffix
    pub include_parent_dir: bool,
}

impl<'a> MdTestRunner<'a> {
    pub fn new(
        phase_name: &'a str,
        processor: impl Fn(&str, &str) -> Result<String, String> + 'a,
    ) -> Self {
        Self {
            phase_name,
            processor: Box::new(processor),
            include_parent_dir: false,
        }
    }

    pub const fn with_parent_dir(mut self, include: bool) -> Self {
        self.include_parent_dir = include;
        self
    }

    /// Run tests from a markdown file and generate snapshots
    pub fn run_file(&self, path: &Path) -> Vec<TestSnapshot> {
        let tests = match extract_tests(path) {
            Ok(tests) => tests,
            Err(e) => {
                panic!("Failed to parse markdown file {}: {}", path.display(), e);
            }
        };

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let parent_dir = if self.include_parent_dir {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        tests
            .into_iter()
            .filter_map(|test| self.process_test(test, path, &file_stem, parent_dir.as_deref()))
            .collect()
    }

    fn process_test(
        &self,
        test: MdTest,
        path: &Path,
        file_stem: &str,
        parent_dir: Option<&str>,
    ) -> Option<TestSnapshot> {
        // Skip ignored tests
        if test.metadata.ignore.is_some() {
            return None;
        }

        let expects_error = test.metadata.expected_error.is_some();
        let result = (self.processor)(&test.cairo_source, &test.name);

        let snapshot_content = match (result, expects_error) {
            (Ok(output), false) => {
                format!(
                    "Test: {}\nFile: {}\n{}\nSource:\n{}\n{}\nGenerated {}:\n{}",
                    test.name,
                    path.display(),
                    "=".repeat(60),
                    test.cairo_source,
                    "=".repeat(60),
                    self.phase_name,
                    output
                )
            }
            (Ok(_), true) => {
                panic!("Test '{}' expected to fail but succeeded", test.name);
            }
            (Err(e), true) => {
                format!(
                    "Test: {}\nFile: {}\n{}\nSource:\n{}\n{}\nResult: EXPECTED ERROR\n{}",
                    test.name,
                    path.display(),
                    "=".repeat(60),
                    test.cairo_source,
                    "=".repeat(60),
                    e
                )
            }
            (Err(e), false) => {
                format!(
                    "Test: {}\nFile: {}\n{}\nSource:\n{}\n{}\nResult: UNEXPECTED ERROR\n{}",
                    test.name,
                    path.display(),
                    "=".repeat(60),
                    test.cairo_source,
                    "=".repeat(60),
                    e
                )
            }
        };

        let test_suffix = sanitize_test_name(&test.name);

        let snapshot_suffix = if let Some(parent) = parent_dir {
            format!("{}__{}__{}", parent, file_stem, test_suffix)
        } else {
            format!("{}@{}", file_stem, test_suffix)
        };

        Some(TestSnapshot {
            name: test.name,
            content: snapshot_content,
            suffix: snapshot_suffix,
        })
    }
}

pub struct TestSnapshot {
    pub name: String,
    pub content: String,
    pub suffix: String,
}

fn sanitize_test_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
