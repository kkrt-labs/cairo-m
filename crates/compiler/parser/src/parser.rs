use crate::lexer::TokenType;
use chumsky::{input::ValueInput, prelude::*};

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Statement {
    // TODO: add actual statements here
    Null,
}

#[allow(dead_code)]
fn parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Statement, extra::Err<Rich<'tokens, TokenType<'src>>>>
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    // BOILERPLATE code - TODO
    recursive(|_statement| just(TokenType::Let).map(|_| Statement::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ariadne::{Label, Report, ReportKind, Source};
    use chumsky::input::Stream;
    use chumsky::Parser;
    use logos::Logos;

    // Macro to simplify snapshot testing for parse results
    macro_rules! assert_parse_snapshot {
        ($input:expr) => {
            assert_parse_snapshot!($input, stringify!($input))
        };
        ($input:expr, $name:expr) => {
            let result = parse_str($input);
            match result {
                Ok(stmt) => {
                    insta::assert_debug_snapshot!($name, stmt);
                }
                Err(errs) => {
                    for (i, err) in errs.iter().enumerate() {
                        let snapshot_name = if errs.len() == 1 {
                            format!("{}_error", $name)
                        } else {
                            format!("{}_error_{}", $name, i)
                        };
                        insta::assert_snapshot!(snapshot_name, err);
                    }
                }
            }
        };
    }

    // Helper function to parse a string input
    fn parse_str(input: &str) -> Result<Statement, Vec<String>> {
        let token_iter = TokenType::lexer(input)
            .spanned()
            .map(|(tok, span)| match tok {
                Ok(tok) => (tok, span.into()),
                Err(()) => (TokenType::Error, span.into()),
            });

        let token_stream =
            Stream::from_iter(token_iter).map((0..input.len()).into(), |(t, s): (_, _)| (t, s));

        parser()
            .parse(token_stream)
            .into_result()
            .map_err(|errs| build_error_message(input, errs))
    }

    fn build_error_message(source: &str, errs: Vec<Rich<TokenType, SimpleSpan>>) -> Vec<String> {
        let mut reports = Vec::new();
        for err in errs {
            let mut write_buffer = Vec::new();
            Report::build(ReportKind::Error, ((), err.span().into_range()))
                .with_config(
                    ariadne::Config::new()
                        .with_index_type(ariadne::IndexType::Byte)
                        .with_color(false),
                )
                .with_code(3)
                .with_message(err.to_string())
                .with_label(
                    Label::new(((), err.span().into_range()))
                        .with_message(err.reason().to_string()),
                )
                .finish()
                .write(Source::from(source), &mut write_buffer)
                .unwrap();
            let report = String::from_utf8_lossy(&write_buffer).to_string();
            reports.push(report);
        }
        reports
    }

    #[test]
    fn test_simple_let_declaration() {
        assert_parse_snapshot!("let x = 3;", "simple_let");
    }
}
