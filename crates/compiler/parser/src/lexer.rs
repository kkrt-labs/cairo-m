// crates/cairo-m-compiler-parser/src/tokens.rs
use std::fmt;

use logos::Logos;

pub const VALID_SUFFIXES: &[&str] = &["felt", "u32"];

/// A numeric literal with an optional type suffix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberLiteral<'a> {
    pub value: u64,
    pub suffix: Option<&'a str>,
}

/// Custom error type for lexing errors in the Cairo-M language.
///
/// This enum represents different types of errors that can occur during lexical analysis,
/// providing detailed information about what went wrong and where.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum LexingError {
    /// An integer literal that cannot be parsed or is out of range
    InvalidNumber {
        /// The problematic number string that failed to parse
        value: String,
        /// The reason for the parsing failure
        reason: NumberParseError,
    },
    /// A character that is not recognized as part of any valid token
    #[default]
    InvalidCharacter,
}

/// Specific errors that can occur when parsing numeric literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberParseError {
    /// The number is too large to fit in the target integer type (u32)
    Overflow,
    /// The number format is invalid (e.g., invalid characters for the base)
    InvalidFormat,
    /// The suffix is invalid (e.g., not a valid type)
    InvalidSuffix,
    /// An unknown parsing error occurred
    Unknown(String),
}

impl fmt::Display for LexingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { value, reason } => {
                write!(f, "Invalid number '{value}': {reason}")
            }
            Self::InvalidCharacter => {
                write!(f, "Invalid character")
            }
        }
    }
}

impl fmt::Display for NumberParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => write!(f, "Value is higher than u64::max"),
            Self::InvalidFormat => write!(f, "Invalid number format"),
            Self::Unknown(s) => write!(f, "{s}"),
            Self::InvalidSuffix => write!(f, "Invalid suffix"),
        }
    }
}

/// Helper function to parse numeric literals with proper error reporting.
///
/// This function handles different numeric bases (decimal, hexadecimal, octal, binary)
/// and optional type suffixes, providing detailed error information when parsing fails.
fn parse_number_literal<'a>(
    lex: &logos::Lexer<'a, TokenType<'a>>,
) -> Result<NumberLiteral<'a>, LexingError> {
    let slice = lex.slice();

    // Parse based on prefix to determine the base
    let (remaining, base) = if slice.starts_with("0x") || slice.starts_with("0X") {
        (&slice[2..], 16)
    } else if slice.starts_with("0o") || slice.starts_with("0O") {
        (&slice[2..], 8)
    } else if slice.starts_with("0b") || slice.starts_with("0B") {
        (&slice[2..], 2)
    } else {
        (slice, 10)
    };

    // Find where the number ends and the suffix begins
    // Check for known suffixes first to handle ambiguous cases like "0xFFfelt"
    let digit_end = VALID_SUFFIXES
        .iter()
        .filter_map(|&suffix| {
            remaining.rfind(suffix).map(|pos| {
                // Make sure the suffix is at a valid position (after at least one digit)
                if pos > 0 && remaining[pos..] == *suffix {
                    pos
                } else {
                    remaining.len()
                }
            })
        })
        .min()
        .unwrap_or_else(|| {
            // Fallback to the standard approach
            remaining
                .find(|c: char| !c.is_digit(base))
                .unwrap_or(remaining.len())
        });

    let (number_str, suffix_str) = remaining.split_at(digit_end);

    // Check for empty number string (e.g., "0x" followed by non-hex chars)
    if number_str.is_empty() {
        return Err(LexingError::InvalidNumber {
            value: slice.to_string(),
            reason: NumberParseError::InvalidFormat,
        });
    }

    // Parse suffix if present
    let suffix = if suffix_str.is_empty() {
        None
    } else {
        // Validate that suffix starts with a letter or underscore (not a digit)
        if suffix_str.chars().next().unwrap().is_numeric() {
            return Err(LexingError::InvalidNumber {
                value: slice.to_string(),
                reason: NumberParseError::InvalidFormat,
            });
        }
        Some(suffix_str)
    };

    // Parse the number string as u64
    match u64::from_str_radix(number_str, base) {
        Ok(n) => Ok(NumberLiteral { value: n, suffix }),
        Err(err) => {
            let reason = match err.kind() {
                std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow => {
                    NumberParseError::Overflow
                }
                std::num::IntErrorKind::InvalidDigit => NumberParseError::InvalidFormat,
                _ => NumberParseError::Unknown(err.to_string()),
            };

            Err(LexingError::InvalidNumber {
                value: slice.to_string(),
                reason,
            })
        }
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[\t\n\r ]+")] // Skip whitespace, including carriage return
#[logos(skip r"//[^\n]*")] // Skip single-line comments
#[logos(error = LexingError)] // Use our custom error type
pub enum TokenType<'a> {
    // Literals
    #[regex(
        r"(0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][01]+|[0-9]+)([a-zA-Z_][a-zA-Z0-9_]*)?",
        parse_number_literal
    )]
    LiteralNumber(NumberLiteral<'a>),
    // Keywords
    #[token("as")]
    As,
    #[token("const")]
    Const,
    #[token("else")]
    Else,
    #[token("false")]
    False,
    #[token("fn")]
    Function,
    #[token("if")]
    If,
    #[token("let")]
    Let,
    #[token("return")]
    Return,
    #[token("struct")]
    Struct,
    #[token("true")]
    True,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("use")]
    Use,
    // Identifiers (must come after keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier(&'a str),
    // Operators (order matters for longest match)
    #[token("!")]
    Not,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("&")]
    BitwiseAnd,
    #[token("|")]
    BitwiseOr,
    #[token("^")]
    BitwiseXor,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Neq,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("=")]
    Eq,
    // Punctuation
    #[token("->")]
    Arrow,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBrack,
    #[token("]")]
    RBrack,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token("..")]
    DotDot,
    #[token(".")]
    Dot,
}

impl<'a> fmt::Display for TokenType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::LiteralNumber(literal) => {
                if let Some(s) = literal.suffix {
                    write!(f, "{}{s}", literal.value)
                } else {
                    write!(f, "{}", literal.value)
                }
            }
            TokenType::Identifier(s) => write!(f, "{s}"),
            TokenType::As => write!(f, "as"),
            TokenType::Const => write!(f, "const"),
            TokenType::Else => write!(f, "else"),
            TokenType::False => write!(f, "false"),
            TokenType::Function => write!(f, "fn"),
            TokenType::If => write!(f, "if"),
            TokenType::Let => write!(f, "let"),
            TokenType::Return => write!(f, "return"),
            TokenType::Struct => write!(f, "struct"),
            TokenType::True => write!(f, "true"),
            TokenType::While => write!(f, "while"),
            TokenType::Loop => write!(f, "loop"),
            TokenType::For => write!(f, "for"),
            TokenType::In => write!(f, "in"),
            TokenType::Break => write!(f, "break"),
            TokenType::Continue => write!(f, "continue"),
            TokenType::Not => write!(f, "!"),
            TokenType::AndAnd => write!(f, "&&"),
            TokenType::OrOr => write!(f, "||"),
            TokenType::BitwiseAnd => write!(f, "&"),
            TokenType::BitwiseOr => write!(f, "|"),
            TokenType::BitwiseXor => write!(f, "^"),
            TokenType::EqEq => write!(f, "=="),
            TokenType::Neq => write!(f, "!="),
            TokenType::LessEqual => write!(f, "<="),
            TokenType::GreaterEqual => write!(f, ">="),
            TokenType::Less => write!(f, "<"),
            TokenType::Greater => write!(f, ">"),
            TokenType::Plus => write!(f, "+"),
            TokenType::Minus => write!(f, "-"),
            TokenType::Mul => write!(f, "*"),
            TokenType::Div => write!(f, "/"),
            TokenType::Eq => write!(f, "="),
            TokenType::Arrow => write!(f, "->"),
            TokenType::LParen => write!(f, "("),
            TokenType::RParen => write!(f, ")"),
            TokenType::LBrace => write!(f, "{{"),
            TokenType::RBrace => write!(f, "}}"),
            TokenType::LBrack => write!(f, "["),
            TokenType::RBrack => write!(f, "]"),
            TokenType::Comma => write!(f, ","),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Colon => write!(f, ":"),
            TokenType::ColonColon => write!(f, "::"),
            TokenType::DotDot => write!(f, ".."),
            TokenType::Dot => write!(f, "."),
            TokenType::Use => write!(f, "use"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lexer() {
        let input = r#"
            use std::math::add;

            fn add(x: felt, y: felt) -> felt {
                let result = x + y;
                if result == 0 {
                    return result;
                } else {
                    return 0;
                }
            }

            let value = add(10, 20);
            const MAX_SIZE = 100;
            let array = alloc();
            array[1];
        "#;
        let lexer = TokenType::lexer(input);

        let mut tokens = vec![];
        let mut errors = vec![];
        for (token, span) in lexer.spanned() {
            match token {
                Ok(token) => tokens.push(token),
                Err(e) => {
                    errors.push((span, e));
                }
            }
        }

        if !errors.is_empty() {
            panic!("lexer errors: {errors:?}");
        }

        let expected = vec![
            TokenType::Use,
            TokenType::Identifier("std"),
            TokenType::ColonColon,
            TokenType::Identifier("math"),
            TokenType::ColonColon,
            TokenType::Identifier("add"),
            TokenType::Semicolon,
            TokenType::Function,
            TokenType::Identifier("add"),
            TokenType::LParen,
            TokenType::Identifier("x"),
            TokenType::Colon,
            TokenType::Identifier("felt"),
            TokenType::Comma,
            TokenType::Identifier("y"),
            TokenType::Colon,
            TokenType::Identifier("felt"),
            TokenType::RParen,
            TokenType::Arrow,
            TokenType::Identifier("felt"),
            TokenType::LBrace,
            TokenType::Let,
            TokenType::Identifier("result"),
            TokenType::Eq,
            TokenType::Identifier("x"),
            TokenType::Plus,
            TokenType::Identifier("y"),
            TokenType::Semicolon,
            TokenType::If,
            TokenType::Identifier("result"),
            TokenType::EqEq,
            TokenType::LiteralNumber(NumberLiteral {
                value: 0,
                suffix: None,
            }),
            TokenType::LBrace,
            TokenType::Return,
            TokenType::Identifier("result"),
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::Else,
            TokenType::LBrace,
            TokenType::Return,
            TokenType::LiteralNumber(NumberLiteral {
                value: 0,
                suffix: None,
            }),
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::RBrace,
            TokenType::Let,
            TokenType::Identifier("value"),
            TokenType::Eq,
            TokenType::Identifier("add"),
            TokenType::LParen,
            TokenType::LiteralNumber(NumberLiteral {
                value: 10,
                suffix: None,
            }),
            TokenType::Comma,
            TokenType::LiteralNumber(NumberLiteral {
                value: 20,
                suffix: None,
            }),
            TokenType::RParen,
            TokenType::Semicolon,
            TokenType::Const,
            TokenType::Identifier("MAX_SIZE"),
            TokenType::Eq,
            TokenType::LiteralNumber(NumberLiteral {
                value: 100,
                suffix: None,
            }),
            TokenType::Semicolon,
            TokenType::Let,
            TokenType::Identifier("array"),
            TokenType::Eq,
            TokenType::Identifier("alloc"),
            TokenType::LParen,
            TokenType::RParen,
            TokenType::Semicolon,
            TokenType::Identifier("array"),
            TokenType::LBrack,
            TokenType::LiteralNumber(NumberLiteral {
                value: 1,
                suffix: None,
            }),
            TokenType::RBrack,
            TokenType::Semicolon,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_should_err_on_number_too_large() {
        let u64_max = format!("{}", (u64::MAX as u128 + 1));
        let input = format!("let x = {};", u64_max);
        let lexer = TokenType::lexer(&input);
        let tokens = lexer.spanned().collect::<Vec<_>>();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].0, Ok(TokenType::Let));
        assert_eq!(tokens[1].0, Ok(TokenType::Identifier("x")));
        assert_eq!(tokens[2].0, Ok(TokenType::Eq));

        // Check that we get a proper LexingError for the oversized number
        match &tokens[3].0 {
            Err(LexingError::InvalidNumber { value, reason }) => {
                assert_eq!(value, &u64_max);
                assert_eq!(reason, &NumberParseError::Overflow);
            }
            _ => panic!(
                "Expected InvalidNumber error for oversized integer, got: {:?}",
                tokens[3].0
            ),
        }

        assert_eq!(tokens[4].0, Ok(TokenType::Semicolon));
    }

    #[test]
    fn test_invalid_number_formats() {
        // Test invalid hexadecimal
        let input = "0xGG";
        let lexer = TokenType::lexer(input);
        let tokens: Vec<_> = lexer.spanned().collect();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].0 {
            Err(LexingError::InvalidNumber { value, reason }) => {
                assert_eq!(value, "0xGG");
                assert_eq!(reason, &NumberParseError::InvalidFormat);
            }
            _ => panic!(
                "Expected InvalidNumber error for invalid hex, got: {:?}",
                tokens[0].0
            ),
        }

        // Test invalid binary
        let input = "0b123";
        let lexer = TokenType::lexer(input);
        let tokens: Vec<_> = lexer.spanned().collect();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].0 {
            Err(LexingError::InvalidNumber { value, reason }) => {
                assert_eq!(value, "0b123");
                assert_eq!(reason, &NumberParseError::InvalidFormat);
            }
            _ => panic!(
                "Expected InvalidNumber error for invalid binary, got: {:?}",
                tokens[0].0
            ),
        }

        // Test invalid octal
        let input = "0o89";
        let lexer = TokenType::lexer(input);
        let tokens: Vec<_> = lexer.spanned().collect();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].0 {
            Err(LexingError::InvalidNumber { value, reason }) => {
                assert_eq!(value, "0o89");
                assert_eq!(reason, &NumberParseError::InvalidFormat);
            }
            _ => panic!(
                "Expected InvalidNumber error for invalid octal, got: {:?}",
                tokens[0].0
            ),
        }
    }

    #[test]
    fn test_valid_number_formats() {
        // Test all valid number formats
        let test_cases = vec![
            ("42", 42),
            ("0x2A", 42),
            ("0o52", 42),
            ("0b101010", 42),
            ("0", 0),
            ("0x0", 0),
            ("0o0", 0),
            ("0b0", 0),
            ("2147483647", 2147483647), // Max value (2^31 - 1)
        ];

        for (input, expected) in test_cases {
            let lexer = TokenType::lexer(input);
            let tokens: Vec<_> = lexer.spanned().collect();
            assert_eq!(tokens.len(), 1, "Input: {input}");
            match &tokens[0].0 {
                Ok(TokenType::LiteralNumber(literal)) => {
                    assert_eq!(literal.value, expected, "Input: {input}");
                    assert_eq!(literal.suffix, None, "Input: {input}");
                }
                other => panic!("Expected LiteralNumber for input '{input}', got: {other:?}"),
            }
        }
    }

    #[test]
    fn test_bitwise_operators() {
        // Test that bitwise operators are recognized correctly
        let input = "a & b | c ^ d";
        let lexer = TokenType::lexer(input);
        let tokens: Vec<_> = lexer.spanned().map(|(t, _)| t).collect();

        assert_eq!(
            tokens,
            vec![
                Ok(TokenType::Identifier("a")),
                Ok(TokenType::BitwiseAnd),
                Ok(TokenType::Identifier("b")),
                Ok(TokenType::BitwiseOr),
                Ok(TokenType::Identifier("c")),
                Ok(TokenType::BitwiseXor),
                Ok(TokenType::Identifier("d")),
            ]
        );
    }

    #[test]
    fn test_logical_vs_bitwise_operators() {
        // Test that logical operators are still recognized correctly and distinct from bitwise
        let input = "a && b || c & d | e";
        let lexer = TokenType::lexer(input);
        let tokens: Vec<_> = lexer.spanned().map(|(t, _)| t).collect();

        assert_eq!(
            tokens,
            vec![
                Ok(TokenType::Identifier("a")),
                Ok(TokenType::AndAnd),
                Ok(TokenType::Identifier("b")),
                Ok(TokenType::OrOr),
                Ok(TokenType::Identifier("c")),
                Ok(TokenType::BitwiseAnd),
                Ok(TokenType::Identifier("d")),
                Ok(TokenType::BitwiseOr),
                Ok(TokenType::Identifier("e")),
            ]
        );
    }

    #[test]
    fn test_number_suffixes() {
        // Test numbers with type suffixes
        let test_cases = vec![
            ("42felt", 42, Some("felt")),
            ("100u32", 100, Some("u32")),
            ("0xFFfelt", 255, Some("felt")),
            ("0b1010u32", 10, Some("u32")),
            ("0o77felt", 63, Some("felt")),
        ];

        for (input, expected_value, expected_suffix) in test_cases {
            let lexer = TokenType::lexer(input);
            let tokens: Vec<_> = lexer.spanned().collect();
            assert_eq!(tokens.len(), 1, "Input: {input}");
            match &tokens[0].0 {
                Ok(TokenType::LiteralNumber(literal)) => {
                    assert_eq!(literal.value, expected_value, "Input: {input}");
                    assert_eq!(literal.suffix, expected_suffix, "Input: {input}");
                }
                other => panic!("Expected LiteralNumber for input '{input}', got: {other:?}"),
            }
        }
    }
}
