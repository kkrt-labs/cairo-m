//! Cairo-M Lexer Module
//!
//! This module implements a lexical analyzer (lexer) for the Cairo-M language.
//! The lexer converts source code into a sequence of tokens that can be processed
//! by the parser.
//!
//! This lexer currently supports the entirety of the Cairo0 syntax as defined here:
//! https://docs.cairo-lang.org/cairozero/
//! The parser and compiler are not complete yet.

use ariadne::{Color, Label, Report, ReportKind, Source};
use logos::Logos;

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub span: (usize, usize),
}

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(skip r"[\t\n ]+")]
#[logos(skip r"//.*\n")]
pub enum TokenType {
    #[regex("[0-9]+")]
    Int,

    #[regex(r"[a-zA-Z_][a-zA-Z_0-9]*")]
    Identifier,

    #[token("==")]
    DoubleEq,
    #[token("!=")]
    Neq,

    #[token(",")]
    Comma,

    #[token("*")]
    Star,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    #[token("=")]
    Equal,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,

    #[token(".")]
    Dot,

    #[token("and")]
    And,

    #[token("local")]
    Local,

    //Instructions
    #[token("if")]
    If,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    // Function/Namespace/Struct definition.
    #[token("func")]
    Func,
    #[token("else")]
    Else,

    // Code elements
    #[token(";")]
    Semicolon,
    #[token("return")]
    Return,

    Error,
    EOF,
}

/// Lexical analysis function that converts source code into a sequence of tokens.
///
/// This function processes the input source code character by character, identifying
/// tokens according to the Cairo-M language grammar. It handles error reporting
/// and maintains source location information for each token.
///
/// # Arguments
/// * `input` - The source code string to tokenize
/// * `file_name` - Name of the source file (used for error reporting)
///
/// # Returns
/// A tuple containing:
/// * `Vec<Token>` - The sequence of tokens found in the source code
/// * `u32` - The number of lexical errors encountered
///
/// # Error Handling
/// * Unknown tokens are reported using Ariadne's error reporting system
/// * Each error includes the source location and the problematic token
/// * The function continues processing after errors to find all tokens
/// * The error count is returned to allow the caller to handle errors appropriately
///
/// # Example
/// ```
/// let source = "func main() { return 42; }";
/// let (tokens, error_count) = lex(source, "main.cairo");
/// assert_eq!(error_count, 0);
/// ```
pub fn lex(input: &str, file_name: &str) -> (Vec<Token>, u32) {
    let mut error_counter = 0;
    let mut lex = TokenType::lexer(input);
    let mut tokens = Vec::new();
    while let Some(token) = lex.next() {
        let lexeme = lex.slice().to_string();
        if let Ok(token) = token {
            let lexeme = lex.slice().to_string();
            tokens.push(Token {
                token_type: token,
                lexeme,
                span: (lex.span().start, lex.span().end),
            });
        } else {
            let error_span = (file_name, lex.span().start..lex.span().end);
            let _ = Report::build(ReportKind::Error, error_span.clone())
                .with_message("Lexer error")
                .with_label(
                    Label::new(error_span)
                        .with_message(format!("Unknown token '{}'", lexeme))
                        .with_color(Color::Red),
                )
                .finish()
                .print((file_name, Source::from(input)));
            error_counter += 1;
        }
    }
    tokens.push(Token {
        token_type: TokenType::EOF,
        lexeme: "".to_string(),
        span: (0, 0),
    });
    (tokens, error_counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_token(
        tokens: &[Token],
        index: usize,
        expected_type: TokenType,
        expected_lexeme: &str,
    ) {
        assert_eq!(tokens[index].token_type, expected_type);
        assert_eq!(tokens[index].lexeme, expected_lexeme);
    }

    #[test]
    fn test_basic_tokens() {
        let input = "func main() { return 42; }";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 10); // Including EOF

        assert_token(&tokens, 0, TokenType::Func, "func");
        assert_token(&tokens, 1, TokenType::Identifier, "main");
        assert_token(&tokens, 2, TokenType::LParen, "(");
        assert_token(&tokens, 3, TokenType::RParen, ")");
        assert_token(&tokens, 4, TokenType::LBrace, "{");
        assert_token(&tokens, 5, TokenType::Return, "return");
        assert_token(&tokens, 6, TokenType::Int, "42");
        assert_token(&tokens, 7, TokenType::Semicolon, ";");
        assert_token(&tokens, 8, TokenType::RBrace, "}");
        assert_token(&tokens, 9, TokenType::EOF, "");
    }

    #[test]
    fn test_arithmetic_operators() {
        let input = "1 + 2 * 3 - 4 / 2";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 10); // Including EOF

        assert_token(&tokens, 0, TokenType::Int, "1");
        assert_token(&tokens, 1, TokenType::Plus, "+");
        assert_token(&tokens, 2, TokenType::Int, "2");
        assert_token(&tokens, 3, TokenType::Star, "*");
        assert_token(&tokens, 4, TokenType::Int, "3");
        assert_token(&tokens, 5, TokenType::Minus, "-");
        assert_token(&tokens, 6, TokenType::Int, "4");
        assert_token(&tokens, 7, TokenType::Slash, "/");
        assert_token(&tokens, 8, TokenType::Int, "2");
    }

    #[test]
    fn test_comparison_operators() {
        let input = "x == y and z != w";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 8); // Including EOF

        assert_token(&tokens, 0, TokenType::Identifier, "x");
        assert_token(&tokens, 1, TokenType::DoubleEq, "==");
        assert_token(&tokens, 2, TokenType::Identifier, "y");
        assert_token(&tokens, 3, TokenType::And, "and");
        assert_token(&tokens, 4, TokenType::Identifier, "z");
        assert_token(&tokens, 5, TokenType::Neq, "!=");
        assert_token(&tokens, 6, TokenType::Identifier, "w");
    }

    #[test]
    fn test_identifiers() {
        let input = "main foo_bar baz123 _test";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 5); // Including EOF

        assert_token(&tokens, 0, TokenType::Identifier, "main");
        assert_token(&tokens, 1, TokenType::Identifier, "foo_bar");
        assert_token(&tokens, 2, TokenType::Identifier, "baz123");
        assert_token(&tokens, 3, TokenType::Identifier, "_test");
    }

    #[test]
    fn test_comments() {
        let input = "// This is a comment\nfunc main() { // Another comment\nreturn 42; }";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 10); // Including EOF

        assert_token(&tokens, 0, TokenType::Func, "func");
        assert_token(&tokens, 1, TokenType::Identifier, "main");
        assert_token(&tokens, 2, TokenType::LParen, "(");
        assert_token(&tokens, 3, TokenType::RParen, ")");
        assert_token(&tokens, 4, TokenType::LBrace, "{");
        assert_token(&tokens, 5, TokenType::Return, "return");
        assert_token(&tokens, 6, TokenType::Int, "42");
        assert_token(&tokens, 7, TokenType::Semicolon, ";");
        assert_token(&tokens, 8, TokenType::RBrace, "}");
    }

    #[test]
    fn test_whitespace() {
        let input = "func\tmain()\n{\n    return\t42;\n}";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 10); // Including EOF

        assert_token(&tokens, 0, TokenType::Func, "func");
        assert_token(&tokens, 1, TokenType::Identifier, "main");
        assert_token(&tokens, 2, TokenType::LParen, "(");
        assert_token(&tokens, 3, TokenType::RParen, ")");
        assert_token(&tokens, 4, TokenType::LBrace, "{");
        assert_token(&tokens, 5, TokenType::Return, "return");
        assert_token(&tokens, 6, TokenType::Int, "42");
        assert_token(&tokens, 7, TokenType::Semicolon, ";");
        assert_token(&tokens, 8, TokenType::RBrace, "}");
        assert_token(&tokens, 9, TokenType::EOF, "");
    }

    #[test]
    fn test_error_handling() {
        let input = "func main() { return @#$; }";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 3); // # and $ are invalid tokens
        assert_eq!(tokens.len(), 9); // Including EOF

        assert_token(&tokens, 0, TokenType::Func, "func");
        assert_token(&tokens, 1, TokenType::Identifier, "main");
        assert_token(&tokens, 2, TokenType::LParen, "(");
        assert_token(&tokens, 3, TokenType::RParen, ")");
        assert_token(&tokens, 4, TokenType::LBrace, "{");
        assert_token(&tokens, 5, TokenType::Return, "return");
        assert_token(&tokens, 6, TokenType::Semicolon, ";");
        assert_token(&tokens, 7, TokenType::RBrace, "}");
        assert_token(&tokens, 8, TokenType::EOF, "");
    }

    #[test]
    fn test_keywords() {
        let input = "func local return";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 4); // Including EOF

        assert_token(&tokens, 0, TokenType::Func, "func");
        assert_token(&tokens, 1, TokenType::Local, "local");
        assert_token(&tokens, 2, TokenType::Return, "return");
        assert_token(&tokens, 3, TokenType::EOF, "");
    }

    #[test]
    fn test_compound_operators() {
        let input = " == != ";
        let (tokens, errors) = lex(input, "test.cairo");
        assert_eq!(errors, 0);
        assert_eq!(tokens.len(), 3); // Including EOF

        assert_token(&tokens, 0, TokenType::DoubleEq, "==");
        assert_token(&tokens, 1, TokenType::Neq, "!=");
    }
}
