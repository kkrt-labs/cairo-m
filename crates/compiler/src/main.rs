use ariadne::{Label, Report, ReportKind, Source};
use cairo_m_compiler_parser::lexer::{LexingError, TokenType};
use cairo_m_compiler_parser::parser::parser;
use chumsky::input::Stream;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use chumsky::Parser as ChumskyParser;
use clap::Parser;
use logos::Logos;
use salsa::Database;
use std::fs;
use std::path::PathBuf;

/// Cairo-M compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file to compile
    #[arg(short, long)]
    input: PathBuf,
}

/// Result of lexing a source file
struct LexResult<'a> {
    tokens: Vec<(TokenType<'a>, logos::Span)>,
    errors: Vec<(LexingError, logos::Span)>,
}

/// Result of parsing tokens
struct ParseResult<'a, 'db> {
    ast: Vec<cairo_m_compiler_parser::parser::TopLevelItem<'db>>,
    errors: Vec<Rich<'a, TokenType<'a>, SimpleSpan>>,
}

fn main() {
    let args = Args::parse();
    println!("Reading file: {}", args.input.display());

    match fs::read_to_string(&args.input) {
        Ok(content) => {
            // Step 1: Lexing
            let lex_result = lex_source(&content);
            if !lex_result.errors.is_empty() {
                println!("\nLexing errors:");
                for (error, span) in lex_result.errors {
                    println!(
                        "{}",
                        build_lexer_error_message(&content, error, span.into())
                    );
                }
                std::process::exit(1);
            }

            // Step 2: Parsing
            let db = cairo_m_compiler_parser::ParserDatabaseImpl::default();
            // Attaching the database for debug printouts
            db.attach(|db| {
                let parse_result = parse_tokens(lex_result.tokens, &content, db);
                if !parse_result.errors.is_empty() {
                    println!("\nParsing errors:");
                    for error in parse_result.errors {
                        println!("{}", build_parser_error_message(&content, error));
                    }
                    std::process::exit(1);
                }

                // For now, just print the AST
                println!("\nAST:");
                println!("{:#?}", parse_result.ast);
            });
        }
        Err(e) => eprintln!("Error reading file: {e}"),
    }
}

/// Lex the source code into tokens
fn lex_source(source: &str) -> LexResult {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (token_result, span) in TokenType::lexer(source).spanned() {
        match token_result {
            Ok(token) => tokens.push((token, span)),
            Err(lexing_error) => {
                errors.push((lexing_error, span));
            }
        }
    }

    LexResult { tokens, errors }
}

/// Parse tokens into an AST
fn parse_tokens<'a: 'db, 'db>(
    tokens: Vec<(TokenType<'a>, logos::Span)>,
    source: &str,
    db: &'db dyn salsa::Database,
) -> ParseResult<'a, 'db> {
    let token_stream =
        Stream::from_iter(tokens).map((0..source.len()).into(), |(t, s): (_, _)| (t, s.into()));

    match parser(db)
        .then_ignore(end())
        .parse(token_stream)
        .into_result()
    {
        Ok(ast) => ParseResult {
            ast,
            errors: Vec::new(),
        },
        Err(errs) => ParseResult {
            ast: Vec::new(),
            errors: errs,
        },
    }
}

fn build_lexer_error_message(source: &str, error: LexingError, span: SimpleSpan) -> String {
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

fn build_parser_error_message(source: &str, error: Rich<TokenType, SimpleSpan>) -> String {
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
