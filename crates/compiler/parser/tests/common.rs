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

/// Helper to wrap statement code inside a function, since most statements are not top-level.
pub fn in_function(code: &str) -> String {
    format!("func test() {{ {code} }}")
}
