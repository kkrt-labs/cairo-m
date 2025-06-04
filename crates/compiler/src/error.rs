//! Cairo-M Error Reporting Module
//!
//! This module provides error reporting functionality for the Cairo-M compiler.
//! It uses the Ariadne library to generate user-friendly error messages with
//! source code context and highlighting.
//!
//! The error reporting system:
//! - Shows the exact location of errors in the source code
//! - Provides context around the error location
//! - Uses color highlighting for better visibility
//! - Supports multiple error types and messages

use ariadne::{Color, Label, Report, ReportKind, Source};

/// Reports a compilation error with source location and context.
///
/// This function generates a user-friendly error message that includes:
/// - The type of error
/// - The error message
/// - The source location
/// - Context around the error
/// - Color highlighting
///
/// # Arguments
/// * `file_name` - Name of the source file containing the error
/// * `source` - The complete source code
/// * `error_span` - Tuple of (start, end) positions of the error in the source
/// * `error_type` - Category or type of the error
/// * `message` - Detailed error message
///
/// # Example
/// ```
/// report_error(
///     "main.cairo".to_string(),
///     "func main() {".to_string(),
///     (11, 12),
///     "Syntax error".to_string(),
///     "Expected ')'".to_string()
/// );
/// ```
pub fn report_error(
    file_name: String,
    source: String,
    error_span: (usize, usize),
    error_type: String,
    message: String,
) {
    let span = (file_name.clone(), error_span.0..error_span.1);
    let _ = Report::build(ReportKind::Error, span.clone())
        .with_message(error_type)
        .with_label(
            Label::new(span)
                .with_message(message)
                .with_color(Color::Red),
        )
        .finish()
        .print((file_name, Source::from(source)));
}
