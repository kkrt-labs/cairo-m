//! # Cairo-M Parser
//!
//! This module implements a recursive descent parser for the Cairo-M language using the `chumsky`
//! parsing library with Salsa integration for incremental compilation. The parser transforms a
//! stream of tokens into an Abstract Syntax Tree (AST) consisting of top-level items like
//! functions, structs, namespaces, and statements.
//!
//! ## Architecture
//!
//! The parser is built using parser combinators, which are small, composable parsing functions
//! that can be combined to build larger parsers. The main components are:
//!
//! - **Expression parsing**: Handles literals, identifiers, function calls, binary operations, etc.
//! - **Type expression parsing**: Parses type annotations like `felt`, `Vector*`, `(felt, felt)`
//! - **Statement parsing**: Handles control flow, variable declarations, assignments
//! - **Top-level item parsing**: Functions, structs, namespaces, imports, and constants
//!
//! ## Salsa Integration & Incremental Compilation
//!
//! This parser is integrated with [Salsa](https://salsa-rs.github.io/salsa/) to enable
//! incremental compilation and caching of parse results. The integration follows best practices:
//!
//! ### Current Implementation
//! - **Input types**: `SourceProgram` marked with `#[salsa::input]` represents source code
//! - **Cached parsing**: Only the `parse_program` operation is tracked, not individual AST nodes
//! - **Plain AST types**: All AST nodes are regular Rust types for maximum performance
//! - **Database integration**: Parser functions take a `&dyn Db` parameter for the parsing operation
//!
//! ### Caching Behavior
//! Salsa caches the entire parse result. When source code changes, only the parsing operation
//! is re-executed. The resulting AST is stored as a single cached unit, which is much more
//! efficient than tracking individual nodes.

use crate::lexer::TokenType;
use chumsky::{input::ValueInput, prelude::*};
use std::ops::Range;

#[salsa::input(debug)]
pub struct SourceProgram {
    #[returns(ref)]
    pub text: String,
}

/// Represents a type expression in the Cairo-M language.
///
/// Type expressions describe the shape and structure of data, including
/// basic types, pointers, and tuple types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    /// A named type (e.g., `felt`, `Vector`)
    Named(String),
    /// A pointer type (e.g., `felt*`, `Vector*`)
    Pointer(Box<TypeExpr>),
    /// A tuple type (e.g., `(felt, felt)`, `(Vector, felt, bool)`)
    Tuple(Vec<TypeExpr>),
}

/// Binary operators supported in expressions.
///
/// These operators have different precedence levels that are handled
/// during expression parsing to ensure correct operator precedence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum BinaryOp {
    /// Addition operator `+`
    Add,
    /// Subtraction operator `-`
    Sub,
    /// Multiplication operator `*`
    Mul,
    /// Division operator `/`
    Div,
    /// Equality operator `==`
    Eq,
    /// Inequality operator `!=`
    Neq,
    /// Logical AND operator `&&`
    And,
    /// Logical OR operator `||`
    Or,
}

/// Represents an expression in the Cairo-M language.
///
/// Expressions are constructs that evaluate to a value, including literals,
/// variables, function calls, binary operations, and data structure access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    /// Integer literal (e.g., `42`, `0`, `1337`)
    Literal(u32),
    /// Variable identifier (e.g., `x`, `my_var`, `result`)
    Identifier(Spanned<String>),
    /// Binary operation (e.g., `a + b`, `x == y`, `p && q`)
    BinaryOp {
        op: BinaryOp,
        left: Box<Spanned<Expression>>,
        right: Box<Spanned<Expression>>,
    },
    /// Function call (e.g., `foo()`, `add(x, y)`)
    FunctionCall {
        callee: Box<Spanned<Expression>>,
        args: Vec<Spanned<Expression>>,
    },
    /// Member access (e.g., `obj.field`, `vector.x`)
    MemberAccess {
        object: Box<Spanned<Expression>>,
        field: Spanned<String>,
    },
    /// Array/collection indexing (e.g., `arr[0]`, `matrix[i][j]`)
    IndexAccess {
        array: Box<Spanned<Expression>>,
        index: Box<Spanned<Expression>>,
    },
    /// Struct literal (e.g., `Point { x: 1, y: 2 }`)
    StructLiteral {
        name: Spanned<String>,
        fields: Vec<(Spanned<String>, Spanned<Expression>)>,
    },
    /// Tuple literal (e.g., `(1, 2, 3)`, `(x, y)`)
    Tuple(Vec<Spanned<Expression>>),
}

/// Represents a function parameter with its name and type.
///
/// Used in function definitions to specify the expected arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Parameter {
    /// The parameter name
    pub name: Spanned<String>,
    /// The parameter's type
    pub type_expr: TypeExpr,
}

/// Represents a statement in the Cairo-M language.
///
/// Statements are constructs that perform actions but don't necessarily
/// evaluate to a value. They form the body of functions and control flow.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// Global variable declaration (e.g., `let x = 5;`)
    Let {
        name: Spanned<String>,
        statement_type: Option<TypeExpr>,
        value: Spanned<Expression>,
    },
    /// Local variable declaration with optional type annotation (e.g., `local x: felt = 5;`)
    Local {
        name: Spanned<String>,
        ty: Option<TypeExpr>,
        value: Spanned<Expression>,
    },
    /// Constant declaration (e.g., `const PI = 314;`)
    Const(ConstDef),
    /// Assignment to an existing variable (e.g., `x = new_value;`)
    Assignment {
        lhs: Spanned<Expression>,
        rhs: Spanned<Expression>,
    },
    /// Return statement (e.g., `return x;`, `return;`)
    Return { value: Option<Spanned<Expression>> },
    /// Conditional statement (e.g., `if (condition) { ... } else { ... }`)
    If {
        condition: Spanned<Expression>,
        then_block: Box<Spanned<Statement>>,
        else_block: Option<Box<Spanned<Statement>>>,
    },
    /// Expression used as a statement (e.g., `foo();`)
    Expression(Spanned<Expression>),
    /// Block of statements (e.g., `{ stmt1; stmt2; stmt3; }`)
    Block(Vec<Spanned<Statement>>),
}

/// Represents a top-level item in a Cairo-M program.
///
/// These are the constructs that can appear at the module level,
/// outside of any function or namespace body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TopLevelItem {
    /// Function definition
    Function(Spanned<FunctionDef>),
    /// Struct definition
    Struct(Spanned<StructDef>),
    /// Namespace definition
    Namespace(Spanned<Namespace>),
    /// Import statement
    Import(Spanned<ImportStmt>),
    /// Constant definition
    Const(Spanned<ConstDef>),
}

/// Represents a constant definition.
///
/// Constants are immutable values that are defined once and can be
/// referenced throughout the program.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstDef {
    /// The constant's name
    pub name: Spanned<String>,
    /// The constant's value expression
    pub value: Spanned<Expression>,
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub struct Spanned<T>(T, SimpleSpan<usize>);

impl<T> Spanned<T> {
    /// Create a new spanned value
    pub const fn new(value: T, span: SimpleSpan<usize>) -> Self {
        Self(value, span)
    }

    /// Get the inner value
    pub const fn value(&self) -> &T {
        &self.0
    }

    /// Get the span
    pub const fn span(&self) -> SimpleSpan<usize> {
        self.1
    }

    /// Destructure into value and span
    pub fn into_parts(self) -> (T, SimpleSpan<usize>) {
        (self.0, self.1)
    }
}

/// Represents a function definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDef {
    /// The function's name
    pub name: Spanned<String>,
    /// The function's parameters
    pub params: Vec<Parameter>,
    /// The function's return type (optional)
    pub return_type: Option<TypeExpr>,
    /// The function's body (list of statements)
    pub body: Vec<Spanned<Statement>>,
}

/// Represents a struct definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDef {
    /// The struct's name
    pub name: Spanned<String>,
    /// The struct's fields (name and type pairs)
    pub fields: Vec<(Spanned<String>, TypeExpr)>,
}

/// Represents a namespace definition.
///
/// Namespaces provide a way to organize related functions, types,
/// and constants under a common name, preventing naming conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespace {
    /// The namespace's name
    pub name: Spanned<String>,
    /// The items contained within the namespace
    pub body: Vec<TopLevelItem>,
}

/// Represents an import statement.
///
/// Import statements allow code to reference items from other modules
/// or namespaces, with optional aliasing for name resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportStmt {
    /// The path to the module (e.g., `["std", "math"]` for `std.math`)
    pub path: Vec<Spanned<String>>,
    /// The specific item being imported
    pub item: Spanned<String>,
    /// Optional alias for the imported item
    pub alias: Option<Spanned<String>>,
}

/// Wrapper for the parsed AST result.
///
/// This follows the Salsa best practice of caching the entire parse result
/// rather than individual AST nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModule {
    /// The top-level items in the module
    pub items: Vec<TopLevelItem>,
}

impl ParsedModule {
    pub const fn new(items: Vec<TopLevelItem>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[TopLevelItem] {
        &self.items
    }
}

/// Parse a source program into a module AST.
///
/// This is the main Salsa-tracked parsing function. It caches the entire
/// parse result, following best practices from the Ruff codebase.
#[salsa::tracked(returns(ref), no_eq)]
pub fn parse_program(db: &dyn crate::Db, source: SourceProgram) -> ParsedModule {
    use logos::Logos;
    let input = source.text(db);

    // Collect tokens and handle lexer errors
    let mut tokens = Vec::new();
    let mut lexer_errors = Vec::new();

    for (token_result, span) in TokenType::lexer(input).spanned() {
        match token_result {
            Ok(token) => tokens.push((token, span.into())),
            Err(_lexing_error) => {
                // For now, we'll skip lexer errors and handle them later
                // In a full implementation, you'd want to report these
                lexer_errors.push(span);
            }
        }
    }

    // If there are lexer errors, return empty module
    if !lexer_errors.is_empty() {
        return ParsedModule::new(vec![]);
    }

    // Create token stream from the successfully lexed tokens
    let token_stream = chumsky::input::Stream::from_iter(tokens)
        .map((0..input.len()).into(), |(t, s): (_, _)| (t, s));

    // Parse using the parser combinator
    match parser()
        .then_ignore(end())
        .parse(token_stream)
        .into_result()
    {
        Ok(items) => ParsedModule::new(items),
        Err(_parse_errors) => {
            // For now, return empty module on parse errors
            // In a full implementation, you'd want to report these
            ParsedModule::new(vec![])
        }
    }
}

/// Helper enum for handling postfix operations during expression parsing.
///
/// This is used internally by the parser to handle chained operations
/// like `obj.field().index[0]` in a left-associative manner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PostfixOp {
    /// Function call with arguments
    Call(Vec<Spanned<Expression>>),
    /// Member access with field name
    Member(Spanned<String>),
    /// Index access with index expression
    Index(Spanned<Expression>),
}

// ===================
// Parser Implementation
// ===================

/// Creates an identifier parser that extracts string content from Identifier tokens
fn ident_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, String, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    select! { TokenType::Identifier(s) => s.to_string() }.labelled("identifier")
}

/// Creates a spanned identifier parser that captures both the identifier and its span
fn spanned_ident_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<String>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    select! { TokenType::Identifier(s) => s.to_string() }
        .map_with(|s, extra| Spanned::new(s, extra.span()))
        .labelled("identifier")
}

/// Creates a parser for type expressions (named types, pointers, tuples)
fn type_expr_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, TypeExpr, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let ident = ident_parser();

    recursive(|type_expr| {
        // Named types: felt, Vector, MyStruct, etc.
        let named_type = ident.map(TypeExpr::Named);

        // Tuple types: (felt, felt), (Vector, bool, felt), etc.
        let tuple_type = type_expr
            .separated_by(just(TokenType::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(TokenType::LParen), just(TokenType::RParen))
            .map(|types| {
                // Single type in parens is just a parenthesized type
                if types.len() == 1 {
                    types.into_iter().next().unwrap()
                } else {
                    // Multiple types form a tuple type
                    TypeExpr::Tuple(types)
                }
            });

        let base_type = named_type.or(tuple_type);

        // Handle pointer types: felt*, Vector**, etc. (right-associative via foldl)
        base_type.foldl(just(TokenType::Mul).repeated(), |ty, _| {
            TypeExpr::Pointer(Box::new(ty))
        })
    })
}

/// Creates a parser for expressions with proper operator precedence
fn expression_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<Expression>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();

    recursive(|expr| {
        // Atomic expressions (cannot be broken down further)

        // Integer literals (e.g., 42, 0, 1337)
        let literal = select! { TokenType::LiteralNumber(n) => Expression::Literal(n) }
            .map_with(|lit, extra| Spanned::new(lit, extra.span()));

        // Variable identifiers (e.g., x, my_var, result)
        let ident_expr = spanned_ident
            .clone()
            .map(Expression::Identifier)
            .map_with(|expr, extra| Spanned::new(expr, extra.span()));

        // Struct literal field parsing: "field_name: expression"
        let struct_literal_fields = spanned_ident
            .clone()
            .then_ignore(just(TokenType::Colon)) // field name, then ignore ':'
            .then(expr.clone()) // followed by the field value
            .separated_by(just(TokenType::Comma)) // fields separated by commas
            .allow_trailing() // allow trailing comma
            .collect::<Vec<_>>()
            .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace)); // wrapped in {}

        // Struct literals: "StructName { field1: value1, field2: value2 }"
        let struct_literal = spanned_ident
            .clone()
            .then(struct_literal_fields)
            .map(|(name, fields)| Expression::StructLiteral { name, fields })
            .map_with(|expr, extra| Spanned::new(expr, extra.span()));

        // Tuple expressions and parenthesized expressions: "(a, b, c)" or "(expr)"
        let tuple_expr = expr
            .clone()
            .separated_by(just(TokenType::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(TokenType::LParen), just(TokenType::RParen))
            .map(|exprs| {
                // Single element in parens is just a parenthesized expression
                if exprs.len() == 1 {
                    exprs.into_iter().next().unwrap().value().clone()
                } else {
                    // Multiple elements form a tuple
                    Expression::Tuple(exprs)
                }
            })
            .map_with(|expr, extra| Spanned::new(expr, extra.span()));

        // Basic atomic expressions - try each alternative in order
        let atom = literal
            .or(struct_literal)
            .or(ident_expr)
            .or(tuple_expr)
            .or(expr
                .clone()
                .delimited_by(just(TokenType::LParen), just(TokenType::RParen)));

        // Postfix operations (left-associative): function calls, member access, indexing
        let postfix_op = choice((
            // Function call: "expr(arg1, arg2, ...)"
            expr.clone()
                .separated_by(just(TokenType::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(TokenType::LParen), just(TokenType::RParen))
                .map(PostfixOp::Call),
            // Member access: "expr.field"
            just(TokenType::Dot)
                .ignore_then(spanned_ident.clone())
                .map(PostfixOp::Member),
            // Index access: "expr[index]"
            expr.clone()
                .delimited_by(just(TokenType::LBrack), just(TokenType::RBrack))
                .map(PostfixOp::Index),
        ));

        // Apply postfix operations left-to-right: expr.field().index[0]
        let call = atom.foldl(postfix_op.repeated(), |expr, op| match op {
            PostfixOp::Call(args) => {
                let span_callee = expr.span();
                let max_range: Range<usize> = args.iter().map(|arg| arg.span().into()).fold(
                    span_callee.into(),
                    |acc: Range<usize>, range: Range<usize>| acc.start..range.end.max(acc.end),
                );
                let span = SimpleSpan::from(span_callee.start..max_range.end); // Span from start of callee to end of args
                Spanned::new(
                    Expression::FunctionCall {
                        callee: Box::new(expr),
                        args,
                    },
                    span,
                )
            }
            PostfixOp::Member(field) => {
                let span_obj = expr.span();
                let span_field = field.span();
                let span = SimpleSpan::from(span_obj.start..span_field.end); // Span from start of object to end of field
                Spanned::new(
                    Expression::MemberAccess {
                        object: Box::new(expr),
                        field,
                    },
                    span,
                )
            }
            PostfixOp::Index(index) => {
                let span_obj = expr.span();
                let span_index = index.span();
                let span = SimpleSpan::from(span_obj.start..span_index.end); // Span from start of object to end of index
                Spanned::new(
                    Expression::IndexAccess {
                        array: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                )
            }
        });

        // Helper to create binary operator parsers
        let op = |token, op| just(token).to(op);

        // Multiplicative operators: *, / (left-associative)
        let mul = call.clone().foldl(
            choice((
                op(TokenType::Mul, BinaryOp::Mul),
                op(TokenType::Div, BinaryOp::Div),
            ))
            .then(call.clone())
            .repeated(),
            |lhs, (op, rhs)| {
                let span_lhs = lhs.span();
                let span_rhs = rhs.span();
                let span = SimpleSpan::from(span_lhs.start..span_rhs.end);
                Spanned::new(
                    Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    span,
                )
            },
        );

        // Additive operators: +, - (left-associative)
        let add = mul.clone().foldl(
            choice((
                op(TokenType::Plus, BinaryOp::Add),
                op(TokenType::Minus, BinaryOp::Sub),
            ))
            .then(mul.clone())
            .repeated(),
            |lhs, (op, rhs)| {
                let span_lhs = lhs.span();
                let span_rhs = rhs.span();
                let span = SimpleSpan::from(span_lhs.start..span_rhs.end);
                Spanned::new(
                    Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    span,
                )
            },
        );

        // Comparison operators: ==, != (left-associative)
        let cmp = add.clone().foldl(
            choice((
                op(TokenType::EqEq, BinaryOp::Eq),
                op(TokenType::Neq, BinaryOp::Neq),
            ))
            .then(add.clone())
            .repeated(),
            |lhs, (op, rhs)| {
                let span_lhs = lhs.span();
                let span_rhs = rhs.span();
                let span = SimpleSpan::from(span_lhs.start..span_rhs.end);
                Spanned::new(
                    Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    span,
                )
            },
        );

        // Logical AND operator: && (left-associative)
        let and = cmp.clone().foldl(
            op(TokenType::AndAnd, BinaryOp::And)
                .then(cmp.clone())
                .repeated(),
            |lhs, (op, rhs)| {
                let span_lhs = lhs.span();
                let span_rhs = rhs.span();
                let span = SimpleSpan::from(span_lhs.start..span_rhs.end);
                Spanned::new(
                    Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    span,
                )
            },
        );

        // Logical OR operator: || (left-associative, lowest precedence)
        and.clone().foldl(
            op(TokenType::OrOr, BinaryOp::Or)
                .then(and.clone())
                .repeated(),
            |lhs, (op, rhs)| {
                let span_lhs = lhs.span();
                let span_rhs = rhs.span();
                let span = SimpleSpan::from(span_lhs.start..span_rhs.end);
                Spanned::new(
                    Expression::BinaryOp {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    span,
                )
            },
        )
    })
}

/// Creates a parser for function parameters
fn parameter_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Parameter, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let type_expr = type_expr_parser();

    // Function parameter: name: type
    spanned_ident
        .then_ignore(just(TokenType::Colon)) // parameter name, ignore ':'
        .then(type_expr) // parameter type
        .map(|(name, type_expr)| Parameter { name, type_expr })
}

/// Creates a parser for statements
fn statement_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<Statement>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let expr = expression_parser();
    let type_expr = type_expr_parser();

    recursive(|statement| {
        // Block statement: { stmt1; stmt2; stmt3; }
        let block = statement
            .clone()
            .repeated()
            .collect::<Vec<Spanned<Statement>>>()
            .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace))
            .map(Statement::Block)
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Let statement: let variable (: type)? = expression;
        let let_stmt = just(TokenType::Let)
            .ignore_then(spanned_ident.clone()) // variable name
            .then(
                just(TokenType::Colon)
                    .ignore_then(type_expr.clone()) // optional type annotation
                    .or_not(),
            )
            .then_ignore(just(TokenType::Eq)) // ignore '='
            .then(expr.clone()) // value expression
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|((name, statement_type), value)| Statement::Let {
                name,
                statement_type,
                value,
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Local statement: local variable: type = expression;
        let local_stmt = just(TokenType::Local)
            .ignore_then(spanned_ident.clone()) // variable name
            .then(
                just(TokenType::Colon)
                    .ignore_then(type_expr.clone()) // optional type annotation
                    .or_not(),
            )
            .then_ignore(just(TokenType::Eq)) // ignore '='
            .then(expr.clone()) // value expression
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|((name, ty), value)| Statement::Local { name, ty, value })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Const statement: const NAME = expression;
        let const_stmt = just(TokenType::Const)
            .ignore_then(spanned_ident.clone()) // constant name
            .then_ignore(just(TokenType::Eq)) // ignore '='
            .then(expr.clone()) // value expression
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|(name, value)| Statement::Const(ConstDef { name, value }))
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // If statement: if (condition) then_stmt else else_stmt
        let if_stmt = just(TokenType::If)
            .ignore_then(
                expr.clone()
                    .delimited_by(just(TokenType::LParen), just(TokenType::RParen)), // condition in parens
            )
            .then(statement.clone()) // then block
            .then(
                just(TokenType::Else)
                    .ignore_then(statement.clone()) // optional else block
                    .or_not(),
            )
            .map(|((condition, then_block), else_block)| Statement::If {
                condition,
                then_block: Box::new(then_block),
                else_block: else_block.map(Box::new),
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Return statement: return expression; or return;
        let return_stmt = just(TokenType::Return)
            .ignore_then(expr.clone().or_not()) // optional return value
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|value| Statement::Return { value })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Assignment or expression statement: lhs = rhs; or expr;
        let assignment_or_expr = expr
            .clone()
            .then(just(TokenType::Eq).ignore_then(expr.clone()).or_not()) // optional assignment
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|(lhs, rhs)| match rhs {
                Some(rhs) => Statement::Assignment { lhs, rhs },
                None => Statement::Expression(lhs),
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Try statement alternatives in order
        block
            .or(if_stmt)
            .or(let_stmt)
            .or(local_stmt)
            .or(const_stmt)
            .or(return_stmt)
            .or(assignment_or_expr)
    })
}

/// Creates a parser for function definitions
fn function_def_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<FunctionDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let param = parameter_parser();
    let type_expr = type_expr_parser();
    let statement = statement_parser();

    // Function definition: func name(param1: type1, param2: type2) -> return_type { body }
    just(TokenType::Function)
        .ignore_then(spanned_ident) // function name
        .then(
            param
                .separated_by(just(TokenType::Comma)) // parameters separated by commas
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(TokenType::LParen), just(TokenType::RParen)), // wrapped in ()
        )
        .then(
            just(TokenType::Arrow)
                .ignore_then(type_expr) // optional return type after ->
                .or_not(),
        )
        .then(
            statement
                .repeated()
                .collect::<Vec<Spanned<Statement>>>()
                .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace)), // body in {}
        )
        .map_with(|(((name, params), return_type), body), extra| {
            Spanned(
                FunctionDef {
                    name,
                    params,
                    return_type,
                    body,
                },
                extra.span(),
            )
        })
}

/// Creates a parser for struct definitions
fn struct_def_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<StructDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let type_expr = type_expr_parser();

    // Struct field: name: type
    let struct_field = spanned_ident
        .clone()
        .then_ignore(just(TokenType::Colon)) // field name, ignore ':'
        .then(type_expr); // field type

    // Struct definition: struct Name { field1: type1, field2: type2 }
    just(TokenType::Struct)
        .ignore_then(spanned_ident) // struct name
        .then(
            struct_field
                .separated_by(just(TokenType::Comma)) // fields separated by commas
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace)), // wrapped in {}
        )
        .map_with(|(name, fields), extra| Spanned(StructDef { name, fields }, extra.span()))
}

/// Creates a parser for import statements
fn import_stmt_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<ImportStmt>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();

    // Import statement: from path.to.module import item as alias
    just(TokenType::From)
        .ignore_then(
            spanned_ident
                .clone()
                .separated_by(just(TokenType::Dot)) // module path separated by dots
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(TokenType::Import)) // ignore 'import' keyword
        .then(spanned_ident.clone()) // imported item name
        .then(just(TokenType::As).ignore_then(spanned_ident).or_not()) // optional alias
        .map_with(|((path, item), alias), extra| {
            Spanned(ImportStmt { path, item, alias }, extra.span())
        })
}

/// Creates a parser for constant definitions
fn const_def_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<ConstDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let expr = expression_parser();

    // Constant definition: const NAME = expression;
    just(TokenType::Const)
        .ignore_then(spanned_ident) // constant name
        .then_ignore(just(TokenType::Eq)) // ignore '='
        .then(expr) // value expression
        .then_ignore(just(TokenType::Semicolon)) // ignore ';'
        .map_with(|(name, value), extra| Spanned(ConstDef { name, value }, extra.span()))
}

/// Creates a parser for namespace definitions
fn namespace_parser<'tokens, 'src: 'tokens, I>(
    top_level_item: impl Parser<'tokens, I, TopLevelItem, extra::Err<Rich<'tokens, TokenType<'src>>>>
        + Clone,
) -> impl Parser<'tokens, I, Spanned<Namespace>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();

    // Namespace definition: namespace Name { items... }
    just(TokenType::Namespace)
        .ignore_then(spanned_ident) // namespace name
        .then(
            top_level_item
                .repeated()
                .collect::<Vec<TopLevelItem>>()
                .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace)), // items in {}
        )
        .map_with(|(name, body), extra| Spanned(Namespace { name, body }, extra.span()))
}

/// Creates a parser for top-level items
fn top_level_item_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, TopLevelItem, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    recursive(|top_level_item| {
        let func_def = function_def_parser().map(TopLevelItem::Function);
        let struct_def = struct_def_parser().map(TopLevelItem::Struct);
        let import_stmt = import_stmt_parser().map(TopLevelItem::Import);
        let const_def = const_def_parser().map(TopLevelItem::Const);
        let namespace_def = namespace_parser(top_level_item).map(TopLevelItem::Namespace);

        // Try top-level item alternatives in order
        func_def
            .or(struct_def)
            .or(import_stmt)
            .or(const_def)
            .or(namespace_def)
    })
}

/// Creates the main parser for Cairo-M source code.
///
/// This function constructs a parser combinator that can parse a complete Cairo-M
/// program from a stream of tokens. The parser uses recursive descent with
/// operator precedence handling for expressions.
///
/// ## Parser Structure
///
/// The parser is organized hierarchically:
/// 1. **Expressions**: Built from atoms (literals, identifiers) up through binary operators
/// 2. **Types**: Handle named types, pointers, and tuples
/// 3. **Statements**: Control flow, declarations, and expression statements
/// 4. **Top-level items**: Functions, structs, namespaces, imports, and constants
///
/// ## Operator Precedence (lowest to highest)
///
/// 1. Logical OR (`||`)
/// 2. Logical AND (`&&`)
/// 3. Equality (`==`, `!=`)
/// 4. Additive (`+`, `-`)
/// 5. Multiplicative (`*`, `/`)
/// 6. Postfix (function calls, member access, indexing)
///
/// ## Generic Parameters
///
/// - `'tokens`: Lifetime of the token stream
/// - `'src`: Lifetime of the source code (must outlive tokens)
/// - `I`: Input type that provides tokens and spans
///
/// ## Returns
///
/// A parser that produces a `Vec<TopLevelItem>` representing the complete program,
/// or parsing errors if the input is malformed.
pub fn parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Vec<TopLevelItem>, extra::Err<Rich<'tokens, TokenType<'src>>>>
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    // Parse zero or more top-level items to form a complete program
    top_level_item_parser().repeated().collect()
}

#[cfg(test)]
mod tests {
    use crate::db::ParserDatabaseImpl;

    use super::*;
    use crate::lexer::LexingError;
    use ariadne::{Label, Report, ReportKind, Source};
    use chumsky::input::Stream;
    use chumsky::Parser;
    use logos::Logos;

    /// A test case specification
    pub struct TestCase {
        pub name: &'static str,
        pub code: &'static str,
        pub expected_construct: Option<&'static str>, // For targeting specific constructs
    }

    /// Enhanced macro for creating test cases with different modes
    macro_rules! test_case {
        (
            name: $name:expr,
            code: $code:expr,

        ) => {
            TestCase {
                name: $name,
                code: $code,
                expected_construct: None,
            }
        };
        (
            name: $name:expr,
            code: $code:expr,

        ) => {
            TestCase {
                name: $name,
                code: $code,
                expected_construct: None,
            }
        };

        (
            name: $name:expr,
            code: $code:expr,

            construct: $construct:expr
        ) => {
            TestCase {
                name: $name,
                code: $code,
                expected_construct: Some($construct),
            }
        };
    }

    /// A snapshot entry that includes both the source code and the result
    #[derive(Debug)]
    struct SnapshotEntry {
        code: String,
        result: SnapshotResult,
    }

    struct SnapshotEntries(Vec<SnapshotEntry>);

    impl std::fmt::Display for SnapshotEntry {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "code: {}\nresult: {}", self.code, self.result)
        }
    }

    impl std::fmt::Display for SnapshotEntries {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for entry in self.0.iter() {
                write!(f, "{entry}")?;
            }
            Ok(())
        }
    }

    #[derive(Debug)]
    enum SnapshotResult {
        ParseSuccess(Vec<TopLevelItem>),
        ParseError(String),
    }

    impl std::fmt::Display for SnapshotResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::ParseSuccess(ast) => write!(f, "{ast:#?}"),
                Self::ParseError(err) => write!(f, "{err}"),
            }
        }
    }

    /// Execute a test case and create appropriate snapshots
    fn run_test_case(test_case: TestCase) {
        let db = ParserDatabaseImpl::default();
        let source = SourceProgram::new(&db, test_case.code.to_string());
        let result = parse_program(&db, source);
        match result {
            Ok(ast) => {
                let snapshot_name = test_case.expected_construct.map_or_else(
                    || test_case.name.to_string(),
                    |construct| format!("{}_{}", construct, test_case.name),
                );
                let snapshot_entry = SnapshotEntry {
                    code: test_case.code.to_string(),
                    result: SnapshotResult::ParseSuccess(ast.items().to_vec()),
                };
                insta::assert_snapshot!(snapshot_name, snapshot_entry);
            }
            Err(errs) => {
                let snapshot_name = format!("{}_diagnostic", test_case.name);
                let mut snapshot_entries = SnapshotEntries(Vec::new());
                for err in errs.iter() {
                    snapshot_entries.0.push(SnapshotEntry {
                        code: test_case.code.to_string(),
                        result: SnapshotResult::ParseError(err.clone()),
                    });
                }
                insta::assert_snapshot!(snapshot_name, snapshot_entries);
            }
        }
    }

    // Helper function to parse a string input
    fn parse_program(
        db: &dyn crate::Db,
        source: SourceProgram,
    ) -> Result<ParsedModule, Vec<String>> {
        let input = source.text(db);
        // First, collect all tokens and check for lexer errors
        let mut tokens = Vec::new();
        let mut lexer_errors = Vec::new();

        for (token_result, span) in TokenType::lexer(input).spanned() {
            match token_result {
                Ok(token) => tokens.push((token, span.into())),
                Err(lexing_error) => {
                    lexer_errors.push(build_lexer_error_message(input, lexing_error, span.into()));
                }
            }
        }

        // If there are lexer errors, return them immediately
        if !lexer_errors.is_empty() {
            return Err(lexer_errors);
        }

        // Create token stream from the successfully lexed tokens
        let token_stream =
            Stream::from_iter(tokens).map((0..input.len()).into(), |(t, s): (_, _)| (t, s));

        parser()
            .then_ignore(end())
            .parse(token_stream)
            .into_result()
            .map_err(|errs| build_parser_error_message(input, errs))
            .map(ParsedModule::new)
    }

    fn build_lexer_error_message(source: &str, error: LexingError, span: SimpleSpan) -> String {
        let mut write_buffer = Vec::new();
        Report::build(ReportKind::Error, ((), span.into_range()))
            .with_config(
                ariadne::Config::new()
                    .with_index_type(ariadne::IndexType::Byte)
                    .with_color(false),
            )
            .with_code(3)
            .with_message(error.to_string())
            .with_label(Label::new(((), span.into_range())).with_message(format!("{error}")))
            .finish()
            .write(Source::from(source), &mut write_buffer)
            .unwrap();
        String::from_utf8_lossy(&write_buffer).to_string()
    }

    fn build_parser_error_message(
        source: &str,
        errs: Vec<Rich<TokenType, SimpleSpan>>,
    ) -> Vec<String> {
        let mut reports = Vec::new();
        for err in errs {
            let mut write_buffer = Vec::new();
            Report::build(ReportKind::Error, ((), err.span().into_range()))
                .with_config(
                    ariadne::Config::new()
                        .with_index_type(ariadne::IndexType::Byte)
                        .with_color(false),
                )
                .with_code(3)
                .with_message(err.to_string())
                .with_label(
                    Label::new(((), err.span().into_range()))
                        .with_message(err.reason().to_string()),
                )
                .finish()
                .write(Source::from(source), &mut write_buffer)
                .unwrap();
            let report = String::from_utf8_lossy(&write_buffer).to_string();
            reports.push(report);
        }
        reports
    }

    // ===================
    // Expression Tests - Individual Functions
    // ===================

    #[test]
    fn test_integer_literal() {
        run_test_case(test_case!(
            name: "integer_literal",
            code: "func test() { 42; }",
            construct: "expression"
        ));
    }

    #[test]
    fn test_simple_identifier() {
        run_test_case(test_case!(
            name: "simple_identifier",
            code: "func test() { my_var; }",
            construct: "expression"
        ));
    }

    #[test]
    fn test_addition() {
        run_test_case(test_case!(
            name: "addition",
            code: "func test() { a + b; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_multiplication_precedence() {
        run_test_case(test_case!(
            name: "multiplication_precedence",
            code: "func test() { a + b * c; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_comparison_and_logical() {
        run_test_case(test_case!(
            name: "comparison_and_logical",
            code: "func test() { a == b && c != d; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_complex_precedence() {
        run_test_case(test_case!(
            name: "complex_precedence",
            code: "func test() { a + b * c == d && e || f; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_simple_function_call() {
        run_test_case(test_case!(
            name: "simple_call",
            code: "func test() { foo(); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_function_call_with_args() {
        run_test_case(test_case!(
            name: "call_with_args",
            code: "func test() { add(a, b, c); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_chained_calls() {
        run_test_case(test_case!(
            name: "chained_calls",
            code: "func test() { obj.method().another(); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_simple_member_access() {
        run_test_case(test_case!(
            name: "simple_member",
            code: "func test() { obj.field; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_nested_member_access() {
        run_test_case(test_case!(
            name: "nested_member",
            code: "func test() { obj.inner.field; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_array_index() {
        run_test_case(test_case!(
            name: "array_index",
            code: "func test() { arr[0]; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_nested_index() {
        run_test_case(test_case!(
            name: "nested_index",
            code: "func test() { matrix[i][j]; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_simple_struct_literal() {
        run_test_case(test_case!(
            name: "simple_struct",
            code: "func test() { Point { x: 1, y: 2 }; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_nested_struct_literal() {
        run_test_case(test_case!(
            name: "nested_struct",
            code: "func test() { Rectangle { top_left: Point { x: 0, y: 0 }, width: 10 }; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_empty_struct_literal() {
        run_test_case(test_case!(
            name: "empty_struct",
            code: "func test() { Unit {}; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_simple_tuple() {
        run_test_case(test_case!(
            name: "simple_tuple",
            code: "func test() { (1, 2, 3); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_nested_tuple() {
        run_test_case(test_case!(
            name: "nested_tuple",
            code: "func test() { ((1, 2), (3, 4)); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_parenthesized_expr() {
        run_test_case(test_case!(
            name: "parenthesized_expr",
            code: "func test() { (a + b); }",

            construct: "expression"
        ));
    }

    // ===================
    // Type Expression Tests
    // ===================

    #[test]
    fn test_named_type() {
        run_test_case(test_case!(
            name: "named_type",
            code: "func test(x: felt) { }",

            construct: "type"
        ));
    }

    #[test]
    fn test_pointer_type() {
        run_test_case(test_case!(
            name: "pointer_type",
            code: "func test(x: felt*) { }",

            construct: "type"
        ));
    }

    #[test]
    fn test_tuple_type() {
        run_test_case(test_case!(
            name: "tuple_type",
            code: "func test(x: (felt, felt)) { }",

            construct: "type"
        ));
    }

    #[test]
    fn test_nested_pointer() {
        run_test_case(test_case!(
            name: "nested_pointer",
            code: "func test(x: felt**) { }",

            construct: "type"
        ));
    }

    // ===================
    // Statement Tests
    // ===================

    #[test]
    fn test_simple_let() {
        run_test_case(test_case!(
            name: "simple_let",
            code: "func test() { let x = 5; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_typed_let() {
        run_test_case(test_case!(
            name: "typed_let",
            code: "func test() { let x: felt = 5; }",
        ));
    }

    #[test]
    fn test_let_with_expression() {
        run_test_case(test_case!(
            name: "let_with_expression",
            code: "func test() { let result = a + b * c; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_local_with_type() {
        run_test_case(test_case!(
            name: "local_with_type",
            code: "func test() { local x: felt = 42; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_local_without_type() {
        run_test_case(test_case!(
            name: "local_without_type",
            code: "func test() { local x = infer_me; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_const_statement() {
        run_test_case(test_case!(
            name: "const_statement",
            code: "func test() { const PI = 314; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_simple_assignment() {
        run_test_case(test_case!(
            name: "simple_assignment",
            code: "func test() { x = 5; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_member_assignment() {
        run_test_case(test_case!(
            name: "member_assignment",
            code: "func test() { obj.field = value; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_index_assignment() {
        run_test_case(test_case!(
            name: "index_assignment",
            code: "func test() { arr[0] = item; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_return_with_value() {
        run_test_case(test_case!(
            name: "return_with_value",
            code: "func test() { return 42; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_return_without_value() {
        run_test_case(test_case!(
            name: "return_without_value",
            code: "func test() { return; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_simple_if() {
        run_test_case(test_case!(
            name: "simple_if",
            code: "func test() { if (condition) { x = 1; } }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_if_else() {
        run_test_case(test_case!(
            name: "if_else",
            code: "func test() { if (a > b) { return a; } else { return b; } }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_nested_if() {
        run_test_case(test_case!(
            name: "nested_if",
            code: "func test() { if (a) { if (b) { c = 1; } else { c = 2; } } }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_simple_block() {
        run_test_case(test_case!(
            name: "simple_block",
            code: "func test() { { let x = 1; let y = 2; } }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_nested_blocks() {
        run_test_case(test_case!(
            name: "nested_blocks",
            code: "func test() { { { let inner = 1; } let outer = 2; } }",

            construct: "statement"
        ));
    }

    // ===================
    // Top-Level Item Tests
    // ===================

    #[test]
    fn test_simple_function() {
        run_test_case(test_case!(
            name: "simple_function",
            code: "func add(a: felt, b: felt) -> felt { return a + b; }",

            construct: "function"
        ));
    }

    #[test]
    fn test_function_no_params() {
        run_test_case(test_case!(
            name: "function_no_params",
            code: "func get_constant() -> felt { return 42; }",

            construct: "function"
        ));
    }

    #[test]
    fn test_function_no_return() {
        run_test_case(test_case!(
            name: "function_no_return",
            code: "func print_hello() { let msg = hello; }",

            construct: "function"
        ));
    }

    #[test]
    fn test_function_multiple_params() {
        run_test_case(test_case!(
            name: "function_multiple_params",
            code: "func complex(a: felt, b: felt*, c: (felt, felt)) { }",

            construct: "function"
        ));
    }

    #[test]
    fn test_simple_struct() {
        run_test_case(test_case!(
            name: "simple_struct",
            code: "struct Point { x: felt, y: felt }",

            construct: "struct"
        ));
    }

    #[test]
    fn test_empty_struct() {
        run_test_case(test_case!(
            name: "empty_struct",
            code: "struct Unit { }",

            construct: "struct"
        ));
    }

    #[test]
    fn test_struct_with_pointers() {
        run_test_case(test_case!(
            name: "struct_with_pointers",
            code: "struct Node { data: felt, next: Node* }",

            construct: "struct"
        ));
    }

    #[test]
    fn test_simple_namespace() {
        run_test_case(test_case!(
            name: "simple_namespace",
            code: "namespace Math { const PI = 314; }",

            construct: "namespace"
        ));
    }

    #[test]
    fn test_namespace_with_function() {
        run_test_case(test_case!(
            name: "namespace_with_function",
            code: "namespace Utils { func helper() -> felt { return 1; } }",

            construct: "namespace"
        ));
    }

    #[test]
    fn test_nested_namespace() {
        run_test_case(test_case!(
            name: "nested_namespace",
            code: "namespace Outer { namespace Inner { const VALUE = 42; } }",

            construct: "namespace"
        ));
    }

    #[test]
    fn test_simple_import() {
        run_test_case(test_case!(
            name: "simple_import",
            code: "from std.math import add",

            construct: "import"
        ));
    }

    #[test]
    fn test_import_with_alias() {
        run_test_case(test_case!(
            name: "import_with_alias",
            code: "from std.math import add as plus",

            construct: "import"
        ));
    }

    #[test]
    fn test_nested_path_import() {
        run_test_case(test_case!(
            name: "nested_path_import",
            code: "from very.deep.module.path import function",

            construct: "import"
        ));
    }

    #[test]
    fn test_toplevel_const() {
        run_test_case(test_case!(
            name: "toplevel_const",
            code: "const MAX_SIZE = 100;",

            construct: "const"
        ));
    }

    #[test]
    fn test_const_with_expression() {
        run_test_case(test_case!(
            name: "const_with_expression",
            code: "const COMPUTED = 2 * 3 + 1;",

            construct: "const"
        ));
    }

    // ===================
    // Top-Level Validation Tests
    // ===================

    #[test]
    fn test_invalid_toplevel_let() {
        run_test_case(test_case!(
            name: "invalid_toplevel_let",
            code: "let x = 5;",

        ));
    }

    #[test]
    fn test_invalid_toplevel_local() {
        run_test_case(test_case!(
            name: "invalid_toplevel_local",
            code: "local x: felt = 42;",

        ));
    }

    #[test]
    fn test_invalid_toplevel_assignment() {
        run_test_case(test_case!(
            name: "invalid_toplevel_assignment",
            code: "x = 10;",

        ));
    }

    #[test]
    fn test_invalid_toplevel_expression() {
        run_test_case(test_case!(
            name: "invalid_toplevel_expression",
            code: "42;",

        ));
    }

    #[test]
    fn test_invalid_toplevel_return() {
        run_test_case(test_case!(
            name: "invalid_toplevel_return",
            code: "return 5;",

        ));
    }

    #[test]
    fn test_invalid_toplevel_if() {
        run_test_case(test_case!(
            name: "invalid_toplevel_if",
            code: "if (true) { x = 1; }",

        ));
    }

    #[test]
    fn test_invalid_toplevel_block() {
        run_test_case(test_case!(
            name: "invalid_toplevel_block",
            code: "{ let x = 1; }",

        ));
    }

    // ===================
    // Diagnostic Tests
    // ===================

    #[test]
    fn test_missing_semicolon() {
        run_test_case(test_case!(
            name: "missing_semicolon",
            code: "func test() { let x = 5 }",

        ));
    }

    #[test]
    fn test_invalid_binary_op() {
        run_test_case(test_case!(
            name: "invalid_binary_op",
            code: "func test() { a +; }",

        ));
    }

    #[test]
    fn test_unclosed_paren() {
        run_test_case(test_case!(
            name: "unclosed_paren",
            code: "func test() { foo(a, b; }",

        ));
    }

    #[test]
    fn test_invalid_struct_literal() {
        run_test_case(test_case!(
            name: "invalid_struct_literal",
            code: "func test() { Point { x: 1, }; }",

        ));
    }

    #[test]
    fn test_missing_function_name() {
        run_test_case(test_case!(
            name: "missing_function_name",
            code: "func (a: felt) -> felt { }",

        ));
    }

    #[test]
    fn test_invalid_parameter() {
        run_test_case(test_case!(
            name: "invalid_parameter",
            code: "func test(: felt) { }",

        ));
    }

    #[test]
    fn test_missing_function_body() {
        run_test_case(test_case!(
            name: "missing_function_body",
            code: "func test() -> felt",

        ));
    }

    #[test]
    fn test_missing_struct_name() {
        run_test_case(test_case!(
            name: "missing_struct_name",
            code: "struct { x: felt }",

        ));
    }

    #[test]
    fn test_invalid_field_definition() {
        run_test_case(test_case!(
            name: "invalid_field_definition",
            code: "struct Point { x, y: felt }",

        ));
    }

    #[test]
    fn test_invalid_if_condition() {
        run_test_case(test_case!(
            name: "invalid_if_condition",
            code: "func test() { if { x = 1; } }",

        ));
    }

    #[test]
    fn test_missing_assignment_target() {
        run_test_case(test_case!(
            name: "missing_assignment_target",
            code: "func test() { = 5; }",

        ));
    }

    #[test]
    fn test_invalid_import_syntax() {
        run_test_case(test_case!(
            name: "invalid_import_syntax",
            code: "import std.math",

        ));
    }

    #[test]
    fn test_empty_import_path() {
        run_test_case(test_case!(
            name: "empty_import_path",
            code: "from import item",

        ));
    }

    // ===================
    // Integration Tests
    // ===================

    #[test]
    fn test_complete_program() {
        run_test_case(test_case!(
            name: "complete_program",
            code: r#"
                struct Vector {
                    x: felt,
                    y: felt
                }

                namespace MathUtils {
                    func magnitude(v: Vector) -> felt {
                        return (v.x * v.x + v.y * v.y);
                    }

                    func rfib(n: felt) -> felt {
                        if (n == 0) {
                            return 0;
                        }
                        if (n == 1) {
                            return 1;
                        }
                        return rfib(n - 1) + rfib(n - 2);
                    }
                }

                const TOP_LEVEL_CONST = 100;
            "#,

        ));
    }

    #[test]
    fn test_imports_and_functions() {
        run_test_case(test_case!(
            name: "imports_and_functions",
            code: r#"
                from std.math import sqrt
                from std.io import print as output

                struct Point {
                    x: felt,
                    y: felt
                }

                func distance(p1: Point, p2: Point) -> felt {
                    local dx: felt = p1.x - p2.x;
                    local dy: felt = p1.y - p2.y;
                    return sqrt(dx * dx + dy * dy);
                }
            "#,

        ));
    }

    #[test]
    fn test_complex_expression_precedence() {
        run_test_case(test_case!(
            name: "complex_expression_precedence",
            code: "func test() { result = a.field[0].method(b + c * d, e && f || g).value; }",

            construct: "expression"
        ));
    }

    // ===================
    // Edge Case Tests
    // ===================

    #[test]
    fn test_empty_program() {
        run_test_case(test_case!(
            name: "empty_program",
            code: "",

        ));
    }

    #[test]
    fn test_whitespace_only() {
        run_test_case(test_case!(
            name: "whitespace_only",
            code: "   \n\t   \n  ",

        ));
    }

    #[test]
    fn test_deeply_nested() {
        run_test_case(test_case!(
            name: "deeply_nested",
            code: "func test() { ((((((a + b) * c) - d) / e) == f) && g); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_long_identifiers() {
        run_test_case(test_case!(
            name: "long_identifiers",
            code: "func test() { let very_long_variable_name_that_tests_identifier_parsing = another_very_long_identifier; }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_complex_struct() {
        run_test_case(test_case!(
            name: "complex_struct",
            code: r#"
                struct ComplexStruct {
                    field1: felt,
                    field2: felt*,
                    field3: (felt, felt),
                    field4: AnotherStruct,
                    field5: AnotherStruct*
                }
            "#,

            construct: "struct"
        ));
    }

    #[test]
    fn test_many_parameters() {
        run_test_case(test_case!(
            name: "many_parameters",
            code: "func complex_function(a: felt, b: felt*, c: (felt, felt), d: MyStruct, e: MyStruct*) -> (felt, felt) { return (a, b); }",

            construct: "function"
        ));
    }

    // ===================
    // Regression Tests
    // ===================

    #[test]
    fn test_trailing_comma_struct_literal() {
        run_test_case(test_case!(
            name: "trailing_comma_struct_literal",
            code: "func test() { Point { x: 1, y: 2, }; }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_trailing_comma_function_params() {
        run_test_case(test_case!(
            name: "trailing_comma_function_params",
            code: "func test(a: felt, b: felt,) { }",

            construct: "function"
        ));
    }

    #[test]
    fn test_trailing_comma_function_call() {
        run_test_case(test_case!(
            name: "trailing_comma_function_call",
            code: "func test() { foo(a, b, c,); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_single_element_tuple_ambiguity() {
        run_test_case(test_case!(
            name: "single_element_tuple_ambiguity",
            code: "func test() { (single_element,); }",

            construct: "expression"
        ));
    }

    #[test]
    fn test_chained_operations() {
        run_test_case(test_case!(
            name: "chained_operations",
            code: "func test() { obj.method1().field.method2()[0].final_field; }",

            construct: "expression"
        ));
    }

    // ===================
    // Boundary Tests
    // ===================

    #[test]
    fn test_deep_nesting() {
        run_test_case(test_case!(
            name: "deep_nesting",
            code: "func test() { if (true) { if (true) { if (true) { if (true) { if (true) { x = 1; } } } } } }",

            construct: "statement"
        ));
    }

    #[test]
    fn test_large_number() {
        run_test_case(test_case!(
            name: "large_number",
            code: "func test() { 4294967295; }", // Max u32

            construct: "expression"
        ));
    }

    #[test]
    fn test_precedence_chain() {
        run_test_case(test_case!(
            name: "precedence_chain",
            code: "func test() { a || b && c == d + e * f / g - h; }",

            construct: "expression"
        ));
    }

    // ===================
    // Error Recovery Tests
    // ===================

    #[test]
    fn test_multiple_syntax_errors() {
        run_test_case(test_case!(
            name: "multiple_syntax_errors",
            code: r#"
                func bad1( { }
                func good() { return 1; }
                struct bad2 x: felt }
                struct Good { x: felt }
            "#,

        ));
    }

    #[test]
    fn test_mixed_valid_invalid() {
        run_test_case(test_case!(
            name: "mixed_valid_invalid",
            code: r#"
                const GOOD = 1;
                let bad = 42;
                const ALSO_GOOD = 2;
            "#,

        ));
    }

    #[test]
    fn test_lexer_error_integration() {
        run_test_case(test_case!(
            name: "lexer_error_integration",
            code: "func test() { let x = 0x80000000; }",
        ));
    }

    #[test]
    fn test_invalid_number_format_integration() {
        run_test_case(test_case!(
            name: "invalid_number_format_integration",
            code: "func test() { let x = 0xGG; }",
        ));
    }
}
