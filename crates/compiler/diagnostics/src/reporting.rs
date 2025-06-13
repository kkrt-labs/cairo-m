//! # Error Reporting Utilities for Parser
//!
//! This module provides error reporting and diagnostic formatting
//! utilities for compiler-related diagnostics.

use crate::Diagnostic;
// `Source` import is removed as it's no longer directly used by name in this function.
// If other functions use `Source::from`, it might still be needed there,
// or ariadne::Source::from can be used.
use ariadne::{Label, Report};

/// Build a formatted message for a parse diagnostic
pub fn build_diagnostic_message(
    source_content: &str,
    diagnostic: &Diagnostic,
    with_color: bool,
) -> String {
    let mut write_buffer = Vec::new();
    let code_u32: u32 = diagnostic.code.into();

    // The ID for Ariadne will be the file path (String).
    let file_id = diagnostic.file_path.clone();

    // The span tuple for Ariadne is (ID, Range).
    // We clone file_id here because report_span needs an owned or clonable ID,
    // and the cache below will also take ownership of an ID.
    let report_span = (file_id.clone(), diagnostic.span.into_range());

    // To use `ariadne::sources`, the source string type `S` must satisfy `S: AsRef<str> + 'static`.
    // If `source_content` is `&str` (i.e., `&'a str`), then `'a` must be `'static` for `S` to be `&'a str`.
    // Since `source_content` is not guaranteed to be `'static str`, we convert it to an owned `String`.
    // `String` as a type is `'static` (it doesn't embed non-static lifetimes).
    let owned_source_content = source_content.to_string();

    // Create a cache that Ariadne can use to fetch source snippets.
    // `ariadne::sources` takes an iterator of (ID, SourceString) tuples.
    // Here, `file_id` (type `String`) is the ID.
    // `owned_source_content` (type `String`) is the source content.
    // Both `String` (for ID) and `String` (for S) satisfy the 'static bounds required by `ariadne::sources`.
    // The `file_id` is moved into the closure for the cache.
    let cache = ariadne::sources(vec![(file_id, owned_source_content)]);

    Report::build(diagnostic.severity.into(), report_span.clone()) // Pass the span with the String ID
        .with_config(
            ariadne::Config::new() // Using qualified path as in original code
                .with_index_type(ariadne::IndexType::Byte) // Using qualified path
                .with_color(with_color),
        )
        .with_code(code_u32)
        .with_message(&diagnostic.message)
        .with_label(Label::new(report_span).with_message(&diagnostic.message)) // Label also uses the span with String ID
        .finish()
        // Pass the cache. `cache` is `impl Cache<String>`.
        .write(cache, &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
