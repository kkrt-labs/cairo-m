//! # Error Reporting Utilities for Parser
//!
//! This module provides error reporting and diagnostic formatting
//! utilities for parser-related errors.

use crate::lexer::LexingError;
use crate::parser::ParseDiagnostic;
use ariadne::{Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;

/// Build a formatted error message for a lexing error
pub fn build_lexer_error_message(
    source: &str,
    error: LexingError,
    span: SimpleSpan,
    with_color: bool,
) -> String {
    let mut write_buffer = Vec::new();
    Report::build(ReportKind::Error, ((), span.into_range()))
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(with_color), // No color for consistent output
        )
        .with_code(3)
        .with_message(error.to_string())
        .with_label(Label::new(((), span.into_range())).with_message(format!("{error}")))
        .finish()
        .write(Source::from(source), &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}

/// Build formatted error messages for parser errors
pub fn build_parser_error_messages<'a, T>(
    source: &str,
    errs: Vec<Rich<'a, T, SimpleSpan>>,
    with_color: bool,
) -> Vec<String>
where
    T: std::fmt::Debug + std::fmt::Display,
{
    let mut reports = Vec::new();
    for err in errs {
        let mut write_buffer = Vec::new();
        Report::build(ReportKind::Error, ((), err.span().into_range()))
            .with_config(
                ariadne::Config::new()
                    .with_index_type(ariadne::IndexType::Byte)
                    .with_color(with_color), // No color for consistent output
            )
            .with_code(3)
            .with_message(err.to_string())
            .with_label(
                Label::new(((), err.span().into_range())).with_message(err.reason().to_string()),
            )
            .finish()
            .write(Source::from(source), &mut write_buffer)
            .unwrap();
        let report = String::from_utf8_lossy(&write_buffer).to_string();
        reports.push(report);
    }
    reports
}

/// Build a formatted error message for a parse diagnostic
pub fn build_parse_diagnostic_message(
    source: &str,
    diagnostic: &ParseDiagnostic,
    with_color: bool,
) -> String {
    let mut write_buffer = Vec::new();
    Report::build(ReportKind::Error, ((), diagnostic.span.into_range()))
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(with_color), // No color for tests
        )
        .with_code(3)
        .with_message(&diagnostic.message)
        .with_label(
            Label::new(((), diagnostic.span.into_range())).with_message(&diagnostic.message),
        )
        .finish()
        .write(Source::from(source), &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
