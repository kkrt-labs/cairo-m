//! # Definition Tracking
//!
//! This module defines structures for tracking definitions and linking them to AST nodes.
//! It provides the connection between syntax (AST) and semantics (places/symbols).

use std::fmt;

use cairo_m_compiler_parser::parser::{
    ConstDef, FunctionDef, Parameter, Spanned, StructDef, TypeExpr,
};
use chumsky::span::SimpleSpan;

use crate::place::{FileScopeId, ScopedPlaceId};
use crate::semantic_index::ExpressionId;
use crate::File;

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
    /// The place (symbol) this definition defines
    pub place_id: ScopedPlaceId,
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
    pub fn from_ast(func: &Spanned<FunctionDef>) -> Self {
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
    pub fn from_ast(struct_def: &Spanned<StructDef>) -> Self {
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
    pub fn from_ast(const_def: &Spanned<ConstDef>, value_expr_id: Option<ExpressionId>) -> Self {
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
    pub fn from_let_statement(
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

    pub fn from_destructuring(
        name: &str,
        explicit_type_ast: Option<Spanned<TypeExpr>>,
        value_expr_id: ExpressionId,
        index: usize,
    ) -> Self {
        Self {
            name: name.to_string(),
            value_expr_id: Some(value_expr_id),
            explicit_type_ast,
            destructuring_info: Some((value_expr_id, vec![index])),
        }
    }

    pub fn from_nested_destructuring(
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
    pub fn from_ast(param: &Parameter) -> Self {
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
    definitions: Vec<(FileScopeId, ScopedPlaceId, DefinitionKind)>,
}

impl Definitions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a definition to the collection
    pub fn add(&mut self, scope_id: FileScopeId, place_id: ScopedPlaceId, kind: DefinitionKind) {
        self.definitions.push((scope_id, place_id, kind));
    }

    /// Create a collection with a single definition
    pub fn single(scope_id: FileScopeId, place_id: ScopedPlaceId, kind: DefinitionKind) -> Self {
        Self {
            definitions: vec![(scope_id, place_id, kind)],
        }
    }

    /// Get all definitions
    pub fn all(&self) -> &[(FileScopeId, ScopedPlaceId, DefinitionKind)] {
        &self.definitions
    }

    /// Find definitions by kind
    pub fn by_kind(
        &self,
        kind_matcher: impl Fn(&DefinitionKind) -> bool,
    ) -> Vec<&(FileScopeId, ScopedPlaceId, DefinitionKind)> {
        self.definitions
            .iter()
            .filter(|(_, _, kind)| kind_matcher(kind))
            .collect()
    }

    /// Check if there are any definitions
    pub const fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Get the number of definitions
    pub const fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Iterate over all definitions
    pub fn iter(&self) -> impl Iterator<Item = &(FileScopeId, ScopedPlaceId, DefinitionKind)> + '_ {
        self.definitions.iter()
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

    fn tuple_type(elements: Vec<Spanned<TypeExpr>>) -> Spanned<TypeExpr> {
        spanned(TypeExpr::Tuple(elements))
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

    #[test]
    fn test_definitions_collection() {
        let scope_id = crate::place::FileScopeId::new(0);
        let place_id1 = crate::place::ScopedPlaceId::new(0);
        let place_id2 = crate::place::ScopedPlaceId::new(1);

        let def1_kind = DefinitionKind::Function(FunctionDefRef {
            name: "func1".to_string(),
            params_ast: vec![],
            return_type_ast: tuple_type(vec![]), // Unit type
        });

        let def2_kind = DefinitionKind::Const(ConstDefRef {
            name: "CONST1".to_string(),
            type_ast: None,
            value_expr_id: None,
        });

        let mut definitions = Definitions::new();
        definitions.add(scope_id, place_id1, def1_kind);
        definitions.add(scope_id, place_id2, def2_kind);

        assert_eq!(definitions.len(), 2);
        assert!(!definitions.is_empty());

        // Test filtering by kind
        let funcs = definitions.by_kind(|kind| matches!(kind, DefinitionKind::Function(_)));
        assert_eq!(funcs.len(), 1);

        let consts = definitions.by_kind(|kind| matches!(kind, DefinitionKind::Const(_)));
        assert_eq!(consts.len(), 1);
    }

    #[test]
    fn test_single_definition() {
        let scope_id = crate::place::FileScopeId::new(0);
        let place_id = crate::place::ScopedPlaceId::new(0);

        let def_kind = DefinitionKind::Function(FunctionDefRef {
            name: "test".to_string(),
            params_ast: vec![("param".to_string(), named_type(NamedType::Felt))],
            return_type_ast: named_type(NamedType::Felt),
        });

        let definitions = Definitions::single(scope_id, place_id, def_kind);
        assert_eq!(definitions.len(), 1);

        let all_defs = definitions.all();
        assert_eq!(all_defs.len(), 1);
    }
}
