//! # Error Reporting Utilities for Parser
//!
//! This module provides error reporting and diagnostic formatting
//! utilities for compiler-related diagnostics.

use crate::Diagnostic;
use ariadne::{Label, Report, Source};

/// Build a formatted message for a parse diagnostic
pub fn build_diagnostic_message(source: &str, diagnostic: &Diagnostic, with_color: bool) -> String {
    let mut write_buffer = Vec::new();
    let code_u32: u32 = diagnostic.code.into();
    Report::build(
        diagnostic.severity.into(),
        ((), diagnostic.span.into_range()),
    )
    .with_config(
        ariadne::Config::new()
            .with_index_type(ariadne::IndexType::Byte)
            .with_color(with_color), // No color for tests
    )
    .with_code(code_u32)
    .with_message(&diagnostic.message)
    .with_label(Label::new(((), diagnostic.span.into_range())).with_message(&diagnostic.message))
    .finish()
    .write(Source::from(source), &mut write_buffer)
    .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
