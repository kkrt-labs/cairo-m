use std::path::{Path, PathBuf};

use cairo_m_compiler_diagnostics::build_diagnostic_message;
use cairo_m_compiler_parser::{parse_file, Db as ParserDb, ParsedModule, SourceFile, Upcast};
use insta::assert_snapshot;

#[salsa::db]
#[derive(Clone, Default)]
pub struct TestDb {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TestDb {}
#[salsa::db]
impl ParserDb for TestDb {}

impl Upcast<dyn ParserDb> for TestDb {
    fn upcast(&self) -> &(dyn ParserDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn ParserDb + 'static) {
        self
    }
}

fn test_db() -> TestDb {
    TestDb::default()
}

/// A snapshot-friendly representation of a successful parse.
#[derive(Debug)]
struct ParseSuccess<'a> {
    code: &'a str,
    ast: ParsedModule,
}

impl std::fmt::Display for ParseSuccess<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "--- Code ---")?;
        writeln!(f, "{}", self.code)?;
        writeln!(f, "--- AST ---")?;
        // Displaying only the items for cleaner snapshots
        write!(f, "{:#?}", self.ast.items())
    }
}

/// A snapshot-friendly representation of a failed parse.
#[derive(Debug)]
struct ParseError<'a> {
    code: &'a str,
    diagnostics: String,
}

impl std::fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "--- Code ---")?;
        writeln!(f, "{}", self.code)?;
        writeln!(f, "--- Diagnostics ---")?;
        write!(f, "{}", self.diagnostics)
    }
}

/// Asserts that the given code parses successfully without any diagnostics.
/// Panics if parsing fails or produces diagnostics.
/// Snapshots the resulting AST.
///
/// Use the `assert_parses_ok!` macro instead of calling this function directly
/// to get better snapshot names.
#[track_caller]
pub fn assert_parses_ok_impl(code: &str, test_name: &str) {
    let db = test_db();
    let source = SourceFile::new(&db, code.to_string(), "test.cairo".to_string());
    let result = parse_file(&db, source);

    if !result.diagnostics.is_empty() {
        let diagnostics = result
            .diagnostics
            .iter()
            .map(|d| build_diagnostic_message(code, d, false))
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Expected successful parse, but got diagnostics:\n{diagnostics}");
    }

    let snapshot = ParseSuccess {
        code,
        ast: result.module,
    };

    // Only keep local path to test name relative from crate root
    let base_path = "parser::";
    let local_test_name = test_name.split(base_path).nth(1).unwrap();

    insta::with_settings!({
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!(local_test_name, snapshot);
    });
}

/// Macro to assert that code parses successfully with a proper snapshot name.
/// The snapshot will be named after the calling function.
#[macro_export]
macro_rules! assert_parses_ok {
    ($code:expr) => {{
        // Use stdext to get the actual function name
        let function_name = stdext::function_name!();
        $crate::common::assert_parses_ok_impl($code, function_name)
    }};
}

/// Asserts that the given code fails to parse and produces diagnostics.
/// Panics if parsing succeeds without any diagnostics.
/// Snapshots the formatted diagnostic messages.
///
/// Use the `assert_parses_err!` macro instead of calling this function directly
/// to get better snapshot names.
#[track_caller]
pub fn assert_parses_err_impl(code: &str, test_name: &str) {
    let db = test_db();
    let source = SourceFile::new(&db, code.to_string(), "test.cairo".to_string());
    let result = parse_file(&db, source);

    if result.diagnostics.is_empty() {
        panic!("Expected parsing to fail, but it succeeded without diagnostics.");
    }

    let diagnostics = result
        .diagnostics
        .iter()
        .map(|d| build_diagnostic_message(code, d, false))
        .collect::<Vec<_>>()
        .join("\n\n");

    let snapshot = ParseError { code, diagnostics };

    let base_path = "parser::";
    let local_test_name = test_name.split(base_path).nth(1).unwrap();

    insta::with_settings!({
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!(format!("diagnostics__{}", local_test_name), snapshot);
    });
}

/// Macro to assert that code fails to parse with a proper snapshot name.
/// The snapshot will be named after the calling function.
#[macro_export]
macro_rules! assert_parses_err {
    ($code:expr) => {{
        // Use stdext to get the actual function name
        let function_name = stdext::function_name!();
        $crate::common::assert_parses_err_impl($code, function_name)
    }};
}

/// Macro for parameterized parser tests
/// Usage: assert_parses_parameterized! {
///     ok: ["valid1", "valid2"],
///     err: ["invalid1", "invalid2"]
/// }
#[macro_export]
macro_rules! assert_parses_parameterized {
    (ok: [$($ok:expr),* $(,)?], err: [$($err:expr),* $(,)?]) => {{
        let inputs_ok: Vec<(String, bool)> = vec![
            $(($ok.to_string(), true),)*
            ];

        let inputs_err: Vec<(String, bool)> = vec![
            $(($err.to_string(), false),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs_ok.iter().map(|(s, b)| (s.as_str(), *b)).collect();
        let inputs_ref_err: Vec<(&str, bool)> = inputs_err.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        let snapshot_ok_name = format!("{}_ok", function_name);
        let snapshot_err_name = format!("{}_err", function_name);
        $crate::common::assert_parses_parameterized_impl(&inputs_ref, &snapshot_ok_name);
        $crate::common::assert_parses_parameterized_impl(&inputs_ref_err, &snapshot_err_name);
    }};
    (ok: [$($ok:expr),* $(,)?]) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($ok.to_string(), true),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        let snapshot_name = format!("{}_ok", function_name);
        $crate::common::assert_parses_parameterized_impl(&inputs_ref, &snapshot_name)
    }};
    (err: [$($err:expr),* $(,)?]) => {{
        let inputs: Vec<(String, bool)> = vec![
            $(($err.to_string(), false),)*
        ];

        let inputs_ref: Vec<(&str, bool)> = inputs.iter().map(|(s, b)| (s.as_str(), *b)).collect();

        let function_name = stdext::function_name!();
        let snapshot_name = format!("{}_err", function_name);
        $crate::common::assert_parses_parameterized_impl(&inputs_ref, &snapshot_name)
    }};
}

/// Macro for file-based parser tests
/// Usage: assert_parses_files!(dir_path, pattern)
/// Example: assert_parses_files!("tests/test_cases", "*.cm")
#[macro_export]
macro_rules! assert_parses_files {
    ($dir_path:expr, $pattern:expr) => {{
        let function_name = stdext::function_name!();
        let path = std::path::Path::new($dir_path);
        $crate::common::assert_parses_files_impl(path, $pattern, function_name)
    }};
    ($dir_path:expr) => {{
        assert_parses_files!($dir_path, "*.cm")
    }};
}

/// Helper to wrap statement code inside a function, since most statements are not top-level.
pub fn in_function(code: &str) -> String {
    format!("fn test() {{ {code} }}")
}

/// Result of parameterized parsing tests
#[derive(Debug)]
struct ParameterizedParseResults {
    results: Vec<ParameterizedResult>,
}

#[derive(Debug)]
enum ParameterizedResult {
    Success { input: String, ast: ParsedModule },
    Error { input: String, diagnostics: String },
}

impl std::fmt::Display for ParameterizedParseResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, result) in self.results.iter().enumerate() {
            if i > 0 {
                writeln!(f, "\n{}\n", "=".repeat(60))?;
            }

            match result {
                ParameterizedResult::Success { input, ast } => {
                    writeln!(f, "--- Input {} ---", i + 1)?;
                    writeln!(f, "{}", input)?;
                    writeln!(f, "--- AST ---")?;
                    write!(f, "{:#?}", ast.items())?;
                }
                ParameterizedResult::Error { input, diagnostics } => {
                    writeln!(f, "--- Input {} (ERROR) ---", i + 1)?;
                    writeln!(f, "{}", input)?;
                    writeln!(f, "--- Diagnostics ---")?;
                    write!(f, "{}", diagnostics)?;
                }
            }
        }
        Ok(())
    }
}

/// Assert that multiple code snippets parse with expected results
#[track_caller]
pub fn assert_parses_parameterized_impl(
    inputs: &[(&str, bool)], // (code, should_succeed)
    test_name: &str,
) {
    let db = test_db();
    let mut results = Vec::new();

    for (code, should_succeed) in inputs {
        let source = SourceFile::new(&db, code.to_string(), "test.cairo".to_string());
        let result = parse_file(&db, source);

        if *should_succeed {
            if !result.diagnostics.is_empty() {
                let diagnostics = result
                    .diagnostics
                    .iter()
                    .map(|d| build_diagnostic_message(code, d, false))
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!(
                    "Expected successful parse for input '{}', but got diagnostics:\n{}",
                    code, diagnostics
                );
            }
            results.push(ParameterizedResult::Success {
                input: code.to_string(),
                ast: result.module,
            });
        } else {
            if result.diagnostics.is_empty() {
                panic!(
                    "Expected parsing to fail for input '{}', but it succeeded",
                    code
                );
            }
            let diagnostics = result
                .diagnostics
                .iter()
                .map(|d| build_diagnostic_message(code, d, false))
                .collect::<Vec<_>>()
                .join("\n\n");
            results.push(ParameterizedResult::Error {
                input: code.to_string(),
                diagnostics,
            });
        }
    }

    let snapshot = ParameterizedParseResults { results };

    let base_path = "parser::";
    let local_test_name = test_name.split(base_path).nth(1).unwrap_or(test_name);

    insta::with_settings!({
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!(format!("parameterized__{}", local_test_name), snapshot);
    });
}

/// Assert that all files in a directory parse successfully
#[track_caller]
pub fn assert_parses_files_impl(
    dir_path: &Path,
    pattern: &str, // e.g., "*.cm"
    test_name: &str,
) {
    let mut files = Vec::new();

    // Collect all matching files
    for entry in std::fs::read_dir(dir_path).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_string_lossy();
            if file_name.ends_with(&pattern[1..]) {
                // Simple pattern matching for now
                files.push(path);
            }
        } else if path.is_dir() {
            // Recursively search subdirectories
            collect_files_recursive(&path, pattern, &mut files);
        }
    }

    files.sort(); // Ensure consistent ordering

    if files.is_empty() {
        panic!(
            "No files found matching pattern '{}' in '{}'",
            pattern,
            dir_path.display()
        );
    }

    let db = test_db();

    for file_path in files {
        let code = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to read file '{}': {}", file_path.display(), e));

        let _file_stem = file_path.file_stem().unwrap().to_string_lossy();
        let relative_path = file_path
            .strip_prefix(dir_path)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "_");

        let source = SourceFile::new(&db, code.clone(), file_path.to_string_lossy().to_string());
        let result = parse_file(&db, source);

        let snapshot_name = format!("file__{}__{}", test_name, relative_path);

        if result.diagnostics.is_empty() {
            let snapshot = ParseSuccess {
                code: &code,
                ast: result.module,
            };

            insta::with_settings!({
                prepend_module_to_snapshot => false,
            }, {
                assert_snapshot!(snapshot_name, snapshot);
            });
        } else {
            let diagnostics = result
                .diagnostics
                .iter()
                .map(|d| build_diagnostic_message(&code, d, false))
                .collect::<Vec<_>>()
                .join("\n\n");

            let snapshot = ParseError {
                code: &code,
                diagnostics,
            };

            insta::with_settings!({
                prepend_module_to_snapshot => false,
            }, {
                assert_snapshot!(format!("diagnostics__{}", snapshot_name), snapshot);
            });
        }
    }
}

fn collect_files_recursive(dir: &Path, pattern: &str, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy();
                if file_name.ends_with(&pattern[1..]) {
                    files.push(path);
                }
            } else if path.is_dir() {
                collect_files_recursive(&path, pattern, files);
            }
        }
    }
}
