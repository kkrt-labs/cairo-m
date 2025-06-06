// crates/cairo-m-compiler-parser/src/tokens.rs
use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[\t\n\r ]+")] // Skip whitespace, including carriage return
#[logos(skip r"//[^\n]*")] // Skip single-line comments
pub enum TokenType {
    // Literals
    #[regex(r"[0-9]+|0x[0-9a-fA-F]+|0o[0-7]+|0b[01]+")]
    LiteralNumber,
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
    Identifier,
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
            TokenType::Identifier,
            TokenType::LParen,
            TokenType::Identifier,
            TokenType::Colon,
            TokenType::Identifier,
            TokenType::Comma,
            TokenType::Identifier,
            TokenType::Colon,
            TokenType::Identifier,
            TokenType::RParen,
            TokenType::Arrow,
            TokenType::Identifier,
            TokenType::LBrace,
            TokenType::Let,
            TokenType::Identifier,
            TokenType::Eq,
            TokenType::Identifier,
            TokenType::Plus,
            TokenType::Identifier,
            TokenType::Semicolon,
            TokenType::If,
            TokenType::Identifier,
            TokenType::Identifier,
            TokenType::LiteralNumber,
            TokenType::LBrace,
            TokenType::Return,
            TokenType::Identifier,
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::Else,
            TokenType::LBrace,
            TokenType::Return,
            TokenType::LiteralNumber,
            TokenType::Semicolon,
            TokenType::RBrace,
            TokenType::RBrace,
            TokenType::Let,
            TokenType::Identifier,
            TokenType::Eq,
            TokenType::Identifier,
            TokenType::LParen,
            TokenType::LiteralNumber,
            TokenType::Comma,
            TokenType::LiteralNumber,
            TokenType::RParen,
            TokenType::Semicolon,
            TokenType::Const,
            TokenType::Identifier,
            TokenType::Eq,
            TokenType::LiteralNumber,
            TokenType::Semicolon,
            TokenType::Let,
            TokenType::Identifier,
            TokenType::Eq,
            TokenType::Identifier,
            TokenType::LParen,
            TokenType::RParen,
            TokenType::Semicolon,
            TokenType::Identifier,
            TokenType::LBrack,
            TokenType::LiteralNumber,
            TokenType::RBrack,
            TokenType::Semicolon,
        ];

        assert_eq!(tokens, expected);
    }
}
