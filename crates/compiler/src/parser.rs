//! Cairo-M Parser Module
//!
//! This module implements a recursive descent parser for the Cairo-M language.
//! The parser converts a stream of tokens into an Abstract Syntax Tree (AST)
//! that represents the program's structure.
//!
//! The parser follows Cairo's syntax rules and supports:
//! - Function definitions and calls
//! - Control flow statements (if/else)
//! - Local variable declarations
//! - Arithmetic and logical expressions
//! - Pointer operations
//! - Type annotations
//! - Assertions and returns
//!
//! The parser is currently incomplete and does not support all features of Cairo-0.

use crate::ast::*;
use crate::error::report_error;
use crate::lexer::*;

/// A recursive descent parser for Cairo-M source code.
///
/// The parser maintains its current position in the token stream and provides
/// methods for parsing different language constructs. It handles error reporting
/// and builds an AST representation of the program.
pub struct Parser {
    /// The complete list of tokens to parse
    tokens: Vec<Token>,
    /// Current position in the token stream
    current: usize,
    /// Original source code for error reporting
    source: String,
    /// Name of the file being parsed for error reporting
    file_name: String,
}

impl Parser {
    /// Creates a new parser instance with the given tokens and source information.
    ///
    /// # Arguments
    /// * `tokens` - Vector of tokens to parse
    /// * `file_name` - Name of the source file
    /// * `source` - Original source code
    pub fn new(tokens: Vec<Token>, file_name: String, source: String) -> Self {
        Self {
            tokens,
            current: 0,
            source,
            file_name,
        }
    }

    /// Attempts to match and consume the current token if it matches the expected type.
    /// Returns true if the token was matched and consumed, false otherwise.
    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            return true;
        }
        false
    }

    /// Checks if the current token matches the expected type without consuming it.
    fn check(&mut self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().token_type == token_type
    }

    /// Returns true if we've reached the end of the token stream.
    fn is_at_end(&mut self) -> bool {
        self.peek().token_type == TokenType::EOF
    }

    /// Advances the parser to the next token and returns the consumed token.
    fn advance(&mut self) -> Token {
        self.current += 1;
        self.tokens[self.current - 1].clone()
    }

    /// Returns the current token without consuming it.
    fn peek(&mut self) -> Token {
        self.tokens[self.current].clone()
    }

    /// Consumes the current token if it matches the expected type, otherwise reports an error.
    ///
    /// # Arguments
    /// * `token_type` - The expected token type
    /// * `message` - Error message to display if the token doesn't match
    fn consume(&mut self, token_type: TokenType, message: &str) -> Token {
        if self.check(token_type) {
            self.advance()
        } else {
            let span = self.peek().span;
            report_error(
                self.file_name.clone(),
                self.source.clone(),
                span,
                "Syntax error".to_string(),
                message.to_string(),
            );
            Token {
                token_type: TokenType::Error,
                lexeme: "".to_string(),
                span: (span.0, span.0),
            }
        }
    }

    /// Parses the entire token stream into a vector of code elements.
    ///
    /// This is the main entry point for parsing a complete program.
    pub fn parse(&mut self) -> Vec<CodeElement> {
        let mut code_elements = Vec::new();
        while !self.is_at_end() {
            code_elements.push(self.code_element());
        }
        code_elements
    }

    /// Parses an identifier token and returns an Identifier struct.
    fn identifier(&mut self) -> Identifier {
        let token = self.consume(TokenType::Identifier, "Expected identifier");
        Identifier { token }
    }

    /// Parses an expression, starting with the lowest precedence level (sum).
    fn expression(&mut self) -> Expr {
        self.sum()
    }

    /// Parses a list of expressions separated by commas.
    fn arglist(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while !self.check(TokenType::RParen) {
            args.push(self.expression());
            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }
        args
    }

    /// Parses a parenthesized list of expressions.
    fn paren_arglist(&mut self) -> Vec<Expr> {
        self.consume(TokenType::LParen, "Expected '('");
        let args = self.arglist();
        self.consume(TokenType::RParen, "Expected ')'");
        args
    }

    /// Parses a sum expression (addition and subtraction).
    /// Handles operator precedence for + and - operators.
    fn sum(&mut self) -> Expr {
        let mut expr = self.product();
        while self.check(TokenType::Plus) || self.check(TokenType::Minus) {
            let operator = self.advance();
            let right = self.product();
            match operator.token_type {
                TokenType::Plus => {
                    expr = Expr::new_binary(ExprType::Add, expr, right);
                }
                TokenType::Minus => {
                    expr = Expr::new_binary(ExprType::Sub, expr, right);
                }
                _ => unreachable!(),
            }
        }
        expr
    }

    /// Parses a product expression (multiplication and division).
    /// Handles operator precedence for * and / operators.
    fn product(&mut self) -> Expr {
        let mut expr = self.unary();
        while self.check(TokenType::Star) || self.check(TokenType::Slash) {
            let operator = self.advance();
            let right = self.expression();
            match operator.token_type {
                TokenType::Star => {
                    expr = Expr::new_binary(ExprType::Mul, expr, right);
                }
                TokenType::Slash => {
                    expr = Expr::new_binary(ExprType::Div, expr, right);
                }
                _ => unreachable!(),
            }
        }
        expr
    }

    /// Parses a unary expression (address-of, negation, or new).
    fn unary(&mut self) -> Expr {
        let next = self.peek();
        match next.token_type {
            TokenType::Minus => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::Neg, right)
            }
            _ => self.bool_and(),
        }
    }

    /// Parses a boolean AND expression.
    fn bool_and(&mut self) -> Expr {
        let mut expr = self.bool_atom();
        while self.check(TokenType::And) {
            self.advance();
            let right = self.bool_atom();
            expr = Expr::new_binary(ExprType::And, expr, right);
        }
        expr
    }

    /// Parses a boolean atom (equality or inequality comparison).
    fn bool_atom(&mut self) -> Expr {
        let expr = self.atom();
        let op = self.peek();
        match op.token_type {
            TokenType::DoubleEq => {
                self.advance();
                let right = self.atom();
                Expr::new_binary(ExprType::Eq, expr, right)
            }
            TokenType::Neq => {
                self.advance();
                let right = self.atom();
                Expr::new_binary(ExprType::Neq, expr, right)
            }
            _ => expr,
        }
    }

    /// Parses an atomic expression, which can be:
    /// - A parenthesized expression
    /// - A literal (integer, hex, string)
    /// - An identifier
    /// - A function call
    /// - A register reference (ap/fp)
    /// - A dereference operation
    /// - A cast operation
    fn atom(&mut self) -> Expr {
        let token = self.peek();
        if self.check(TokenType::LParen) {
            let args = self.paren_arglist();
            // in the case of a single expression, we return it directly
            if args.len() == 1 {
                return args[0].clone();
            }
            Expr::new_tuple_or_paren(args)
        } else {
            self.advance();
            match token.token_type {
                TokenType::Int => Expr::new_terminal(ExprType::IntegerLiteral, token),
                TokenType::Identifier => {
                    if self.check(TokenType::LBrace) {
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args)
                    } else if self.check(TokenType::LParen) {
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args)
                    } else {
                        Expr::new_identifier(Identifier { token })
                    }
                }
                _ => {
                    report_error(
                        self.file_name.clone(),
                        self.source.clone(),
                        token.span,
                        "Syntax error".to_string(),
                        format!("Expected expression, got {:?}", token.lexeme),
                    );
                    Expr::new_error()
                }
            }
        }
    }

    /// Parses a list of identifiers enclosed in parentheses.
    fn identifier_list_paren(&mut self) -> Vec<Identifier> {
        let mut identifiers = Vec::new();
        self.consume(TokenType::LParen, "Expected '('");
        while !self.check(TokenType::RParen) {
            identifiers.push(self.identifier());
            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }
        self.consume(TokenType::RParen, "Expected ')'");
        identifiers
    }

    /// Parses a code element, which can be:
    /// - An if statement
    /// - A function definition
    /// - A local variable declaration
    /// - An assertion
    /// - A return statement
    /// - An alloc_locals directive
    /// - An instruction
    fn code_element(&mut self) -> CodeElement {
        let token = self.peek();
        match token.token_type {
            TokenType::If => {
                self.advance();
                self.consume(TokenType::LParen, "Expected '(' after if");
                let cond = self.expression();
                self.consume(TokenType::RParen, "Expected ')' after if");
                self.consume(TokenType::LBrace, "Expected '{' after if");
                let mut body = Vec::new();
                while !self.check(TokenType::RBrace) {
                    body.push(self.code_element());
                }
                self.consume(TokenType::RBrace, "Expected '}' after if");
                if self.match_token(TokenType::Else) {
                    self.consume(TokenType::LBrace, "Expected '{' after else");
                    let mut else_body = Vec::new();
                    while !self.check(TokenType::RBrace) {
                        else_body.push(self.code_element());
                    }
                    self.consume(TokenType::RBrace, "Expected '}' after else");
                    CodeElement::If(cond, body, else_body)
                } else {
                    CodeElement::If(cond, body, vec![])
                }
            }

            TokenType::Func => {
                self.advance();
                let ident = self.identifier();

                let args = self.identifier_list_paren();

                self.consume(TokenType::LBrace, "Expected '{' after function");
                let mut body = Vec::new();
                while !self.check(TokenType::RBrace) {
                    body.push(self.code_element());
                }
                self.consume(TokenType::RBrace, "Expected '}' after function");
                CodeElement::Function(ident, args, body)
            }

            TokenType::Local => {
                self.advance();
                let ident = self.identifier();
                if self.match_token(TokenType::Equal) {
                    let expr = self.expression();
                    self.consume(TokenType::Semicolon, "Expected ';' after local");
                    CodeElement::LocalVar(ident, Some(expr))
                } else {
                    self.consume(TokenType::Semicolon, "Expected ';' after local");
                    CodeElement::LocalVar(ident, None)
                }
            }

            TokenType::Return => {
                self.advance();
                let expr = self.expression();
                self.consume(TokenType::Semicolon, "Expected ';' after return");
                CodeElement::Return(expr)
            }

            _ => {
                let left = self.identifier();
                self.consume(TokenType::Equal, "Expected '=' in assignment");
                let right = self.expression();
                self.consume(TokenType::Semicolon, "Expected ';' after assignment");
                CodeElement::Assign(left, right)
            }
        }
    }
}
