use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    source: String,
    file_name: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file_name: String, source: String) -> Self {
        Self {
            tokens,
            current: 0,
            source,
            file_name,
        }
    }

    fn match_token(&mut self, token_type: crate::lexer::TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            return true;
        }
        false
    }

    fn check(&mut self, token_type: crate::lexer::TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().token_type == token_type
    }

    fn is_at_end(&mut self) -> bool {
        self.peek().token_type == crate::lexer::TokenType::EOF
    }

    fn advance(&mut self) -> crate::lexer::Token {
        self.current += 1;
        self.tokens[self.current - 1].clone()
    }

    fn peek(&mut self) -> crate::lexer::Token {
        self.tokens[self.current].clone()
    }

    fn peekpeek(&mut self) -> crate::lexer::Token {
        self.tokens[self.current + 1].clone()
    }

    fn consume(&mut self, token_type: crate::lexer::TokenType, message: &str) -> Token {
        if self.check(token_type) {
            self.advance()
        } else {
            let span = self.peek().span;
            crate::error::report_error(
                self.file_name.clone(),
                self.source.clone(),
                span,
                "Syntax error".to_string(),
                message.to_string(),
            );
            Token {
                token_type: crate::lexer::TokenType::Error,
                lexeme: "".to_string(),
                span: (span.0, span.0),
            }
        }
    }

    pub fn parse(&mut self) -> Vec<CodeElement> {
        let mut code_elements = Vec::new();
        while !self.is_at_end() {
            code_elements.push(self.code_element());
        }
        code_elements
    }

    fn type_(&mut self) -> Type {
        self.pointer()
    }

    fn named_type(&mut self) -> Type {
        if self.peek().token_type == crate::lexer::TokenType::Identifier {
            if self.peekpeek().token_type == crate::lexer::TokenType::Colon {
                let ident = self.identifier();
                self.consume(crate::lexer::TokenType::Colon, "");
                let type_ = self.type_();
                Type::Named(ident, Box::new(type_))
            } else {
                self.pointer()
            }
        } else {
            self.pointer()
        }
    }

    fn pointer(&mut self) -> Type {
        let type_ = self.type_atom();
        if self.check(crate::lexer::TokenType::Star) {
            self.advance();
            Type::Pointer(Box::new(type_))
        } else if self.check(crate::lexer::TokenType::DoubleStar) {
            self.advance();
            Type::Pointer2(Box::new(type_))
        } else {
            type_
        }
    }

    fn paren_type_list(&mut self) -> Vec<Type> {
        let mut args = Vec::new();
        self.consume(crate::lexer::TokenType::LParen, "Expected '('");
        while !self.check(crate::lexer::TokenType::RParen) {
            args.push(self.named_type());
            if !self.check(crate::lexer::TokenType::Comma) {
                break;
            }
            self.advance();
        }
        self.consume(crate::lexer::TokenType::RParen, "Expected ')'");
        args
    }

    fn type_atom(&mut self) -> Type {
        let token = self.peek();
        match token.token_type {
            crate::lexer::TokenType::Felt => {
                self.advance();
                Type::Felt
            }
            crate::lexer::TokenType::CodeOffset => {
                self.advance();
                Type::CodeOffset
            }
            crate::lexer::TokenType::Identifier => Type::Struct(self.identifier()),
            crate::lexer::TokenType::LParen => {
                let types = self.paren_type_list();
                Type::Tuple(types)
            }
            _ => {
                crate::error::report_error(
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

    fn identifier(&mut self) -> Identifier {
        let token = self.consume(crate::lexer::TokenType::Identifier, "Expected identifier");
        Identifier { token }
    }

    fn expression(&mut self) -> Expr {
        self.sum()
    }

    fn expr_assignment(&mut self) -> ExprAssignment {
        let expr = self.expression();
        if let ExprType::Identifier = expr.expr_type {
            let ident = expr.ident.clone().unwrap();
            if self.check(crate::lexer::TokenType::Equal) {
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

    fn arglist(&mut self) -> Vec<ExprAssignment> {
        let mut args = Vec::new();
        while !self.check(crate::lexer::TokenType::RParen) {
            args.push(self.expr_assignment());
            if !self.check(crate::lexer::TokenType::Comma) {
                break;
            }
            self.advance();
        }
        args
    }

    fn paren_arglist(&mut self) -> Vec<ExprAssignment> {
        self.consume(crate::lexer::TokenType::LParen, "Expected '('");
        let args = self.arglist();
        self.consume(crate::lexer::TokenType::RParen, "Expected ')'");
        args
    }

    fn brace_arglist(&mut self) -> Vec<ExprAssignment> {
        self.consume(crate::lexer::TokenType::LBrace, "Expected '{'");
        let args = self.arglist();
        self.consume(crate::lexer::TokenType::RBrace, "Expected '}'");
        args
    }

    fn sum(&mut self) -> Expr {
        let mut expr = self.product();
        while self.check(crate::lexer::TokenType::Plus)
            || self.check(crate::lexer::TokenType::Minus)
        {
            let operator = self.advance();
            let right = self.product();
            match operator.token_type {
                crate::lexer::TokenType::Plus => {
                    expr = Expr::new_binary(ExprType::Add, expr, right);
                }
                crate::lexer::TokenType::Minus => {
                    expr = Expr::new_binary(ExprType::Sub, expr, right);
                }
                _ => unreachable!(),
            }
        }
        expr
    }

    fn product(&mut self) -> Expr {
        let mut expr = self.unary();
        while self.check(crate::lexer::TokenType::Star)
            || self.check(crate::lexer::TokenType::Slash)
        {
            let operator = self.advance();
            let right = self.expression();
            match operator.token_type {
                crate::lexer::TokenType::Star => {
                    expr = Expr::new_binary(ExprType::Mul, expr, right);
                }
                crate::lexer::TokenType::Slash => {
                    expr = Expr::new_binary(ExprType::Div, expr, right);
                }
                _ => unreachable!(),
            }
        }
        expr
    }

    fn unary(&mut self) -> Expr {
        let next = self.peek();
        match next.token_type {
            crate::lexer::TokenType::Ampersand => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::AddressOf, right)
            }
            crate::lexer::TokenType::Minus => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::Neg, right)
            }
            crate::lexer::TokenType::New => {
                self.advance();
                let right = self.unary();
                Expr::new_unary(ExprType::New, right)
            }
            _ => self.pow(),
        }
    }

    fn pow(&mut self) -> Expr {
        let mut expr = self.bool_and();
        while self.check(crate::lexer::TokenType::DoubleStar) {
            self.advance();
            let right = self.expression();
            expr = Expr::new_binary(ExprType::Pow, expr, right);
        }
        expr
    }

    fn bool_and(&mut self) -> Expr {
        let mut expr = self.bool_atom();
        while self.check(crate::lexer::TokenType::And) {
            self.advance();
            let right = self.bool_atom();
            expr = Expr::new_binary(ExprType::And, expr, right);
        }
        expr
    }

    fn bool_atom(&mut self) -> Expr {
        let expr = self.atom();
        let op = self.peek();
        match op.token_type {
            crate::lexer::TokenType::DoubleEq => {
                self.advance();
                let right = self.atom();
                Expr::new_binary(ExprType::Eq, expr, right)
            }
            crate::lexer::TokenType::Neq => {
                self.advance();
                let right = self.atom();
                Expr::new_binary(ExprType::Neq, expr, right)
            }
            _ => expr,
        }
    }

    fn atom(&mut self) -> Expr {
        let token = self.peek();
        if self.check(crate::lexer::TokenType::LParen) {
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
                crate::lexer::TokenType::Int => Expr::new_terminal(ExprType::IntegerLiteral, token),
                crate::lexer::TokenType::Identifier => {
                    if self.check(crate::lexer::TokenType::LBrace) {
                        let brace_args = self.brace_arglist();
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args, brace_args)
                    } else if self.check(crate::lexer::TokenType::LParen) {
                        let paren_args = self.paren_arglist();
                        Expr::new_function_call(Identifier { token }, paren_args, vec![])
                    } else if self.check(crate::lexer::TokenType::LBracket) {
                        self.advance();
                        let expr = self.expression();
                        self.consume(
                            crate::lexer::TokenType::RBracket,
                            "Expected ']' after expression",
                        );
                        Expr::new_unary(ExprType::Subscript, expr)
                    } else {
                        Expr::new_identifier(Identifier { token })
                    }
                }
                crate::lexer::TokenType::HexInt => {
                    Expr::new_terminal(ExprType::IntegerLiteral, token)
                }
                crate::lexer::TokenType::ShortString => {
                    Expr::new_terminal(ExprType::IntegerLiteral, token)
                }
                crate::lexer::TokenType::NonDet => {
                    let hint = self.consume(
                        crate::lexer::TokenType::NonDet,
                        "Expected hint after nondet",
                    );
                    Expr::new_terminal(ExprType::Hint, hint)
                }
                crate::lexer::TokenType::Ap | crate::lexer::TokenType::Fp => {
                    Expr::new_terminal(ExprType::Register, token)
                }
                crate::lexer::TokenType::LBracket => {
                    let expr = self.expression();
                    self.consume(
                        crate::lexer::TokenType::RBracket,
                        "Expected ']' after dereferencing",
                    );
                    Expr::new_unary(ExprType::Deref, expr)
                }

                crate::lexer::TokenType::Cast => {
                    self.consume(crate::lexer::TokenType::LParen, "Expected '(' after cast");
                    let expr = self.expression();
                    self.consume(crate::lexer::TokenType::Comma, "Expected ','");
                    let type_ = self.type_();
                    println!("type_ is :{:?}", type_.clone());
                    self.consume(crate::lexer::TokenType::RParen, "Expected ')'");
                    Expr::new_cast(type_, expr)
                }
                _ => {
                    crate::error::report_error(
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

    fn does_increment_ap(&mut self) -> bool {
        let old_current = self.current;
        if self.match_token(crate::lexer::TokenType::Comma)
            && self.match_token(crate::lexer::TokenType::Ap)
            && self.match_token(crate::lexer::TokenType::PlusPlus)
        {
            true
        } else {
            self.current = old_current;
            false
        }
    }

    fn instruction(&mut self) -> Instruction {
        if self.match_token(crate::lexer::TokenType::Call) {
            if self.match_token(crate::lexer::TokenType::Rel) {
                Instruction::new_unary(
                    InstructionType::CallRel,
                    self.expression(),
                    self.does_increment_ap(),
                )
            } else if self.match_token(crate::lexer::TokenType::Abs) {
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
        } else if self.match_token(crate::lexer::TokenType::Jmp) {
            if self.match_token(crate::lexer::TokenType::Rel) {
                let expr = self.expression();
                if self.match_token(crate::lexer::TokenType::If) {
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
            } else if self.match_token(crate::lexer::TokenType::Abs) {
                let expr = self.expression();
                // jump abs
                Instruction::new_unary(InstructionType::JmpAbs, expr, self.does_increment_ap())
            } else {
                let ident = self.identifier();
                if self.match_token(crate::lexer::TokenType::If) {
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
        } else if self.match_token(crate::lexer::TokenType::Ret) {
            Instruction::new_ret(self.does_increment_ap())
        } else if self.match_token(crate::lexer::TokenType::Ap) {
            self.consume(crate::lexer::TokenType::PlusEq, "Expected '+=' after ap");
            Instruction::new_unary(
                InstructionType::AddAp,
                self.expression(),
                self.does_increment_ap(),
            )
        } else if self.match_token(crate::lexer::TokenType::Dw) {
            Instruction::new_unary(
                InstructionType::DataWord,
                self.expression(),
                self.does_increment_ap(),
            )
        } else {
            let left = self.expression();
            self.consume(crate::lexer::TokenType::Equal, "Expected '=' in assertion");
            let right = self.expression();
            Instruction::new_binary(
                InstructionType::AssertEq,
                left,
                right,
                self.does_increment_ap(),
            )
        }
    }

    fn identifier_list_paren(&mut self) -> Vec<Identifier> {
        let mut identifiers = Vec::new();
        self.consume(crate::lexer::TokenType::LParen, "Expected '('");
        while !self.check(crate::lexer::TokenType::RParen) {
            identifiers.push(self.identifier());
            if !self.check(crate::lexer::TokenType::Comma) {
                break;
            }
            self.advance();
        }
        self.consume(crate::lexer::TokenType::RParen, "Expected ')'");
        identifiers
    }

    fn code_element(&mut self) -> CodeElement {
        let token = self.peek();
        match token.token_type {
            crate::lexer::TokenType::If => {
                self.advance();
                self.consume(crate::lexer::TokenType::LParen, "Expected '(' after if");
                let cond = self.expression();
                self.consume(crate::lexer::TokenType::RParen, "Expected ')' after if");
                self.consume(crate::lexer::TokenType::LBrace, "Expected '{' after if");
                let mut body = Vec::new();
                while !self.check(crate::lexer::TokenType::RBrace) {
                    body.push(self.code_element());
                }
                self.consume(crate::lexer::TokenType::RBrace, "Expected '}' after if");
                if self.match_token(crate::lexer::TokenType::Else) {
                    self.consume(crate::lexer::TokenType::LBrace, "Expected '{' after else");
                    let mut else_body = Vec::new();
                    while !self.check(crate::lexer::TokenType::RBrace) {
                        else_body.push(self.code_element());
                    }
                    self.consume(crate::lexer::TokenType::RBrace, "Expected '}' after else");
                    CodeElement::If(cond, body, else_body)
                } else {
                    CodeElement::If(cond, body, vec![])
                }
            }

            crate::lexer::TokenType::Func => {
                self.advance();
                let ident = self.identifier();

                let args = self.identifier_list_paren();

                self.consume(
                    crate::lexer::TokenType::LBrace,
                    "Expected '{' after function",
                );
                let mut body = Vec::new();
                while !self.check(crate::lexer::TokenType::RBrace) {
                    body.push(self.code_element());
                }
                self.consume(
                    crate::lexer::TokenType::RBrace,
                    "Expected '}' after function",
                );
                CodeElement::Function(ident, args, body)
            }

            crate::lexer::TokenType::Local => {
                self.advance();
                let ident = self.identifier();
                if self.match_token(crate::lexer::TokenType::Equal) {
                    let expr = self.expression();
                    self.consume(
                        crate::lexer::TokenType::Semicolon,
                        "Expected ';' after local",
                    );
                    CodeElement::LocalVar(ident, Some(expr))
                } else {
                    self.consume(
                        crate::lexer::TokenType::Semicolon,
                        "Expected ';' after local",
                    );
                    CodeElement::LocalVar(ident, None)
                }
            }

            crate::lexer::TokenType::Assert => {
                self.advance();
                let left = self.expression();
                self.consume(crate::lexer::TokenType::Equal, "Expected '=' after assert");
                let right = self.expression();
                self.consume(
                    crate::lexer::TokenType::Semicolon,
                    "Expected ';' after static assert",
                );
                CodeElement::CompoundAssertEqual(left, right)
            }

            crate::lexer::TokenType::Return => {
                self.advance();
                let expr = self.expression();
                self.consume(
                    crate::lexer::TokenType::Semicolon,
                    "Expected ';' after return",
                );
                CodeElement::Return(expr)
            }

            crate::lexer::TokenType::AllocLocals => {
                self.advance();
                self.consume(
                    crate::lexer::TokenType::Semicolon,
                    "Expected ';' after alloc_locals",
                );
                CodeElement::AllocLocals
            }

            _ => {
                let instr = self.instruction();
                self.consume(
                    crate::lexer::TokenType::Semicolon,
                    "Expected ';' after instruction",
                );
                CodeElement::Instruction(instr)
            }
        }
    }
}
