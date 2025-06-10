//! # Definition Tracking
//!
//! This module defines structures for tracking definitions and linking them to AST nodes.
//! It provides the connection between syntax (AST) and semantics (places/symbols).

use crate::place::{FileScopeId, ScopedPlaceId};
use crate::File;
use cairo_m_compiler_parser::parser::{
    ConstDef, FunctionDef, ImportStmt, Namespace, Parameter, Spanned, StructDef, TypeExpr,
};
use chumsky::span::SimpleSpan;
use std::fmt;

// Import ExpressionId from semantic_index
// We'll define it here temporarily to avoid circular dependencies
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
    /// Local variable definition
    Local(LocalDefRef),
    /// Function parameter definition
    Parameter(ParameterDefRef),
    /// Import definition (imported symbol)
    Import(ImportDefRef),
    /// Namespace definition
    Namespace(NamespaceDefRef),
}

impl fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(_) => write!(f, "function"),
            Self::Struct(_) => write!(f, "struct"),
            Self::Const(_) => write!(f, "constant"),
            Self::Let(_) => write!(f, "variable"),
            Self::Local(_) => write!(f, "local variable"),
            Self::Parameter(_) => write!(f, "parameter"),
            Self::Import(_) => write!(f, "import"),
            Self::Namespace(_) => write!(f, "namespace"),
        }
    }
}

/// Reference to a function definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefRef {
    pub name: String,
    /// Parameter information with names and AST type expressions
    pub params_ast: Vec<(String, TypeExpr)>,
    /// Return type AST expression, if specified
    pub return_type_ast: Option<TypeExpr>,
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
    pub fields_ast: Vec<(String, TypeExpr)>,
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
    /// The expression ID for the constant's value (to be assigned during semantic analysis)
    pub value_expr_id: Option<ExpressionId>,
}

impl ConstDefRef {
    pub fn from_ast(const_def: &Spanned<ConstDef>) -> Self {
        Self {
            name: const_def.value().name.value().clone(),
            value_expr_id: None, // Will be set during semantic analysis
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
    pub explicit_type_ast: Option<TypeExpr>,
}

impl LetDefRef {
    pub fn from_let_statement(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value_expr_id: None,     // Will be set during semantic analysis
            explicit_type_ast: None, // TODO: Extract from let statement when parser supports it
        }
    }
}

/// Reference to a local variable definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalDefRef {
    pub name: String,
    /// The expression ID for the local's value (to be assigned during semantic analysis)
    pub value_expr_id: Option<ExpressionId>,
    /// Explicit type annotation, if provided
    pub explicit_type_ast: Option<TypeExpr>,
}

impl LocalDefRef {
    pub fn from_local_statement(name: &str, type_ast: Option<TypeExpr>) -> Self {
        Self {
            name: name.to_string(),
            value_expr_id: None, // Will be set during semantic analysis
            explicit_type_ast: type_ast,
        }
    }
}

/// Reference to a parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterDefRef {
    pub name: String,
    /// The AST type expression for this parameter
    pub type_ast: TypeExpr,
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
pub struct ImportDefRef {
    pub imported_name: String,
    pub alias: Option<String>,
    pub module_path: Vec<String>,
}

impl ImportDefRef {
    pub fn from_ast(import: &Spanned<ImportStmt>) -> Self {
        Self {
            imported_name: import.value().item.value().clone(),
            alias: import.value().alias.as_ref().map(|a| a.value().clone()),
            module_path: import
                .value()
                .path
                .iter()
                .map(|p| p.value().clone())
                .collect(),
        }
    }
}

/// Reference to a namespace definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespaceDefRef {
    pub name: String,
    pub item_count: usize,
}

impl NamespaceDefRef {
    pub fn from_ast(namespace: &Spanned<Namespace>) -> Self {
        Self {
            name: namespace.value().name.value().clone(),
            item_count: namespace.value().body.len(),
        }
    }
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
    use super::*;

    use cairo_m_compiler_parser::parser::{Expression, TypeExpr};

    #[test]
    fn test_definition_kinds() {
        use chumsky::span::SimpleSpan;

        // Test various definition kind constructors
        let func_def = FunctionDef {
            name: Spanned::new("test_func".to_string(), SimpleSpan::from(0..5)),
            params: vec![],
            return_type: Some(TypeExpr::Named("felt".to_string())),
            body: vec![],
        };
        let spanned_func = Spanned::new(func_def, SimpleSpan::from(0..10));
        let func_ref = FunctionDefRef::from_ast(&spanned_func);
        assert_eq!(func_ref.name, "test_func");
        assert!(func_ref.return_type_ast.is_some());
        assert_eq!(func_ref.params_ast.len(), 0);

        let struct_def = StructDef {
            name: Spanned::new("Point".to_string(), SimpleSpan::from(0..5)),
            fields: vec![
                (
                    Spanned::new("x".to_string(), SimpleSpan::from(6..7)),
                    TypeExpr::Named("felt".to_string()),
                ),
                (
                    Spanned::new("y".to_string(), SimpleSpan::from(8..9)),
                    TypeExpr::Named("felt".to_string()),
                ),
            ],
        };
        let spanned_struct = Spanned::new(struct_def, SimpleSpan::from(0..10));
        let struct_ref = StructDefRef::from_ast(&spanned_struct);
        assert_eq!(struct_ref.name, "Point");
        assert_eq!(struct_ref.fields_ast.len(), 2);

        let const_def = ConstDef {
            name: Spanned::new("PI".to_string(), SimpleSpan::from(0..2)),
            value: Spanned::new(Expression::Literal(314), SimpleSpan::from(3..6)),
        };
        let spanned_const = Spanned::new(const_def, SimpleSpan::from(0..10));
        let const_ref = ConstDefRef::from_ast(&spanned_const);
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
            return_type_ast: None,
        });

        let def2_kind = DefinitionKind::Const(ConstDefRef {
            name: "CONST1".to_string(),
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
            params_ast: vec![("param".to_string(), TypeExpr::Named("felt".to_string()))],
            return_type_ast: Some(TypeExpr::Named("felt".to_string())),
        });

        let definitions = Definitions::single(scope_id, place_id, def_kind);
        assert_eq!(definitions.len(), 1);

        let all_defs = definitions.all();
        assert_eq!(all_defs.len(), 1);
    }
}
