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
    Hint,
    Identifier,
    Register,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Neg,
    Deref,
    AddressOf,
    Cast,
    New,
    Eq,
    Neq,
    And,
    FunctionCall,
    Subscript,
    TupleOrParen,
    ErrorExpr,
}

#[derive(Clone)]
pub struct Expr {
    pub token: Option<Token>,
    pub ident: Option<Identifier>,
    pub expr_type: ExprType,
    pub left: Option<Box<Expr>>,
    pub right: Option<Box<Expr>>,
    pub type_arg: Option<Type>,
    pub paren_args: Vec<ExprAssignment>,
    pub brace_args: Vec<ExprAssignment>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub token: Token,
}

#[derive(Clone)]
pub enum ExprAssignment {
    Expr(Expr),
    Assign(Identifier, Expr),
}

// Instructions

#[derive(Clone)]
pub struct Instruction {
    pub instruction_type: InstructionType,
    pub ident: Option<Identifier>,
    pub args: Vec<Expr>,
    pub increment_ap: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionType {
    CallRel,
    CallAbs,
    Call,
    AssertEq,
    JmpRel,
    JmpAbs,
    Jmp,
    Jnz,
    JnzLabel,
    Ret,
    AddAp,
    DataWord,
}

#[derive(Clone)]
pub enum CodeElement {
    Instruction(Instruction),
    Const,
    Reference(Identifier, Expr),
    LocalVar(Identifier, Option<Expr>),
    TempVar,
    CompoundAssertEqual(Expr, Expr),
    StaticAssert,
    Return(Expr),
    If(Expr, Vec<CodeElement>, Vec<CodeElement>),
    FuncCall,
    Label,
    Function(Identifier, Vec<Identifier>, Vec<CodeElement>),
    Struct,
    NameSpace,
    TypeDef,
    WithAttr,
    With,
    Hint,
    Directive,
    Import,
    AllocLocals,
}

impl Expr {
    pub fn new_error() -> Self {
        Self {
            token: None,
            ident: None,
            expr_type: ExprType::ErrorExpr,
            left: None,
            right: None,
            type_arg: None,
            paren_args: vec![],
            brace_args: vec![],
        }
    }
    pub fn new_identifier(ident: Identifier) -> Self {
        Self {
            token: None,
            ident: Some(ident),
            expr_type: ExprType::Identifier,
            left: None,
            right: None,
            type_arg: None,
            paren_args: vec![],
            brace_args: vec![],
        }
    }

    pub fn new_terminal(expr_type: ExprType, token: Token) -> Self {
        Self {
            token: Some(token),
            ident: None,
            expr_type,
            left: None,
            right: None,
            type_arg: None,
            paren_args: vec![],
            brace_args: vec![],
        }
    }

    pub fn new_unary(expr_type: ExprType, child: Expr) -> Self {
        Self {
            token: None,
            ident: None,
            expr_type,
            left: Some(Box::new(child)),
            right: None,
            type_arg: None,
            paren_args: vec![],
            brace_args: vec![],
        }
    }

    pub fn new_binary(expr_type: ExprType, left: Expr, right: Expr) -> Self {
        Self {
            token: None,
            ident: None,
            expr_type,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            type_arg: None,
            paren_args: vec![],
            brace_args: vec![],
        }
    }

    pub fn new_function_call(
        name: Identifier,
        paren_args: Vec<ExprAssignment>,
        brace_args: Vec<ExprAssignment>,
    ) -> Self {
        Self {
            token: None,
            ident: Some(name),
            expr_type: ExprType::FunctionCall,
            left: None,
            right: None,
            type_arg: None,
            paren_args,
            brace_args,
        }
    }

    pub fn new_tuple_or_paren(args: Vec<ExprAssignment>) -> Self {
        Self {
            token: None,
            ident: None,
            expr_type: ExprType::TupleOrParen,
            left: None,
            right: None,
            type_arg: None,
            paren_args: args,
            brace_args: vec![],
        }
    }

    pub fn new_cast(type_arg: Type, child: Expr) -> Self {
        Self {
            token: None,
            ident: None,
            expr_type: ExprType::Cast,
            left: Some(Box::new(child)),
            right: None,
            type_arg: Some(type_arg),
            paren_args: vec![],
            brace_args: vec![],
        }
    }
}

impl Instruction {
    pub fn new_unary(instruction_type: InstructionType, child: Expr, increment_ap: bool) -> Self {
        Self {
            instruction_type,
            ident: None,
            args: vec![child],
            increment_ap,
        }
    }

    pub fn new_binary(
        instruction_type: InstructionType,
        left: Expr,
        right: Expr,
        increment_ap: bool,
    ) -> Self {
        Self {
            instruction_type,
            ident: None,
            args: vec![left, right],
            increment_ap,
        }
    }

    pub fn new_call(
        instruction_type: InstructionType,
        ident: Identifier,
        increment_ap: bool,
    ) -> Self {
        Self {
            instruction_type,
            ident: Some(ident),
            args: vec![],
            increment_ap,
        }
    }

    pub fn new_ret(increment_ap: bool) -> Self {
        Self {
            instruction_type: InstructionType::Ret,
            ident: None,
            args: vec![],
            increment_ap,
        }
    }

    pub fn new_jmp_label(
        instruction_type: InstructionType,
        ident: Identifier,
        increment_ap: bool,
    ) -> Self {
        Self {
            instruction_type,
            ident: Some(ident),
            args: vec![],
            increment_ap,
        }
    }

    pub fn new_jmp_label_if(
        instruction_type: InstructionType,
        ident: Identifier,
        condition: Expr,
        increment_ap: bool,
    ) -> Self {
        Self {
            instruction_type,
            ident: Some(ident),
            args: vec![condition],
            increment_ap,
        }
    }
}

impl Type {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}", "", indent = indent * 2)?;
        match self {
            Type::Felt => write!(f, "Felt"),
            Type::CodeOffset => write!(f, "CodeOffset"),
            Type::Pointer(inner) => {
                write!(f, "Pointer")?;
                writeln!(f)?;
                inner.fmt_with_indent(f, indent + 1)
            }
            Type::Pointer2(inner) => {
                write!(f, "Pointer2")?;
                writeln!(f)?;
                inner.fmt_with_indent(f, indent + 1)
            }
            Type::Tuple(types) => {
                write!(f, "Tuple")?;
                writeln!(f)?;
                for (i, t) in types.iter().enumerate() {
                    t.fmt_with_indent(f, indent + 1)?;
                    if i < types.len() - 1 {
                        writeln!(f)?;
                    }
                }
                Ok(())
            }
            Type::Struct(ident) => {
                write!(f, "Struct '{}'", ident.token.lexeme)
            }
            Type::Named(ident, inner) => {
                write!(f, "Named '{}'", ident.token.lexeme)?;
                writeln!(f)?;
                inner.fmt_with_indent(f, indent + 1)
            }
            Type::Error => write!(f, "Error"),
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl Debug for ExprAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl ExprAssignment {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        match self {
            ExprAssignment::Expr(expr) => expr.fmt_with_indent(f, indent),
            ExprAssignment::Assign(ident, expr) => {
                write!(f, "{:indent$}", "", indent = indent * 2)?;
                write!(f, "Assign '{}' = ", ident.token.lexeme)?;
                writeln!(f)?;
                expr.fmt_with_indent(f, indent + 1)
            }
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

        // Print the expression type
        write!(f, "{:?}", self.expr_type)?;

        // Print token if present
        if let Some(token) = &self.token {
            write!(f, " '{}'", token.lexeme)?;
        }

        // Print identifier if present
        if let Some(ident) = &self.ident {
            write!(f, " '{}'", ident.token.lexeme)?;
        }

        let has_children = self.left.is_some()
            || self.right.is_some()
            || !self.paren_args.is_empty()
            || !self.brace_args.is_empty()
            || self.type_arg.is_some();
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

            // Print type argument if present
            if let Some(type_arg) = &self.type_arg {
                type_arg.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
            }

            // Print args if present
            for (i, arg) in self.paren_args.iter().enumerate() {
                arg.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
            }
            for (i, arg) in self.brace_args.iter().enumerate() {
                arg.fmt_with_indent(f, indent + 1)?;
                if i < self.brace_args.len() - 1 {
                    writeln!(f)?;
                }
            }
        }

        Ok(())
    }
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl Instruction {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}", "", indent = indent * 2)?;
        write!(f, "{:?}", self.instruction_type)?;
        if self.increment_ap {
            write!(f, " (ap++)")?;
        }
        writeln!(f)?;

        // Print identifier if present
        if let Some(ident) = &self.ident {
            write!(f, "{:indent$}", "", indent = (indent + 1) * 2)?;
            write!(f, "Identifier '{}'", ident.token.lexeme)?;
            writeln!(f)?;
        }

        // Print arguments
        for (i, arg) in self.args.iter().enumerate() {
            arg.fmt_with_indent(f, indent + 1)?;
            if i < self.args.len() - 1 {
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
            CodeElement::Instruction(instr) => {
                write!(f, "Instruction")?;
                writeln!(f)?;
                instr.fmt_with_indent(f, indent + 1)
            }
            CodeElement::Const => write!(f, "Const"),
            CodeElement::Reference(ident, expr) => {
                write!(f, "Reference '{}' = ", ident.token.lexeme)?;
                writeln!(f)?;
                expr.fmt_with_indent(f, indent + 1)
            }
            CodeElement::LocalVar(ident, expr) => {
                write!(f, "LocalVar '{}' = ", ident.token.lexeme)?;
                writeln!(f)?;
                if let Some(expr) = expr {
                    expr.fmt_with_indent(f, indent + 1)
                } else {
                    Ok(())
                }
            }
            CodeElement::TempVar => write!(f, "TempVar"),
            CodeElement::StaticAssert => write!(f, "StaticAssert"),
            CodeElement::CompoundAssertEqual(left, right) => {
                write!(f, "CompoundAssertEqual")?;
                writeln!(f)?;
                left.fmt_with_indent(f, indent + 1)?;
                writeln!(f)?;
                right.fmt_with_indent(f, indent + 1)
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
            CodeElement::FuncCall => write!(f, "FuncCall"),
            CodeElement::Label => write!(f, "Label"),
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
            CodeElement::Struct => write!(f, "Struct"),
            CodeElement::NameSpace => write!(f, "NameSpace"),
            CodeElement::TypeDef => write!(f, "TypeDef"),
            CodeElement::WithAttr => write!(f, "WithAttr"),
            CodeElement::With => write!(f, "With"),
            CodeElement::Hint => write!(f, "Hint"),
            CodeElement::Directive => write!(f, "Directive"),
            CodeElement::Import => write!(f, "Import"),
            CodeElement::AllocLocals => write!(f, "AllocLocals"),
        }
    }
}
