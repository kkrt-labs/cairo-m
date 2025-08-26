//! # AST Visitor Pattern for Semantic Analysis
//!
//! This module provides a visitor pattern implementation for traversing the Cairo-M AST.
//! Following the approach used in ruff, it separates traversal logic from semantic analysis,
//! enabling clean, extensible analysis passes.
//!
//! ## Architecture
//!
//! The visitor pattern consists of:
//! - **Visitor trait**: Defines visit methods for each AST node type
//! - **Walk functions**: Default traversal implementations that visit child nodes
//! - **Custom visitors**: Implement the trait to perform specific analyzes
//!
//! ## Usage
//!
//! ```rust,ignore
//! struct MyVisitor;
//!
//! impl<'ast> Visitor<'ast> for MyVisitor {
//!     fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
//!         // Custom logic before traversing children
//!         match stmt.value() {
//!             Statement::Let { .. } => {
//!                 // Handle let statements specially
//!             }
//!         }
//!     }
//! }
//! ```

use cairo_m_compiler_parser::parser::{
    ConstDef, Expression, FunctionDef, Parameter, Spanned, Statement, StructDef, TopLevelItem,
    TypeExpr, UseStmt,
};

/// Core visitor trait for AST traversal.
///
/// Each visit method has a default implementation that calls the corresponding
/// walk function, enabling selective overriding of traversal behavior.
pub trait Visitor<'ast> {
    fn visit_top_level_items(&mut self, items: &'ast [TopLevelItem]) {
        for item in items {
            self.visit_top_level_item(item);
        }
    }

    /// Visit a top-level item (function, struct, etc.)
    fn visit_top_level_item(&mut self, item: &'ast TopLevelItem) {
        walk_top_level_item(self, item);
    }

    /// Visit a statement node
    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>);

    /// Visit an expression node
    fn visit_expr(&mut self, expr: &'ast Spanned<Expression>);

    /// Visit a function definition
    fn visit_function(&mut self, func: &'ast Spanned<FunctionDef>);

    /// Visit a function's parameters
    fn visit_parameters(&mut self, params: &'ast Vec<Parameter>) {
        for param in params {
            self.visit_parameter(param);
        }
    }

    /// Visit a function parameter
    fn visit_parameter(&mut self, param: &'ast Parameter);

    /// Visit a struct definition
    fn visit_struct(&mut self, struct_def: &'ast Spanned<StructDef>);

    /// Visit a const definition
    fn visit_const(&mut self, const_def: &'ast Spanned<ConstDef>);

    /// Visit a use statement
    fn visit_use(&mut self, use_stmt: &'ast Spanned<UseStmt>);

    /// Visit a function body (list of statements)
    fn visit_body(&mut self, stmts: &'ast [Spanned<Statement>]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    /// Visit a type expression node
    fn visit_type_expr(&mut self, type_expr: &'ast Spanned<TypeExpr>) {
        walk_type_expr(self, type_expr);
    }
}

fn walk_top_level_item<'ast, V: Visitor<'ast> + ?Sized>(visitor: &mut V, item: &'ast TopLevelItem) {
    match item {
        TopLevelItem::Function(func) => visitor.visit_function(func),
        TopLevelItem::Struct(struct_def) => visitor.visit_struct(struct_def),
        TopLevelItem::Const(const_def) => visitor.visit_const(const_def),
        TopLevelItem::Use(use_stmt) => visitor.visit_use(use_stmt),
    }
}

/// Walk a type expression, visiting nested type expressions
pub fn walk_type_expr<'ast, V: Visitor<'ast> + ?Sized>(
    visitor: &mut V,
    type_expr: &'ast Spanned<TypeExpr>,
) {
    match type_expr.value() {
        TypeExpr::Pointer(inner) => visitor.visit_type_expr(inner),
        TypeExpr::Tuple(elements) => {
            for element in elements {
                visitor.visit_type_expr(element);
            }
        }
        TypeExpr::FixedArray { element_type, .. } => {
            visitor.visit_type_expr(element_type);
        }
        TypeExpr::Named(_) => {
            // Specific to implementing visitors.
        }
    }
}
