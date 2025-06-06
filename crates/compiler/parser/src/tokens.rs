// crates/cairo-m-compiler-parser/src/tokens.rs
use logos::Logos;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lexer() {
        let input = r#"
        func add(x: felt, y: felt) -> felt {
            let result = x + y;
            if result > 0 {
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
        for (token, span) in lexer.spanned() {
            match token {
                Ok(token) => tokens.push(token),
                Err(e) => {
                    println!("lexer error at {:?}: {:?}", span, e);
                    return;
                }
            }
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
            TokenType::Identifier("result"),
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
}
