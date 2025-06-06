// crates/cairo-m-compiler-parser/src/tokens.rs
use logos::Logos;
use std::fmt;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[\t\n\r ]+")] // Skip whitespace, including carriage return
#[logos(skip r"//[^\n]*")] // Skip single-line comments
pub enum TokenType<'a> {
    // Literals
    #[regex(r"[0-9]+|0x[0-9a-fA-F]+|0o[0-7]+|0b[01]+", |lex| {
        lex.slice().parse::<u32>().ok().and_then(|n| {
            if n >= 0x80000000 {
                None
            } else {
                Some(n)
            }
        })
    })]
    LiteralNumber(u32),
    // Keywords
    #[token("as")]
    As,
    #[token("const")]
    Const,
    #[token("else")]
    Else,
    #[token("false")]
    False,
    #[token("from")]
    From,
    #[token("func")]
    Function,
    #[token("if")]
    If,
    #[token("import")]
    Import,
    #[token("let")]
    Let,
    #[token("local")]
    Local,
    #[token("namespace")]
    Namespace,
    #[token("return")]
    Return,
    #[token("struct")]
    Struct,
    #[token("true")]
    True,
    // Identifiers (must come after keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier(&'a str),
    // Operators (order matters for longest match)
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Neq,
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
    #[token(".")]
    Dot,

    Error,
}

impl<'a> fmt::Display for TokenType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::LiteralNumber(n) => write!(f, "{n}"),
            TokenType::Identifier(s) => write!(f, "{s}"),
            TokenType::As => write!(f, "as"),
            TokenType::Const => write!(f, "const"),
            TokenType::Else => write!(f, "else"),
            TokenType::False => write!(f, "false"),
            TokenType::From => write!(f, "from"),
            TokenType::Function => write!(f, "func"),
            TokenType::If => write!(f, "if"),
            TokenType::Import => write!(f, "import"),
            TokenType::Let => write!(f, "let"),
            TokenType::Local => write!(f, "local"),
            TokenType::Namespace => write!(f, "namespace"),
            TokenType::Return => write!(f, "return"),
            TokenType::Struct => write!(f, "struct"),
            TokenType::True => write!(f, "true"),
            TokenType::AndAnd => write!(f, "&&"),
            TokenType::OrOr => write!(f, "||"),
            TokenType::EqEq => write!(f, "=="),
            TokenType::Neq => write!(f, "!="),
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
            TokenType::Dot => write!(f, "."),
            TokenType::Error => write!(f, "Error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lexer() {
        let input = r#"
            func add(x: felt, y: felt) -> felt {
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
            TokenType::LiteralNumber(0),
            TokenType::LBrace,
            TokenType::Return,
            TokenType::Identifier("result"),
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::Else,
            TokenType::LBrace,
            TokenType::Return,
            TokenType::LiteralNumber(0),
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::RBrace,
            TokenType::Let,
            TokenType::Identifier("value"),
            TokenType::Eq,
            TokenType::Identifier("add"),
            TokenType::LParen,
            TokenType::LiteralNumber(10),
            TokenType::Comma,
            TokenType::LiteralNumber(20),
            TokenType::RParen,
            TokenType::Semicolon,
            TokenType::Const,
            TokenType::Identifier("MAX_SIZE"),
            TokenType::Eq,
            TokenType::LiteralNumber(100),
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
            TokenType::LiteralNumber(1),
            TokenType::RBrack,
            TokenType::Semicolon,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_should_err_on_number_too_large() {
        let input = "let x = 0x80000000;";
        let lexer = TokenType::lexer(input);
        let tokens = lexer.spanned().collect::<Vec<_>>();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].0, Ok(TokenType::Let));
        assert_eq!(tokens[1].0, Ok(TokenType::Identifier("x")));
        assert_eq!(tokens[2].0, Ok(TokenType::Eq));
        assert_eq!(tokens[3].0, Err(()));
        assert_eq!(tokens[4].0, Ok(TokenType::Semicolon));
    }
}
