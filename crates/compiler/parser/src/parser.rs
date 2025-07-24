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
//! - **Input types**: `SourceFile` marked with `#[salsa::input]` represents source code
//! - **Cached parsing**: Only the `parse_file` operation is tracked, not individual AST nodes
//! - **Plain AST types**: All AST nodes are regular Rust types for maximum performance
//! - **Database integration**: Parser functions take a `&dyn Db` parameter for the parsing operation
//!
//! ### Caching Behavior
//! Salsa caches the entire parse result. When source code changes, only the parsing operation
//! is re-executed. The resulting AST is stored as a single cached unit, which is much more
//! efficient than tracking individual nodes.

use cairo_m_compiler_diagnostics::Diagnostic;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::SourceFile;
use crate::lexer::TokenType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamedType {
    Felt,
    Bool,
    U32,
    Custom(String),
}

impl std::fmt::Display for NamedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Felt => write!(f, "felt"),
            Self::Bool => write!(f, "bool"),
            Self::U32 => write!(f, "u32"),
            Self::Custom(name) => write!(f, "{}", name),
        }?;
        Ok(())
    }
}

/// Represents a type expression in the Cairo-M language.
///
/// Type expressions describe the shape and structure of data, including
/// basic types, pointers, and tuple types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    /// A named type (e.g., `felt`, `Vector`)
    Named(Spanned<NamedType>),
    /// A pointer type (e.g., `felt*`, `Vector*`)
    Pointer(Box<Spanned<TypeExpr>>),
    /// A tuple type (e.g., `(felt, felt)`, `(Vector, felt, bool)`)
    Tuple(Vec<Spanned<TypeExpr>>),
}

/// Unary operators supported in expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum UnaryOp {
    /// Logical NOT operator `!`
    Not,
    /// Arithmetic negation operator `-`
    Neg,
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
    /// Less than operator `<`
    Less,
    /// Greater than operator `>`
    Greater,
    /// Less than or equal operator `<=`
    LessEqual,
    /// Greater than or equal operator `>=`
    GreaterEqual,
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
    /// Boolean literal (e.g., `true`, `false`)
    BooleanLiteral(bool),
    /// Variable identifier (e.g., `x`, `my_var`, `result`)
    Identifier(Spanned<String>),
    /// Unary operation (e.g., `!x`, `-y`)
    UnaryOp {
        op: UnaryOp,
        expr: Box<Spanned<Expression>>,
    },
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
    pub type_expr: Spanned<TypeExpr>,
}

/// Represents a pattern in let bindings.
///
/// Patterns allow destructuring values during variable binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// Single identifier pattern (e.g., `x`)
    Identifier(Spanned<String>),
    /// Tuple pattern for destructuring (e.g., `(x, y, z)`)
    Tuple(Vec<Spanned<String>>),
}

/// Represents a statement in the Cairo-M language.
///
/// Statements are constructs that perform actions but don't necessarily
/// evaluate to a value. They form the body of functions and control flow.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    /// Global variable declaration (e.g., `let x = 5;` or `let (x, y) = (1, 2);`)
    Let {
        pattern: Pattern,
        statement_type: Option<Spanned<TypeExpr>>,
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
    /// Infinite loop (e.g., `loop { ... }`)
    Loop { body: Box<Spanned<Statement>> },
    /// While loop (e.g., `while condition { ... }`)
    While {
        condition: Spanned<Expression>,
        body: Box<Spanned<Statement>>,
    },
    /// For loop (e.g., `for i in 0..10 { ... }`)
    For {
        variable: Spanned<String>,
        iterable: Spanned<Expression>,
        body: Box<Spanned<Statement>>,
    },
    /// Break statement (e.g., `break;`)
    Break,
    /// Continue statement (e.g., `continue;`)
    Continue,
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
    /// Constant definition
    Const(Spanned<ConstDef>),
    /// Use statement
    Use(Spanned<UseStmt>),
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
    /// The function's return type (defaults to unit type if not specified)
    pub return_type: Spanned<TypeExpr>,
    /// The function's body (list of statements)
    pub body: Vec<Spanned<Statement>>,
}

/// Represents a struct definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDef {
    /// The struct's name
    pub name: Spanned<String>,
    /// The struct's fields (name and type pairs)
    pub fields: Vec<(Spanned<String>, Spanned<TypeExpr>)>,
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

/// Represents items in a use statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UseItems {
    /// A single item import.
    Single(Spanned<String>),
    /// A list of items in braces.
    List(Vec<Spanned<String>>),
}

impl UseItems {
    /// Returns the list of names of imported items.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::Single(name) => vec![name.value().as_str()],
            Self::List(names) => names.iter().map(|name| name.value().as_str()).collect(),
        }
    }

    /// Returns the list of spans of imported items.
    pub fn spans(&self) -> Vec<SimpleSpan<usize>> {
        match self {
            Self::Single(name) => vec![name.span()],
            Self::List(names) => names.iter().map(|name| name.span()).collect(),
        }
    }
}

/// Represents a use statement for Rust-like imports.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UseStmt {
    /// The module path (e.g., ["std", "math"] for std::math).
    pub path: Vec<Spanned<String>>,
    /// The imported items.
    pub items: UseItems,
}

/// Wrapper for the parsed AST result.
///
/// This follows the Salsa best practice of caching the entire parse result
/// rather than individual AST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Output from the parsing process, including both AST and diagnostics
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseOutput {
    pub module: ParsedModule,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutput {
    pub const fn new(module: ParsedModule, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            module,
            diagnostics,
        }
    }
}

/// Parse a source program into a module AST with diagnostics.
///
/// This is the main Salsa-tracked parsing function. It caches the entire
/// parse result, following best practices from the Ruff codebase.
#[salsa::tracked]
pub fn parse_file(db: &dyn crate::Db, source: SourceFile) -> ParseOutput {
    use logos::Logos;
    let _file_path = source.file_path(db);
    let input = source.text(db);

    // Collect tokens and handle lexer errors
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    for (token_result, span) in TokenType::lexer(input).spanned() {
        match token_result {
            Ok(token) => tokens.push((token, span.into())),
            Err(lexing_error) => {
                // Create a meaningful diagnostic for lexer errors
                let diagnostic = Diagnostic::lexical_error(
                    source.file_path(db).to_string(),
                    format!("{lexing_error}"),
                    span.into(),
                );
                diagnostics.push(diagnostic);
            }
        }
    }

    // If there are lexer errors, return empty module with diagnostics
    if !diagnostics.is_empty() {
        return ParseOutput::new(ParsedModule::new(vec![]), diagnostics);
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
        Ok(items) => ParseOutput::new(ParsedModule::new(items), diagnostics),
        Err(parse_errors) => {
            // Convert parser errors to diagnostics with better messages
            for error in parse_errors {
                let diagnostic = Diagnostic::syntax_error(
                    source.file_path(db).to_string(),
                    format!("{error}"),
                    *error.span(),
                );
                diagnostics.push(diagnostic);
            }
            ParseOutput::new(ParsedModule::new(vec![]), diagnostics)
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
fn ident_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, String, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    select! { TokenType::Identifier(s) => s.to_string() }.labelled("identifier")
}

/// Creates a spanned identifier parser that captures both the identifier and its span
fn spanned_ident_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<String>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    select! { TokenType::Identifier(s) => s.to_string() }
        .map_with(|s, extra| Spanned::new(s, extra.span()))
        .labelled("identifier")
}

/// Creates a parser for type expressions (named types, pointers, tuples)
fn type_expr_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<TypeExpr>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let ident = ident_parser();

    recursive(|type_expr| {
        // Named types: felt, Vector, MyStruct, etc.
        let named_type = ident.clone().map_with(|name, extra| {
            let named_type = match name.as_str() {
                "felt" => NamedType::Felt,
                "bool" => NamedType::Bool,
                "u32" => NamedType::U32,
                _ => NamedType::Custom(name),
            };
            let span = extra.span();
            Spanned::new(TypeExpr::Named(Spanned::new(named_type, span)), span)
        });

        // Tuple types: (felt, felt), (Vector, bool, felt), etc.
        let tuple_type = type_expr
            .clone()
            .separated_by(just(TokenType::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(TokenType::LParen), just(TokenType::RParen))
            .map_with(|types, extra| {
                let span = extra.span();
                // Single type in parens is just a parenthesized type
                if types.len() == 1 {
                    // Extract the inner type but use the parentheses span
                    types.into_iter().next().unwrap()
                } else {
                    // Multiple types form a tuple type
                    Spanned::new(TypeExpr::Tuple(types), span)
                }
            });

        let base_type = named_type.or(tuple_type);

        // Handle pointer types: felt*, Vector**, etc. (right-associative via foldl)
        base_type.foldl(
            just(TokenType::Mul)
                .map_with(|_, extra| extra.span())
                .repeated(),
            |ty, star_span: SimpleSpan| {
                let start = ty.span().start;
                let end = star_span.end;
                let span = SimpleSpan::from(start..end);
                Spanned::new(TypeExpr::Pointer(Box::new(ty)), span)
            },
        )
    })
}

/// Creates a parser for expressions with proper operator precedence
fn expression_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expression>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();

    recursive(|expr| {
        // Atomic expressions (cannot be broken down further)

        // Integer literals (e.g., 42, 0, 1337)
        let literal = select! { TokenType::LiteralNumber(n) => Expression::Literal(n) }
            .map_with(|lit, extra| Spanned::new(lit, extra.span()));

        // Boolean literals (e.g., true, false)
        let boolean_literal = select! {
            TokenType::True => Expression::BooleanLiteral(true),
            TokenType::False => Expression::BooleanLiteral(false),
        }
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
            .or(boolean_literal)
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
                let span = if args.is_empty() {
                    // For empty argument lists, we need to include the parentheses
                    // Since we don't have direct access to the closing paren position,
                    // we'll extend minimally beyond the callee span
                    SimpleSpan::from(span_callee.start..span_callee.end + 2) // +2 for "()"
                } else {
                    // With arguments, span from start of callee to end of last argument
                    let last_arg_end = args.last().unwrap().span().end;
                    SimpleSpan::from(span_callee.start..last_arg_end)
                };
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
        let unary_op = |token, op| just(token).to(op);

        // Unary operators: !, - (right-associative, high precedence)
        let unary = choice((
            unary_op(TokenType::Not, UnaryOp::Not).map_with(|op, extra| (op, extra.span())),
            unary_op(TokenType::Minus, UnaryOp::Neg).map_with(|op, extra| (op, extra.span())),
        ))
        .repeated()
        .foldr(
            call.clone(),
            |(op, op_span): (UnaryOp, SimpleSpan), expr| {
                let expr_span = expr.span();
                // Span should be from start of operator to end of expression
                let full_span = SimpleSpan::from(op_span.start..expr_span.end);
                Spanned::new(
                    Expression::UnaryOp {
                        op,
                        expr: Box::new(expr),
                    },
                    full_span,
                )
            },
        );

        // Multiplicative operators: *, / (left-associative)
        let mul = unary.clone().foldl(
            choice((
                op(TokenType::Mul, BinaryOp::Mul),
                op(TokenType::Div, BinaryOp::Div),
            ))
            .then(unary.clone())
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

        // Comparison operators: ==, !=, <, >, <=, >= (left-associative)
        let cmp = add.clone().foldl(
            choice((
                op(TokenType::EqEq, BinaryOp::Eq),
                op(TokenType::Neq, BinaryOp::Neq),
                op(TokenType::Less, BinaryOp::Less),
                op(TokenType::Greater, BinaryOp::Greater),
                op(TokenType::LessEqual, BinaryOp::LessEqual),
                op(TokenType::GreaterEqual, BinaryOp::GreaterEqual),
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
fn parameter_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Parameter, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
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
fn statement_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Statement>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let expr = expression_parser();
    let type_expr = type_expr_parser();

    // Pattern parser for destructuring
    let pattern = {
        // Tuple pattern: (x, y, z)
        let tuple_pattern = spanned_ident
            .clone()
            .separated_by(just(TokenType::Comma))
            .at_least(2)
            .collect::<Vec<_>>()
            .delimited_by(just(TokenType::LParen), just(TokenType::RParen))
            .map(Pattern::Tuple);

        // Single identifier pattern
        let ident_pattern = spanned_ident.clone().map(Pattern::Identifier);

        // Try tuple pattern first, then fall back to identifier
        tuple_pattern.or(ident_pattern)
    };

    recursive(|statement| {
        // Block statement: { stmt1; stmt2; stmt3; }
        let block = statement
            .clone()
            .repeated()
            .collect::<Vec<Spanned<Statement>>>()
            .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace))
            .map(Statement::Block)
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Let statement: let pattern (: type)? = expression;
        let let_stmt = just(TokenType::Let)
            .ignore_then(pattern.clone()) // pattern (identifier or tuple)
            .then(
                just(TokenType::Colon)
                    .ignore_then(type_expr.clone()) // optional type annotation
                    .or_not(),
            )
            .then_ignore(just(TokenType::Eq)) // ignore '='
            .then(expr.clone()) // value expression
            .then_ignore(just(TokenType::Semicolon)) // ignore ';'
            .map(|((pattern, statement_type), value)| Statement::Let {
                pattern,
                statement_type,
                value,
            })
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

        // Loop statement: loop { ... }
        let loop_stmt = just(TokenType::Loop)
            .ignore_then(statement.clone()) // loop body (typically a block)
            .map(|body| Statement::Loop {
                body: Box::new(body),
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // While statement: while (condition) { ... }
        let while_stmt = just(TokenType::While)
            .ignore_then(
                expr.clone()
                    .delimited_by(just(TokenType::LParen), just(TokenType::RParen)), // condition in parens
            )
            .then(statement.clone()) // body
            .map(|(condition, body)| Statement::While {
                condition,
                body: Box::new(body),
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // For statement: for variable in iterable { ... }
        let for_stmt = just(TokenType::For)
            .ignore_then(spanned_ident.clone()) // loop variable
            .then_ignore(just(TokenType::In)) // ignore 'in'
            .then(expr.clone()) // iterable expression
            .then(statement.clone()) // body
            .map(|((variable, iterable), body)| Statement::For {
                variable,
                iterable,
                body: Box::new(body),
            })
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Break statement: break;
        let break_stmt = just(TokenType::Break)
            .then_ignore(just(TokenType::Semicolon))
            .to(Statement::Break)
            .map_with(|stmt, extra| Spanned::new(stmt, extra.span()));

        // Continue statement: continue;
        let continue_stmt = just(TokenType::Continue)
            .then_ignore(just(TokenType::Semicolon))
            .to(Statement::Continue)
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
            .or(loop_stmt)
            .or(while_stmt)
            .or(for_stmt)
            .or(break_stmt)
            .or(continue_stmt)
            .or(let_stmt)
            .or(const_stmt)
            .or(return_stmt)
            .or(assignment_or_expr)
    })
}

/// Creates a parser for function definitions
fn function_def_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<FunctionDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();
    let param = parameter_parser();
    let type_expr = type_expr_parser();
    let statement = statement_parser();

    // Function definition: fn name(param1: type1, param2: type2) -> return_type { body }
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
            // If no return type is specified, default to unit type ()
            let return_type = return_type.unwrap_or_else(|| {
                let span = SimpleSpan::from(0..0); // Default span for unit type
                Spanned::new(TypeExpr::Tuple(vec![]), span)
            });
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
fn struct_def_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<StructDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
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

/// Creates a parser for use statements (Rust-like imports).
fn use_stmt_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<UseStmt>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    let spanned_ident = spanned_ident_parser();

    let module_path = spanned_ident
        .clone()
        .separated_by(just(TokenType::ColonColon))
        .at_least(1) // Ensure at least one segment
        .collect::<Vec<_>>();

    // Single item: use path::to::module::item;
    let single = module_path
        .clone()
        .map(|mut path| {
            // Safe unwrap: at_least(1) ensures path has at least one segment
            // But we use safe pattern matching for robustness
            let item = path.pop().unwrap_or_else(|| {
                // This should never happen due to at_least(1) constraint
                Spanned(String::from(""), SimpleSpan::new((), 0..0))
            });
            UseStmt {
                path,
                items: UseItems::Single(item),
            }
        })
        .then_ignore(just(TokenType::Semicolon));

    // List: use path::to::module::{item1, item2};
    let list = module_path
        .then_ignore(just(TokenType::ColonColon))
        .then(
            spanned_ident
                .clone()
                .separated_by(just(TokenType::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(TokenType::LBrace), just(TokenType::RBrace)),
        )
        .map(|(path, items)| UseStmt {
            path,
            items: UseItems::List(items),
        })
        .then_ignore(just(TokenType::Semicolon));

    just(TokenType::Use)
        .ignore_then(single.or(list))
        .map_with(|stmt, extra| Spanned::new(stmt, extra.span()))
}

/// Creates a parser for constant definitions
fn const_def_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<ConstDef>, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
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
fn top_level_item_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, TopLevelItem, extra::Err<Rich<'tokens, TokenType<'src>>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    recursive(|top_level_item| {
        let func_def = function_def_parser().map(TopLevelItem::Function);
        let struct_def = struct_def_parser().map(TopLevelItem::Struct);
        let const_def = const_def_parser().map(TopLevelItem::Const);
        let namespace_def = namespace_parser(top_level_item).map(TopLevelItem::Namespace);
        let use_stmt = use_stmt_parser().map(TopLevelItem::Use);

        // Try top-level item alternatives in order
        func_def
            .or(struct_def)
            .or(const_def)
            .or(namespace_def)
            .or(use_stmt)
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
pub fn parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<TopLevelItem>, extra::Err<Rich<'tokens, TokenType<'src>>>>
where
    I: ValueInput<'tokens, Token = TokenType<'src>, Span = SimpleSpan>,
{
    // Parse zero or more top-level items to form a complete program
    top_level_item_parser().repeated().collect()
}
