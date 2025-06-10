//! # Definition Tracking
//!
//! This module defines structures for tracking definitions and linking them to AST nodes.
//! It provides the connection between syntax (AST) and semantics (places/symbols).

use crate::place::{FileScopeId, ScopedPlaceId};
use crate::File;
use cairo_m_compiler_parser::parser::{
    ConstDef, FunctionDef, ImportStmt, Namespace, Parameter, StructDef,
};
use std::fmt;

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
    pub scope: FileScopeId,
    /// The place (symbol) this definition defines
    pub place: ScopedPlaceId,
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
    pub parameter_count: usize,
    pub has_return_type: bool,
    // In a full implementation, you might store the actual AST node reference
    // or a stable ID that can be used to retrieve it
}

impl FunctionDefRef {
    pub fn from_ast(func: &FunctionDef) -> Self {
        Self {
            name: func.name.clone(),
            parameter_count: func.params.len(),
            has_return_type: func.return_type.is_some(),
        }
    }
}

/// Reference to a struct definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDefRef {
    pub name: String,
    pub field_count: usize,
}

impl StructDefRef {
    pub fn from_ast(struct_def: &StructDef) -> Self {
        Self {
            name: struct_def.name.clone(),
            field_count: struct_def.fields.len(),
        }
    }
}

/// Reference to a constant definition in the AST
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstDefRef {
    pub name: String,
}

impl ConstDefRef {
    pub fn from_ast(const_def: &ConstDef) -> Self {
        Self {
            name: const_def.name.clone(),
        }
    }
}

/// Reference to a let statement definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LetDefRef {
    pub name: String,
}

impl LetDefRef {
    pub fn from_let_statement(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// Reference to a local variable definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalDefRef {
    pub name: String,
    pub has_type_annotation: bool,
}

impl LocalDefRef {
    pub fn from_local_statement(name: &str, has_type: bool) -> Self {
        Self {
            name: name.to_string(),
            has_type_annotation: has_type,
        }
    }
}

/// Reference to a parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterDefRef {
    pub name: String,
    pub type_name: String, // Simplified - in full implementation would be a type reference
}

impl ParameterDefRef {
    pub fn from_ast(param: &Parameter) -> Self {
        Self {
            name: param.name.clone(),
            type_name: format!("{:?}", param.type_expr), // Simplified type representation
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
    pub fn from_ast(import: &ImportStmt) -> Self {
        Self {
            imported_name: import.item.clone(),
            alias: import.alias.clone(),
            module_path: import.path.clone(),
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
    pub fn from_ast(namespace: &Namespace) -> Self {
        Self {
            name: namespace.name.clone(),
            item_count: namespace.body.len(),
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
        // Test various definition kind constructors
        let func_def = FunctionDef {
            name: "test_func".to_string(),
            params: vec![],
            return_type: Some(TypeExpr::Named("felt".to_string())),
            body: vec![],
        };
        let func_ref = FunctionDefRef::from_ast(&func_def);
        assert_eq!(func_ref.name, "test_func");
        assert!(func_ref.has_return_type);
        assert_eq!(func_ref.parameter_count, 0);

        let struct_def = StructDef {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), TypeExpr::Named("felt".to_string())),
                ("y".to_string(), TypeExpr::Named("felt".to_string())),
            ],
        };
        let struct_ref = StructDefRef::from_ast(&struct_def);
        assert_eq!(struct_ref.name, "Point");
        assert_eq!(struct_ref.field_count, 2);

        let const_def = ConstDef {
            name: "PI".to_string(),
            value: Expression::Literal(314),
        };
        let const_ref = ConstDefRef::from_ast(&const_def);
        assert_eq!(const_ref.name, "PI");
    }

    #[test]
    fn test_definitions_collection() {
        let scope_id = crate::place::FileScopeId::new(0);
        let place_id1 = crate::place::ScopedPlaceId::new(0);
        let place_id2 = crate::place::ScopedPlaceId::new(1);

        let def1_kind = DefinitionKind::Function(FunctionDefRef {
            name: "func1".to_string(),
            parameter_count: 0,
            has_return_type: false,
        });

        let def2_kind = DefinitionKind::Const(ConstDefRef {
            name: "CONST1".to_string(),
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
            parameter_count: 1,
            has_return_type: true,
        });

        let definitions = Definitions::single(scope_id, place_id, def_kind);
        assert_eq!(definitions.len(), 1);

        let all_defs = definitions.all();
        assert_eq!(all_defs.len(), 1);
    }
}
