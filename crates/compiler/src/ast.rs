use crate::lexer::Token;
use std::fmt::{self, Debug};

// Types

#[derive(Clone)]
pub enum Type {
    Felt,
    CodeOffset,
    Pointer(Box<Type>),
    Pointer2(Box<Type>),
    Tuple(Vec<Type>),
    Struct(Identifier),
    Named(Identifier, Box<Type>),
    Error,
}

// Expressions

#[derive(Debug, Clone)]
pub enum ExprType {
    IntegerLiteral,
    Identifier,
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Eq,
    Neq,
    And,
    FunctionCall,
    TupleOrParen,
    ErrorExpr,
}

#[derive(Clone)]
pub struct Expr {
    pub expr_type: ExprType,
    pub token: Option<Token>,
    pub ident: Option<Identifier>,
    pub left: Option<Box<Expr>>,
    pub right: Option<Box<Expr>>,
    pub paren_args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub token: Token,
}

#[derive(Clone)]
pub enum CodeElement {
    LocalVar(Identifier, Option<Expr>),
    Return(Expr),
    If(Expr, Vec<CodeElement>, Vec<CodeElement>),
    Function(Identifier, Vec<Identifier>, Vec<CodeElement>),
    Assign(Identifier, Expr),
}

impl Expr {
    pub fn new_error() -> Self {
        Self {
            expr_type: ExprType::ErrorExpr,
            token: None,
            ident: None,
            left: None,
            right: None,
            paren_args: vec![],
        }
    }
    pub fn new_identifier(ident: Identifier) -> Self {
        Self {
            expr_type: ExprType::Identifier,
            token: None,
            ident: Some(ident),
            left: None,
            right: None,
            paren_args: vec![],
        }
    }

    pub fn new_terminal(expr_type: ExprType, token: Token) -> Self {
        Self {
            expr_type: expr_type,
            token: Some(token),
            ident: None,
            left: None,
            right: None,
            paren_args: vec![],
        }
    }

    pub fn new_unary(expr_type: ExprType, child: Expr) -> Self {
        Self {
            expr_type: expr_type,
            token: None,
            ident: None,
            left: Some(Box::new(child)),
            right: None,
            paren_args: vec![],
        }
    }

    pub fn new_binary(expr_type: ExprType, left: Expr, right: Expr) -> Self {
        Self {
            expr_type: expr_type,
            token: None,
            ident: None,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            paren_args: vec![],
        }
    }

    pub fn new_function_call(name: Identifier, paren_args: Vec<Expr>) -> Self {
        Self {
            expr_type: ExprType::FunctionCall,
            token: None,
            ident: Some(name),
            left: None,
            right: None,
            paren_args,
        }
    }

    pub fn new_tuple_or_paren(args: Vec<Expr>) -> Self {
        Self {
            expr_type: ExprType::TupleOrParen,
            token: None,
            ident: None,
            left: None,
            right: None,
            paren_args: args,
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl Expr {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}", "", indent = indent * 2)?;

        // Print token if present
        if let Some(token) = &self.token {
            write!(f, " '{}'", token.lexeme)?;
        }

        // Print identifier if present
        if let Some(ident) = &self.ident {
            write!(f, " '{}'", ident.token.lexeme)?;
        }

        let has_children =
            self.left.is_some() || self.right.is_some() || !self.paren_args.is_empty();
        if has_children {
            writeln!(f)?;

            // Print left child if present
            if let Some(left) = &self.left {
                left.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
            }

            // Print right child if present
            if let Some(right) = &self.right {
                right.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
            }

            // Print args if present
            for (_i, arg) in self.paren_args.iter().enumerate() {
                arg.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

impl Debug for CodeElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl CodeElement {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}", "", indent = indent * 2)?;
        match self {
            CodeElement::LocalVar(ident, expr) => {
                write!(f, "LocalVar '{}' = ", ident.token.lexeme)?;
                writeln!(f)?;
                if let Some(expr) = expr {
                    expr.fmt_with_indent(f, indent + 1)
                } else {
                    Ok(())
                }
            }
            CodeElement::Return(expr) => {
                write!(f, "Return")?;
                writeln!(f)?;
                expr.fmt_with_indent(f, indent + 1)
            }
            CodeElement::If(cond, body, else_body) => {
                write!(f, "If")?;
                writeln!(f)?;
                write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
                write!(f, "Condition:")?;
                writeln!(f)?;
                cond.fmt_with_indent(f, indent + 2)?;
                writeln!(f)?;
                write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
                write!(f, "Body:")?;
                writeln!(f)?;
                for (i, elem) in body.iter().enumerate() {
                    elem.fmt_with_indent(f, indent + 2)?;
                    if i < body.len() - 1 {
                        writeln!(f)?;
                    }
                }
                if !else_body.is_empty() {
                    writeln!(f)?;
                    write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
                    write!(f, "Else:")?;
                    writeln!(f)?;
                    for (i, elem) in else_body.iter().enumerate() {
                        elem.fmt_with_indent(f, indent + 2)?;
                        if i < else_body.len() - 1 {
                            writeln!(f)?;
                        }
                    }
                }
                Ok(())
            }
            CodeElement::Function(ident, args, body) => {
                write!(f, "Function '{}'", ident.token.lexeme)?;
                writeln!(f)?;
                write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
                write!(f, "Arguments:")?;
                writeln!(f)?;
                for (i, arg) in args.iter().enumerate() {
                    write!(f, "{:indent$}", "", indent = (indent + 2) * 2)?;
                    write!(f, "'{}'", arg.token.lexeme)?;
                    if i < args.len() - 1 {
                        writeln!(f)?;
                    }
                }
                writeln!(f)?;
                write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
                write!(f, "Body:")?;
                writeln!(f)?;
                for (i, elem) in body.iter().enumerate() {
                    elem.fmt_with_indent(f, indent + 2)?;
                    if i < body.len() - 1 {
                        writeln!(f)?;
                    }
                }
                Ok(())
            }
            CodeElement::Assign(ident, expr) => {
                write!(f, "Assign '{}' = ", ident.token.lexeme)?;
                writeln!(f)?;
                expr.fmt_with_indent(f, indent + 1)
            }
        }
    }
}
