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

    /// Returns the next token without consuming it.
    fn peekpeek(&mut self) -> Token {
        self.tokens[self.current + 1].clone()
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

    /// Parses a type expression, which can be a pointer type or a base type.
    fn type_(&mut self) -> Type {
        self.pointer()
    }

    /// Parses a named type, which can be an identifier followed by a colon and type,
    /// or just a pointer type.
    fn named_type(&mut self) -> Type {
        if self.peek().token_type == TokenType::Identifier {
            if self.peekpeek().token_type == TokenType::Colon {
                let ident = self.identifier();
                self.consume(TokenType::Colon, "");
                let type_ = self.type_();
                Type::Named(ident, Box::new(type_))
            } else {
                self.pointer()
            }
        } else {
            self.pointer()
        }
    }

    /// Parses a pointer type, which can be a base type followed by one or two asterisks.
    fn pointer(&mut self) -> Type {
        let type_ = self.type_atom();
        if self.check(TokenType::Star) {
            self.advance();
            Type::Pointer2(Box::new(type_))
        } else if self.check(TokenType::Star) {
            self.advance();
            Type::Pointer(Box::new(type_))
        } else {
            type_
        }
    }

    /// Parses a list of types enclosed in parentheses, separated by commas.
    fn paren_type_list(&mut self) -> Vec<Type> {
        let mut args = Vec::new();
        self.consume(TokenType::LParen, "Expected '('");
        while !self.check(TokenType::RParen) {
            args.push(self.named_type());
            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }
        self.consume(TokenType::RParen, "Expected ')'");
        args
    }

    /// Parses an atomic type, which can be:
    /// - A felt type
    /// - A code offset type
    /// - A struct type (identifier)
    /// - A tuple type (parenthesized list of types)
    fn type_atom(&mut self) -> Type {
        let token = self.peek();
        match token.token_type {
            TokenType::Felt => {
                self.advance();
                Type::Felt
            }
            TokenType::CodeOffset => {
                self.advance();
                Type::CodeOffset
            }
            TokenType::Identifier => Type::Struct(self.identifier()),
            TokenType::LParen => {
                let types = self.paren_type_list();
                Type::Tuple(types)
            }
            _ => {
                report_error(
                    self.file_name.clone(),
                    self.source.clone(),
                    self.peek().span,
                    "Syntax error".to_string(),
                    format!("Expected type, got {:?}", self.peek().lexeme),
                );
                Type::Error
            }
        }
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

    /// Parses an assignment expression or a regular expression.
    /// Handles both variable assignments and regular expressions.
    fn expr_assignment(&mut self) -> ExprAssignment {
        let expr = self.expression();
        if let ExprType::Identifier = expr.expr_type {
            let ident = expr.ident.clone().unwrap();
            if self.check(TokenType::Equal) {
                self.advance();
                let expr = self.expression();
                ExprAssignment::Assign(ident.clone(), expr)
            } else {
                ExprAssignment::Expr(expr)
            }
        } else {
            ExprAssignment::Expr(expr)
        }
    }

    /// Parses a list of expressions separated by commas.
    fn arglist(&mut self) -> Vec<ExprAssignment> {
        let mut args = Vec::new();
        while !self.check(TokenType::RParen) {
            args.push(self.expr_assignment());
            if !self.check(TokenType::Comma) {
                break;
            }
            self.advance();
        }
        args
    }

    /// Parses a parenthesized list of expressions.
    fn paren_arglist(&mut self) -> Vec<ExprAssignment> {
        self.consume(TokenType::LParen, "Expected '('");
        let args = self.arglist();
        self.consume(TokenType::RParen, "Expected ')'");
        args
    }

    /// Parses a brace-enclosed list of expressions.
    fn brace_arglist(&mut self) -> Vec<ExprAssignment> {
        self.consume(TokenType::LBrace, "Expected '{'");
        let args = self.arglist();
        self.consume(TokenType::RBrace, "Expected '}'");
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
            TokenType::Ampersand => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::AddressOf, right)
            }
            TokenType::Minus => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::Neg, right)
            }
            TokenType::New => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::New, right)
            }
            _ => self.pow(),
        }
    }

    /// Parses a power expression (exponentiation).
    fn pow(&mut self) -> Expr {
        let mut expr = self.bool_and();
        while self.check(TokenType::DoubleStar) {
            self.advance();
            let right = self.expression();
            expr = Expr::new_binary(ExprType::Pow, expr, right);
        }
        expr
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
                match args[0].clone() {
                    ExprAssignment::Expr(expr) => {
                        return expr;
                    }
                    _ => {}
                }
            }
            Expr::new_tuple_or_paren(args)
        } else {
            self.advance();
            match token.token_type {
                TokenType::Int => Expr::new_terminal(ExprType::IntegerLiteral, token),
                TokenType::Identifier => {
                    if self.check(TokenType::LBrace) {
                        let brace_args = self.brace_arglist();
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args, brace_args)
                    } else if self.check(TokenType::LParen) {
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args, vec![])
                    } else if self.check(TokenType::LBracket) {
                        self.advance();
                        let expr = self.expression();
                        self.consume(TokenType::RBracket, "Expected ']' after expression");
                        Expr::new_unary(ExprType::Subscript, expr)
                    } else {
                        Expr::new_identifier(Identifier { token })
                    }
                }
                TokenType::HexInt => Expr::new_terminal(ExprType::IntegerLiteral, token),
                TokenType::ShortString => Expr::new_terminal(ExprType::IntegerLiteral, token),
                TokenType::NonDet => {
                    let hint = self.consume(TokenType::NonDet, "Expected hint after nondet");
                    Expr::new_terminal(ExprType::Hint, hint)
                }
                TokenType::Ap | TokenType::Fp => Expr::new_terminal(ExprType::Register, token),
                TokenType::LBracket => {
                    let expr = self.expression();
                    self.consume(TokenType::RBracket, "Expected ']' after dereferencing");
                    Expr::new_unary(ExprType::Deref, expr)
                }

                TokenType::Cast => {
                    self.consume(TokenType::LParen, "Expected '(' after cast");
                    let expr = self.expression();
                    self.consume(TokenType::Comma, "Expected ','");
                    let type_ = self.type_();
                    println!("type_ is :{:?}", type_.clone());
                    self.consume(TokenType::RParen, "Expected ')'");
                    Expr::new_cast(type_, expr)
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

    /// Checks if the current token sequence indicates an AP increment operation.
    fn does_increment_ap(&mut self) -> bool {
        let old_current = self.current;
        if self.match_token(TokenType::Comma)
            && self.match_token(TokenType::Ap)
            && self.match_token(TokenType::PlusPlus)
        {
            true
        } else {
            self.current = old_current;
            false
        }
    }

    /// Parses a Cairo-M instruction, which can be:
    /// - Call instructions (call, call_rel, call_abs)
    /// - Jump instructions (jmp, jmp_rel, jmp_abs, jnz)
    /// - Return instructions
    /// - AP manipulation
    /// - Data word declarations
    /// - Assertions
    fn instruction(&mut self) -> Instruction {
        if self.match_token(TokenType::Call) {
            if self.match_token(TokenType::Rel) {
                Instruction::new_unary(
                    InstructionType::CallRel,
                    self.expression(),
                    self.does_increment_ap(),
                )
            } else if self.match_token(TokenType::Abs) {
                Instruction::new_unary(
                    InstructionType::CallAbs,
                    self.expression(),
                    self.does_increment_ap(),
                )
            } else {
                Instruction::new_call(
                    InstructionType::Call,
                    self.identifier(),
                    self.does_increment_ap(),
                )
            }
        } else if self.match_token(TokenType::Jmp) {
            if self.match_token(TokenType::Rel) {
                let expr = self.expression();
                if self.match_token(TokenType::If) {
                    let condition = self.expression();
                    // jump rel if
                    Instruction::new_binary(
                        InstructionType::Jnz,
                        expr,
                        condition,
                        self.does_increment_ap(),
                    )
                } else {
                    // jump rel
                    Instruction::new_unary(InstructionType::JmpRel, expr, self.does_increment_ap())
                }
            } else if self.match_token(TokenType::Abs) {
                let expr = self.expression();
                // jump abs
                Instruction::new_unary(InstructionType::JmpAbs, expr, self.does_increment_ap())
            } else {
                let ident = self.identifier();
                if self.match_token(TokenType::If) {
                    let condition = self.expression();
                    // jump if
                    Instruction::new_jmp_label_if(
                        InstructionType::JnzLabel,
                        ident,
                        condition,
                        self.does_increment_ap(),
                    )
                } else {
                    // jump
                    Instruction::new_jmp_label(
                        InstructionType::Jmp,
                        ident,
                        self.does_increment_ap(),
                    )
                }
            }
        } else if self.match_token(TokenType::Ret) {
            Instruction::new_ret(self.does_increment_ap())
        } else if self.match_token(TokenType::Ap) {
            self.consume(TokenType::PlusEq, "Expected '+=' after ap");
            Instruction::new_unary(
                InstructionType::AddAp,
                self.expression(),
                self.does_increment_ap(),
            )
        } else if self.match_token(TokenType::Dw) {
            Instruction::new_unary(
                InstructionType::DataWord,
                self.expression(),
                self.does_increment_ap(),
            )
        } else {
            let left = self.expression();
            self.consume(TokenType::Equal, "Expected '=' in assertion");
            let right = self.expression();
            Instruction::new_binary(
                InstructionType::AssertEq,
                left,
                right,
                self.does_increment_ap(),
            )
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

            TokenType::Assert => {
                self.advance();
                let left = self.expression();
                self.consume(TokenType::Equal, "Expected '=' after assert");
                let right = self.expression();
                self.consume(TokenType::Semicolon, "Expected ';' after static assert");
                CodeElement::CompoundAssertEqual(left, right)
            }

            TokenType::Return => {
                self.advance();
                let expr = self.expression();
                self.consume(TokenType::Semicolon, "Expected ';' after return");
                CodeElement::Return(expr)
            }

            TokenType::AllocLocals => {
                self.advance();
                self.consume(TokenType::Semicolon, "Expected ';' after alloc_locals");
                CodeElement::AllocLocals
            }

            _ => {
                let instr = self.instruction();
                self.consume(TokenType::Semicolon, "Expected ';' after instruction");
                CodeElement::Instruction(instr)
            }
        }
    }
}
