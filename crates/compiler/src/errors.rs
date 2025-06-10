//! This module contains functions to build error messages for the compiler using ariadne.

use ariadne::{Label, Report, ReportKind, Source};
use cairo_m_compiler_parser::lexer::LexingError;
use cairo_m_compiler_parser::lexer::TokenType;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;

pub fn build_lexer_error_message(source: &str, error: LexingError, span: SimpleSpan) -> String {
    let mut write_buffer = Vec::new();
    Report::build(ReportKind::Error, ((), span.into_range()))
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(false),
        )
        .with_code(3)
        .with_message(error.to_string())
        .with_label(Label::new(((), span.into_range())).with_message(format!("{error}")))
        .finish()
        .write(Source::from(source), &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}

pub fn build_parser_error_message(source: &str, error: Rich<TokenType, SimpleSpan>) -> String {
    let mut write_buffer = Vec::new();
    Report::build(ReportKind::Error, ((), error.span().into_range()))
        .with_config(
            ariadne::Config::new()
                .with_index_type(ariadne::IndexType::Byte)
                .with_color(false),
        )
        .with_code(3)
        .with_message(error.to_string())
        .with_label(
            Label::new(((), error.span().into_range())).with_message(error.reason().to_string()),
        )
        .finish()
        .write(Source::from(source), &mut write_buffer)
        .unwrap();
    String::from_utf8_lossy(&write_buffer).to_string()
}
