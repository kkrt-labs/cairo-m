use cairo_m_compiler_parser::{ParserDatabaseImpl, SourceFile};
use cairo_m_formatter::{FormatterConfig, format_source_file};

fn format_code(source: &str) -> String {
    let db = ParserDatabaseImpl::default();
    let file = SourceFile::new(&db, source.to_string(), "test.cm".to_string());
    let config = FormatterConfig::default();
    format_source_file(&db, file, &config)
}

// Verify docstrings and nearby line comments stick to the right function
#[test]
fn attaches_docstring_and_todo_to_function() {
    let input = r#"use math::ops::{add, sub};

/// Adds one to the input
// TODO: consider overflow behavior
fn inc(x: felt) -> felt {
    let y = x + 1; // increment
    return y;
}

/// No-op function
fn noop() -> () {
}
"#;

    // Expected: two blank lines after the use, then the docstring block, then the function.
    // The inline end-of-line comment should remain at the end of the `let` line.
    let expected = r#"use math::ops::{add, sub};

/// Adds one to the input
// TODO: consider overflow behavior
fn inc(x: felt) -> felt {
    let y = x + 1; // increment
    return y;
}

/// No-op function
fn noop() {}
"#;

    assert_eq!(format_code(input), expected);
}
