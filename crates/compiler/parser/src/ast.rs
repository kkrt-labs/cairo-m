//! # Cairo-M AST
//!
//! This module contains the abstract syntax tree (AST) for the Cairo-M language.
//!
//! The AST is used to represent the structure of the program, including functions,
//! structs, namespaces, and constants.

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
    Identifier(String),
    /// Binary operation (e.g., `a + b`, `x == y`, `p && q`)
    BinaryOp {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Function call (e.g., `foo()`, `add(x, y)`)
    FunctionCall {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    /// Member access (e.g., `obj.field`, `vector.x`)
    MemberAccess {
        object: Box<Expression>,
        field: String,
    },
    /// Array/collection indexing (e.g., `arr[0]`, `matrix[i][j]`)
    IndexAccess {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    /// Struct literal (e.g., `Point { x: 1, y: 2 }`)
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    /// Tuple literal (e.g., `(1, 2, 3)`, `(x, y)`)
    Tuple(Vec<Expression>),
}

/// Represents a function parameter with its name and type.
///
/// Used in function definitions to specify the expected arguments.
#[salsa::interned(debug)]
pub struct Parameter<'db> {
    /// The parameter name
    pub name: String,
    /// The parameter's type
    pub type_expr: TypeExpr,
}

/// Represents a statement in the Cairo-M language.
///
/// Statements are constructs that perform actions but don't necessarily
/// evaluate to a value. They form the body of functions and control flow.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement<'db> {
    /// Global variable declaration (e.g., `let x = 5;`)
    Let { name: String, value: Expression },
    /// Local variable declaration with optional type annotation (e.g., `local x: felt = 5;`)
    Local {
        name: String,
        ty: Option<TypeExpr>,
        value: Expression,
    },
    /// Constant declaration (e.g., `const PI = 314;`)
    Const(ConstDef<'db>),
    /// Assignment to an existing variable (e.g., `x = new_value;`)
    Assignment { lhs: Expression, rhs: Expression },
    /// Return statement (e.g., `return x;`, `return;`)
    Return { value: Option<Expression> },
    /// Conditional statement (e.g., `if (condition) { ... } else { ... }`)
    If {
        condition: Expression,
        then_block: Box<Statement<'db>>,
        else_block: Option<Box<Statement<'db>>>,
    },
    /// Expression used as a statement (e.g., `foo();`)
    Expression(Expression),
    /// Block of statements (e.g., `{ stmt1; stmt2; stmt3; }`)
    Block(Vec<Statement<'db>>),
}

/// Represents a top-level item in a Cairo-M program.
///
/// These are the constructs that can appear at the module level,
/// outside of any function or namespace body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TopLevelItem<'db> {
    /// Function definition
    Function(FunctionDef<'db>),
    /// Struct definition
    Struct(StructDef<'db>),
    /// Namespace definition
    Namespace(Namespace<'db>),
    /// Import statement
    Import(ImportStmt<'db>),
    /// Constant definition
    Const(ConstDef<'db>),
}

/// Represents a constant definition.
///
/// Constants are immutable values that are defined once and can be
/// referenced throughout the program.
#[salsa::interned(debug)]
pub struct ConstDef<'db> {
    /// The constant's name
    pub name: String,
    /// The constant's value expression
    pub value: Expression,
}

/// Represents a function definition.
#[salsa::interned(debug)]
pub struct FunctionDef<'db> {
    /// The function's name
    pub name: String,
    /// The function's parameters
    pub params: Vec<Parameter<'db>>,
    /// The function's return type (optional)
    pub return_type: Option<TypeExpr>,
    /// The function's body (list of statements)
    pub body: Vec<Statement<'db>>,
}

/// Represents a struct definition.
#[salsa::interned(debug)]
pub struct StructDef<'db> {
    /// The struct's name
    pub name: String,
    /// The struct's fields (name and type pairs)
    pub fields: Vec<(String, TypeExpr)>,
}

/// Represents a namespace definition.
///
/// Namespaces provide a way to organize related functions, types,
/// and constants under a common name, preventing naming conflicts.
#[salsa::interned(debug)]
pub struct Namespace<'db> {
    /// The namespace's name
    pub name: String,
    /// The items contained within the namespace
    pub body: Vec<TopLevelItem<'db>>,
}

/// Represents an import statement.
///
/// Import statements allow code to reference items from other modules
/// or namespaces, with optional aliasing for name resolution.
#[salsa::interned(debug)]
pub struct ImportStmt<'db> {
    /// The path to the module (e.g., `["std", "math"]` for `std.math`)
    pub path: Vec<String>,
    /// The specific item being imported
    pub item: String,
    /// Optional alias for the imported item
    pub alias: Option<String>,
}

/// Helper enum for handling postfix operations during expression parsing.
///
/// This is used internally by the parser to handle chained operations
/// like `obj.field().index[0]` in a left-associative manner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PostfixOp {
    /// Function call with arguments
    Call(Vec<Expression>),
    /// Member access with field name
    Member(String),
    /// Index access with index expression
    Index(Expression),
}
