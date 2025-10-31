//! # Definition Tracking
//!
//! This module defines structures for tracking definitions and linking them to AST nodes.
//! It provides the connection between syntax (AST) and semantics (places/symbols).

use std::fmt;

use cairo_m_compiler_parser::parser::{
    ConstDef, FunctionDef, Parameter, Spanned, StructDef, TypeExpr,
};
use chumsky::span::SimpleSpan;

use crate::File;
use crate::place::FileScopeId;
use crate::semantic_index::ExpressionId;

/// A definition that links a semantic place to its AST node
///
/// This is the primary way to connect semantic analysis results back to
/// the original source code for error reporting, IDE features, etc.
/// This is now a plain data container - the SemanticIndex owns the collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    /// The file containing this definition
    pub file: File,
    /// The scope containing this definition
    pub scope_id: FileScopeId,
    /// The name of the defined entity
    pub name: String,
    /// The span of the name identifier
    pub name_span: SimpleSpan<usize>,
    /// The span of the entire definition statement/construct
    pub full_span: SimpleSpan<usize>,
    /// The kind of definition and reference to AST node
    pub kind: DefinitionKind,
}

/// The kind of definition, referencing the original AST node
///
/// This enum allows us to trace back from semantic analysis results
/// to the original source code location and AST structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionKind {
    /// Function definition
    Function(FunctionDefRef),
    /// Struct definition
    Struct(StructDefRef),
    /// Constant definition
    Const(ConstDefRef),
    /// Variable definition from let statement
    Let(LetDefRef),
    /// Function parameter definition
    Parameter(ParameterDefRef),
    /// Import definition (imported symbol)
    Use(UseDefRef),
    /// Loop variable definition (from for loops)
    LoopVariable(LoopVariableDefRef),
}

impl DefinitionKind {
    pub const fn struct_def(&self) -> Option<&StructDefRef> {
        match self {
            Self::Struct(def) => Some(def),
            _ => None,
        }
    }
}

impl fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(_) => write!(f, "function"),
            Self::Struct(_) => write!(f, "struct"),
            Self::Const(_) => write!(f, "constant"),
            Self::Let(_) => write!(f, "variable"),
            Self::Parameter(_) => write!(f, "parameter"),
            Self::Use(_) => write!(f, "use"),
            Self::LoopVariable(_) => write!(f, "loop variable"),
        }
    }
}

/// Reference to a function definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefRef {
    pub name: String,
    /// Parameter information with names and AST type expressions
    pub params_ast: Vec<(String, Spanned<TypeExpr>)>,
    /// Return type AST expression (defaults to unit type)
    pub return_type_ast: Spanned<TypeExpr>,
}

impl FunctionDefRef {
    pub(crate) fn from_ast(func: &Spanned<FunctionDef>) -> Self {
        Self {
            name: func.value().name.value().clone(),
            params_ast: func
                .value()
                .params
                .iter()
                .map(|param| (param.name.value().clone(), param.type_expr.clone()))
                .collect(),
            return_type_ast: func.value().return_type.clone(),
        }
    }
}

/// Reference to a struct definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDefRef {
    pub name: String,
    /// Field information with names and AST type expressions
    pub fields_ast: Vec<(String, Spanned<TypeExpr>)>,
}

impl StructDefRef {
    pub(crate) fn from_ast(struct_def: &Spanned<StructDef>) -> Self {
        Self {
            name: struct_def.value().name.value().clone(),
            fields_ast: struct_def
                .value()
                .fields
                .iter()
                .map(|(name, type_expr)| (name.value().clone(), type_expr.clone()))
                .collect(),
        }
    }
}

/// Reference to a constant definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstDefRef {
    pub name: String,
    /// Optional explicit type annotation
    pub type_ast: Option<Spanned<TypeExpr>>,
    /// The expression ID for the constant's value (to be assigned during semantic analysis)
    pub value_expr_id: Option<ExpressionId>,
}

impl ConstDefRef {
    pub(crate) fn from_ast(
        const_def: &Spanned<ConstDef>,
        value_expr_id: Option<ExpressionId>,
    ) -> Self {
        Self {
            name: const_def.value().name.value().clone(),
            type_ast: const_def.value().ty.clone(),
            value_expr_id,
        }
    }
}

/// Reference to a let statement definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LetDefRef {
    pub name: String,
    /// The expression ID for the let's value (to be assigned during semantic analysis)
    pub value_expr_id: Option<ExpressionId>,
    /// Explicit type annotation, if provided
    pub explicit_type_ast: Option<Spanned<TypeExpr>>,
    /// Destructuring information: (RHS expression ID, path to element in nested tuple)
    pub destructuring_info: Option<(ExpressionId, Vec<usize>)>,
}

impl LetDefRef {
    pub(crate) fn from_let_statement(
        name: &str,
        explicit_type_ast: Option<Spanned<TypeExpr>>,
        value_expr_id: Option<ExpressionId>,
    ) -> Self {
        Self {
            name: name.to_string(),
            value_expr_id,
            explicit_type_ast,
            destructuring_info: None,
        }
    }

    pub(crate) fn from_nested_destructuring(
        name: &str,
        explicit_type_ast: Option<Spanned<TypeExpr>>,
        value_expr_id: ExpressionId,
        path: Vec<usize>,
    ) -> Self {
        Self {
            name: name.to_string(),
            value_expr_id: Some(value_expr_id),
            explicit_type_ast,
            destructuring_info: Some((value_expr_id, path)),
        }
    }
}

/// Reference to a parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterDefRef {
    pub name: String,
    /// The AST type expression for this parameter
    pub type_ast: Spanned<TypeExpr>,
}

impl ParameterDefRef {
    pub(crate) fn from_ast(param: &Parameter) -> Self {
        Self {
            name: param.name.value().clone(),
            type_ast: param.type_expr.clone(),
        }
    }
}

/// Reference to an import definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UseDefRef {
    pub imported_module: Spanned<String>,
    pub item: Spanned<String>,
}

/// Reference to a loop variable definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LoopVariableDefRef {
    pub name: String,
}

/// Collection of definitions within a scope or file
///
/// This provides organized access to all definitions and their metadata.
/// For now, we'll keep this simple and store definition metadata directly.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Definitions {
    /// All definition metadata
    definitions: Vec<(FileScopeId, DefinitionKind)>,
}

impl Definitions {
    /// Check if there are any definitions
    pub const fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Get the number of definitions
    pub const fn len(&self) -> usize {
        self.definitions.len()
    }
}

#[cfg(test)]
mod tests {
    use cairo_m_compiler_parser::parser::{Expression, NamedType, Spanned, TypeExpr};

    use super::*;

    // Helper functions for tests
    fn spanned<T>(value: T) -> Spanned<T> {
        Spanned::new(value, SimpleSpan::from(0..0))
    }

    fn named_type(name: NamedType) -> Spanned<TypeExpr> {
        spanned(TypeExpr::Named(spanned(name)))
    }

    #[test]
    fn test_definition_kinds() {
        use chumsky::span::SimpleSpan;

        // Test various definition kind constructors
        let func_def = FunctionDef {
            name: Spanned::new("test_func".to_string(), SimpleSpan::from(0..5)),
            params: vec![],
            return_type: named_type(NamedType::Felt),
            body: vec![],
        };
        let spanned_func = Spanned::new(func_def, SimpleSpan::from(0..10));
        let func_ref = FunctionDefRef::from_ast(&spanned_func);
        assert_eq!(func_ref.name, "test_func");
        assert_eq!(func_ref.return_type_ast, named_type(NamedType::Felt));
        assert_eq!(func_ref.params_ast.len(), 0);

        let struct_def = StructDef {
            name: Spanned::new("Point".to_string(), SimpleSpan::from(0..5)),
            fields: vec![
                (
                    Spanned::new("x".to_string(), SimpleSpan::from(6..7)),
                    named_type(NamedType::Felt),
                ),
                (
                    Spanned::new("y".to_string(), SimpleSpan::from(8..9)),
                    named_type(NamedType::Felt),
                ),
            ],
        };
        let spanned_struct = Spanned::new(struct_def, SimpleSpan::from(0..10));
        let struct_ref = StructDefRef::from_ast(&spanned_struct);
        assert_eq!(struct_ref.name, "Point");
        assert_eq!(struct_ref.fields_ast.len(), 2);

        let const_def = ConstDef {
            name: Spanned::new("PI".to_string(), SimpleSpan::from(0..2)),
            ty: None,
            value: Spanned::new(Expression::Literal(314, None), SimpleSpan::from(3..6)),
        };
        let spanned_const = Spanned::new(const_def, SimpleSpan::from(0..10));
        let const_ref = ConstDefRef::from_ast(&spanned_const, None);
        assert_eq!(const_ref.name, "PI");
    }
}
