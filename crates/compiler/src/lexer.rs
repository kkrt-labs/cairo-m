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

    #[regex(r"%\{(.*)%\}")]
    Hint,

    #[regex(r"0x[0-9a-fA-F]+")]
    HexInt,
    #[regex(r"[a-zA-Z_][a-zA-Z_0-9]*(\.[a-zA-Z_][a-zA-Z_0-9]*)*")]
    Identifier,
    #[regex(r#"".""#)]
    String,
    #[regex(r"'.'")]
    ShortString,

    #[token("++")]
    PlusPlus,
    #[token("==")]
    DoubleEq,
    #[token("**")]
    DoubleStar,
    #[token("!=")]
    Neq,
    #[token("->")]
    Arrow,
    #[token("@")]
    At,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[token("felt")]
    Felt,
    #[token("codeoffset")]
    CodeOffset,

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

    #[token("&")]
    Ampersand,
    #[token("new")]
    New,

    #[token(".")]
    Dot,

    //Atom
    #[token("nondet")]
    NonDet,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("cast")]
    Cast,

    //Reg
    #[token("ap")]
    Ap,
    #[token("fp")]
    Fp,

    #[token("and")]
    And,

    #[token("local")]
    Local,

    #[token("ret")]
    Ret,

    //Instructions
    #[token("call")]
    Call,
    #[token("rel")]
    Rel,
    #[token("abs")]
    Abs,
    #[token("jmp")]
    Jmp,
    #[token("if")]
    If,
    #[token("+=")]
    PlusEq,
    #[token("dw")]
    Dw,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    // Import statement
    #[token("import")]
    Import,
    #[token("from")]
    From,
    #[token("as")]
    As,

    // Function/Namespace/Struct definition.
    #[token("func")]
    Func,
    #[token("with")]
    With,
    #[token("struct")]
    Struct,
    #[token("namespace")]
    Namespace,
    #[token("with_attr")]
    WithAttr,
    #[token("else")]
    Else,

    // Cairo file
    // #[token("\n")]
    // Newline,

    // Code elements
    #[token(";")]
    Semicolon,
    #[token("const")]
    Const,
    #[token("let")]
    Let,
    #[token("tempvar")]
    TempVar,
    #[token("assert")]
    Assert,
    #[token("static_assert")]
    StaticAssert,
    #[token("return")]
    Return,
    #[token("using")]
    Using,
    #[token("alloc_locals")]
    AllocLocals,

    EOF,
    Error,
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
