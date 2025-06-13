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

    let file_id = diagnostic.file_path.clone();
    let report_span = (file_id.clone(), diagnostic.span.into_range());
    let owned_source_content = source_content.to_string();

    // Create a cache that Ariadne can use to fetch source snippets.
    let cache = ariadne::sources(vec![(file_id, owned_source_content)]);

    Report::build(diagnostic.severity.into(), report_span.clone())
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(with_color),
        )
        .with_code(code_u32)
        .with_message(&diagnostic.message)
        .with_label(Label::new(report_span).with_message(&diagnostic.message))
        .finish()
        .write(cache, &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
