//! # Semantic Index
//!
//! This module defines the main semantic analysis data structure and query.
//! The SemanticIndex contains all semantic information for a source file,
//! including scopes, places, and their relationships.
//!
//! ## Architecture & Completeness
//!
//! The SemanticIndex provides **complete information** for scope and type checking:
//!
//! ### ✅ **Scope Analysis Support**
//! - **Hierarchical Scopes**: Complete parent-child scope relationships
//! - **Symbol Tables**: Per-scope symbol tables with usage tracking
//! - **Name Resolution**: Full scope chain traversal for identifier resolution
//! - **Use-Def Chains**: Complete tracking from identifier usage to definition
//! - **Span Mapping**: Precise source location to scope mapping for IDE features
//!
//! ### ✅ **Type System Integration**
//! - **Expression Tracking**: Every AST expression gets metadata with scope context
//! - **AST Preservation**: Original AST nodes preserved for type inference
//! - **Definition Metadata**: Complete symbol information with type expressions
//! - **Expression IDs**: Unique identifiers enabling type caching and validation
//!
//! ### ✅ **Validation Infrastructure**
//! - **Comprehensive Coverage**: All expression types and statements handled
//! - **Error Recovery**: Graceful handling of semantic errors
//! - **Extensible Validators**: Plugin-based validation system
//! - **IDE Integration**: Rich diagnostic information with precise locations
//!
//! ## Implementation Quality
//!
//! ### **Two-Pass Analysis** ✅
//! 1. **Pass 1**: Declaration collection (enables forward references)
//! 2. **Pass 2**: Body processing (handles nested scopes correctly)
//!
//! ### **Robust Traversal** ✅
//! - All AST node types properly visited
//! - Stack-based scope management with automatic cleanup
//! - Recursive expression handling with full coverage
//! - Proper span tracking for all constructs
//!
//! ### **Performance Optimized** ✅
//! - O(1) scope and symbol lookups via IndexVec
//! - HashMap-based span mappings for fast location queries
//! - Salsa integration for incremental compilation
//! - Memory-efficient indexed data structures

use std::collections::HashMap;

use cairo_m_compiler_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCollection, DiagnosticSink, VecSink,
};
use cairo_m_compiler_parser::parser::{
    ConstDef, Expression, FunctionDef, NamedType, Parameter, Pattern, Spanned, Statement,
    StructDef, TopLevelItem, TypeExpr, UseItems, UseStmt,
};
use cairo_m_compiler_parser::ParsedModule;
use chumsky::span::SimpleSpan;
use index_vec::IndexVec;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definition::{DefinitionKind, ParameterDefRef, UseDefRef};
use crate::place::{FileScopeId, Scope};
use crate::semantic_errors::{SemanticSyntaxChecker, SemanticSyntaxContext};
use crate::visitor::{walk_type_expr, Visitor};
// TypeUsage is defined in this module; refer to it directly
use crate::{module_semantic_index, Crate, Definition, File, SemanticDb};

// Define DefinitionIndex as an index type for definitions within a single file.
index_vec::define_index_type! {
    /// Unique identifier for a definition within a single file.
    pub struct DefinitionIndex = usize;
}

/// A globally unique identifier for a definition.
/// It combines the file with a file-local index.
#[salsa::interned(debug)]
pub struct DefinitionId {
    pub file: File,
    #[id]
    pub id_in_file: DefinitionIndex,
}

// Define ExpressionId as an index type
index_vec::define_index_type! {
    /// Unique identifier for an AST expression node within a file, used for type inference.
    pub struct ExpressionId = usize;
}

/// Information about an identifier usage site
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentifierUsage {
    /// The name of the identifier
    pub name: String,
    /// The span of the identifier in the source
    pub span: SimpleSpan<usize>,
    /// The scope where this identifier is used
    pub scope_id: FileScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeUsage {
    pub name: String,
    pub span: SimpleSpan<usize>,
    pub scope_id: FileScopeId,
}

/// Kind of condition in control flow statements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionKind {
    If,
    While,
    For,
}

impl std::fmt::Display for ConditionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::If => write!(f, "if"),
            Self::While => write!(f, "while"),
            Self::For => write!(f, "for"),
        }
    }
}

/// Origin information tracking where an expression comes from within its parent context
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Regular expression not within a specific structural context
    Plain,
    /// Expression is a field value within a struct literal
    StructField {
        parent: ExpressionId,
        field: String,
        field_span: SimpleSpan<usize>,
    },
    /// Expression is an element within a tuple literal
    TupleElem { parent: ExpressionId, index: usize },
    /// Expression is an element within an array literal
    ArrayElem { parent: ExpressionId, index: usize },
    /// Expression is a function argument.
    Arg { callee: ExpressionId, index: usize },
    /// Expression is the RHS of an assignment.
    AssignmentRhs { lhs: ExpressionId },
    /// Expression is in a return statement.
    ReturnExpr,
    /// Expression is a condition in a control-flow statement.
    Condition { kind: ConditionKind },
}

/// Information about an expression node in the AST
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionInfo {
    /// File containing this expression
    pub file: File,
    /// The actual AST node for direct access without lookup
    pub ast_node: Expression,
    /// Span of the original AST node for diagnostics
    pub ast_span: SimpleSpan<usize>,
    /// Scope in which this expression occurs
    pub scope_id: FileScopeId,
    /// The expected type of the expression, if any
    pub expected_type_ast: Option<Spanned<TypeExpr>>,
    /// Origin information tracking where this expression comes from
    pub origin: Origin,
}

/// The main semantic analysis result for a source file
///
/// This contains all semantic information derived from the AST,
/// including scopes, symbol tables, and relationships between them.
///
/// # Architecture
///
/// The SemanticIndex follows a layered approach:
/// 1. **Scopes**: Hierarchical containers for symbols
/// 2. **Place Tables**: Symbol tables for each scope
/// 3. **Definitions**: Links between symbols and their AST nodes
/// 4. **Use-Def Chains**: Tracks how identifiers resolve to definitions
///
/// # Performance Considerations
///
/// This structure is designed for efficient lookup operations during
/// validation and IDE features like go-to-definition.
///
/// Note: Currently stores SemanticIndex directly. For better performance with large projects,
/// this could be changed to store Arc<SemanticIndex> to avoid cloning when sharing between threads.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProjectSemanticIndex {
    pub modules: HashMap<String, SemanticIndex>,
}

impl ProjectSemanticIndex {
    pub const fn new(modules: HashMap<String, SemanticIndex>) -> Self {
        Self { modules }
    }

    pub const fn modules(&self) -> &HashMap<String, SemanticIndex> {
        &self.modules
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SemanticIndex {
    /// **Scope hierarchy**: List of all scopes in this file with parent-child relationships.
    ///
    /// Defines the hierarchical structure of scopes (module -> function -> block).
    /// Each scope knows its parent, enabling scope chain traversal for symbol resolution.
    /// The root (module) scope has `parent: None`.
    ///
    /// **Used by**: Name resolution (walking up scope chain), scope validation, IDE navigation
    /// **Indexed by**: `FileScopeId` - each scope can be directly accessed
    scopes: IndexVec<FileScopeId, Scope>,

    /// **IDE support**: Maps AST node spans to their containing scope for precise location queries.
    ///
    /// Critical for IDE features and error reporting that need to determine which scope
    /// a specific source position belongs to. When a user hovers over code at position X,
    /// this map tells us which scope that position is in, enabling context-aware features.
    ///
    /// **Used by**: Error reporting, hover info, go-to-definition, autocomplete
    /// **Key**: Source span of AST nodes, **Value**: The scope containing that span
    span_to_scope_id: FxHashMap<SimpleSpan<usize>, FileScopeId>,

    /// **Symbol definitions**: All symbol definitions in the file with their metadata.
    ///
    /// Contains the complete definition information for every symbol, linking semantic
    /// places back to their AST nodes. Each definition includes the symbol's name,
    /// location spans, type information, and AST references for code generation.
    ///
    /// **Used by**: Go-to-definition, symbol search, code refactoring, type checking
    /// **Indexed by**: `DefinitionIndex` - enables efficient definition lookup
    definitions: IndexVec<DefinitionIndex, Definition>,

    /// **Use-def chains**: Maps identifier usage sites to their resolved definitions.
    ///
    /// For every identifier usage in the code, this tracks which definition it resolves to.
    /// This is the core of semantic analysis - connecting uses to their definitions across
    /// scope boundaries. Essential for rename refactoring, dead code detection, etc.
    ///
    /// **Used by**: Rename refactoring, find all references, dead code analysis, validation
    /// **Key**: Index into `identifier_usages`, **Value**: The definition this usage resolves to
    uses: FxHashMap<usize, DefinitionIndex>,

    /// **Usage tracking**: All identifier usage sites with location and scope context.
    ///
    /// Records every place an identifier is used (not defined), with its exact source
    /// location and the scope it appears in. Combined with `uses`, this provides complete
    /// use-def information. Unresolved usages (not in `uses` map) are undeclared variables.
    ///
    /// **Used by**: Undeclared variable detection, unused variable analysis, find references
    /// **Indexed by**: Vector index (used as key in `uses` map)
    identifier_usages: Vec<IdentifierUsage>,

    /// **Type usage tracking**: All type usage sites with location and scope context.
    ///
    /// Records every place a type is used (not defined), with its exact source
    /// location and the scope it appears in. Combined with `uses`, this provides complete
    /// use-def information. Unresolved usages (not in `uses` map) are undeclared types.
    ///
    /// **Used by**: Undeclared type detection, unused type analysis, find references
    /// **Indexed by**: Vector index (used as key in `uses` map)
    type_usages: Vec<TypeUsage>,

    // TODO: is this the optimal approach?
    /// Type usage lookup: Maps a type usage index to its definition index
    type_usage_to_definition: FxHashMap<usize, DefinitionIndex>,

    /// **Type inference support**: Information about each expression node for type checking.
    ///
    /// Every expression in the AST gets an entry here with its AST node, scope context,
    /// and location information. This is essential for type inference, as the type system
    /// needs to analyze expressions within their proper scope context.
    ///
    /// **Used by**: Type inference, expression validation, IDE type information
    /// **Indexed by**: `ExpressionId` - unique identifier for each expression
    expressions: IndexVec<ExpressionId, ExpressionInfo>,

    /// **Expression lookup**: Fast mapping from source spans to expression IDs.
    ///
    /// Enables efficient lookup of expression metadata when given an AST node's span.
    /// Type checking and validation systems use this to get expression IDs from AST nodes,
    /// then use those IDs to look up full expression information and inferred types.
    ///
    /// **Used by**: Type inference, validators, IDE features on expressions
    /// **Key**: Source span of expression AST nodes, **Value**: Expression ID
    span_to_expression_id: FxHashMap<SimpleSpan<usize>, ExpressionId>,

    /// Per-scope name index: maps a name to the ordered list of DefinitionIndices (oldest → newest).
    name_index: IndexVec<FileScopeId, FxHashMap<String, Vec<DefinitionIndex>>>,

    /// Track used definitions (read or write).
    used_definitions: FxHashSet<DefinitionIndex>,

    /// Mapping from identifier expression IDs to their usage indices.
    ///
    /// This links each identifier expression directly to the recorded usage entry,
    /// enabling downstream systems (like type inference) to reuse the builder's
    /// single-source-of-truth use-def mapping rather than re-resolving names.
    identifier_expr_to_usage: FxHashMap<ExpressionId, usize>,

    /// **Import tracking**: All use statements in the file with their scope context.
    ///
    /// Records all `use` statements with the scope they appear in and the imported items.
    /// This is used for cross-module name resolution to check if an unresolved name
    /// might be imported from another module.
    ///
    /// **Used by**: Cross-module name resolution, import validation
    /// **Key**: Scope where the use statement appears, **Value**: The imported item info
    pub(crate) imports: Vec<(FileScopeId, crate::definition::UseDefRef)>,

    /// **Semantic errors**: All semantic errors collected while building the index.
    pub semantic_syntax_errors: DiagnosticCollection,
}

impl SemanticIndex {
    pub(crate) fn new() -> Self {
        Self {
            scopes: IndexVec::new(),
            span_to_scope_id: FxHashMap::default(),
            definitions: IndexVec::new(),
            uses: FxHashMap::default(),
            identifier_usages: Vec::new(),
            type_usages: Vec::new(),
            type_usage_to_definition: FxHashMap::default(),
            expressions: IndexVec::new(),
            span_to_expression_id: FxHashMap::default(),
            name_index: IndexVec::new(),
            used_definitions: FxHashSet::default(),
            identifier_expr_to_usage: FxHashMap::default(),
            imports: Vec::new(),
            semantic_syntax_errors: Default::default(),
        }
    }

    /// Add a new scope and return its ID
    pub(crate) fn add_scope(&mut self, scope: Scope) -> FileScopeId {
        let scope_id = FileScopeId::new(self.scopes.len());
        self.scopes.push(scope);
        self.name_index.push(FxHashMap::default());
        scope_id
    }

    /// Get a scope by ID
    pub fn scope(&self, id: FileScopeId) -> Option<&Scope> {
        self.scopes.get(id.as_usize())
    }

    /// Map a source span to its containing scope
    ///
    /// This is used during semantic analysis to track which scope each AST node belongs to.
    /// The mapping enables IDE features and error reporting that need scope context.
    ///
    /// # Example
    /// ```rust,ignore
    /// // During function processing
    /// let func_span = function_def.span();
    /// let func_scope_id = builder.push_scope(ScopeKind::Function);
    /// builder.index.set_scope_for_span(func_span, func_scope_id);
    /// ```
    pub(crate) fn set_scope_for_span(&mut self, span: SimpleSpan<usize>, scope_id: FileScopeId) {
        self.span_to_scope_id.insert(span, scope_id);
    }

    /// Get the scope containing a given source span
    ///
    /// This is used by IDE features and error reporting to determine which scope
    /// a particular source location belongs to.
    ///
    /// # Example
    /// ```rust,ignore
    /// // For error reporting
    /// let error_span = SimpleSpan::from(error_start..error_end);
    /// if let Some(scope_id) = index.scope_for_span(error_span) {
    ///     let scope = index.scope(scope_id).unwrap();
    ///     println!("Error in {} scope", scope.kind);
    /// }
    ///
    /// // For IDE hover/completion
    /// let cursor_span = SimpleSpan::from(cursor_pos..cursor_pos);
    /// let scope_id = index.scope_for_span(cursor_span)?;
    /// ```
    pub fn scope_for_span(&self, span: SimpleSpan<usize>) -> Option<FileScopeId> {
        self.span_to_scope_id.get(&span).copied()
    }

    /// Get all scopes in the file
    pub fn scopes(&self) -> impl Iterator<Item = (FileScopeId, &Scope)> {
        self.scopes
            .iter()
            .enumerate()
            .map(|(i, scope)| (FileScopeId::new(i), scope))
    }

    /// Find the root (module) scope
    pub fn root_scope(&self) -> Option<FileScopeId> {
        self.scopes()
            .find(|(_, scope)| scope.parent.is_none())
            .map(|(id, _)| id)
    }

    /// Get child scopes of a given scope
    pub fn child_scopes(&self, parent_id: FileScopeId) -> impl Iterator<Item = FileScopeId> + '_ {
        self.scopes().filter_map(move |(id, scope)| {
            if scope.parent == Some(parent_id) {
                Some(id)
            } else {
                None
            }
        })
    }

    /// Resolve a name as of a specific source position by walking up the scope chain,
    /// respecting temporal ordering for shadowed variables. The returned definition must
    /// have been declared no later than `position.start`, and `let` initializers that
    /// contain this usage are skipped to avoid self-referential cycles.
    pub fn resolve_name_at_position(
        &self,
        name: &str,
        starting_scope: FileScopeId,
        position: SimpleSpan<usize>,
    ) -> Option<(DefinitionIndex, &Definition)> {
        let mut current_scope = Some(starting_scope);

        while let Some(scope_id) = current_scope {
            // Lookup definition indices for `name` in this scope (oldest → newest), iterate newest-first
            if let Some(map) = self.name_index.get(scope_id.as_usize())
                && let Some(def_indices) = map.get(name)
            {
                for &def_idx in def_indices.iter().rev() {
                    if let Some(def) = self.definition(def_idx) {
                        // Skip if declared after position for local (non-top-level) definitions only.
                        // Forward references are allowed for top-level Function/Struct/Use.
                        let is_top_level_allowed = matches!(
                            def.kind,
                            crate::definition::DefinitionKind::Function(_)
                                | crate::definition::DefinitionKind::Struct(_)
                                | crate::definition::DefinitionKind::Use(_)
                        );
                        if !is_top_level_allowed && def.full_span.start > position.start {
                            continue;
                        }

                        // Skip if usage falls inside this let initializer
                        let skip =
                            if let crate::definition::DefinitionKind::Let(let_ref) = &def.kind {
                                if let Some(init_expr_id) = let_ref.value_expr_id {
                                    if let Some(init_expr) = self.expression(init_expr_id) {
                                        let init_span = init_expr.ast_span;
                                        position.start >= init_span.start
                                            && position.end <= init_span.end
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                        if skip {
                            continue;
                        }

                        return Some((def_idx, def));
                    }
                }
            }

            // Move to parent scope
            current_scope = self.scope(scope_id).and_then(|s| s.parent);
        }

        None
    }

    /// Resolve a name with cross-module support, using position-aware local resolution first.
    /// For local resolution, respects temporal ordering and let-initializer exclusion.
    ///
    /// Note: imported-module resolution is intended for top-level items only
    /// (Function/Struct/Use). It is not position-aware within the imported file
    /// and allows forward references for those top-level kinds.
    pub fn resolve_name_with_imports_at_position(
        &self,
        db: &dyn SemanticDb,
        crate_id: Crate,
        file: File,
        name: &str,
        starting_scope: FileScopeId,
        position: SimpleSpan<usize>,
    ) -> Option<(DefinitionIndex, crate::Definition, File)> {
        // Try local, position-aware
        if let Some((def_idx, def)) = self.resolve_name_at_position(name, starting_scope, position)
        {
            if !matches!(def.kind, DefinitionKind::Use(_)) {
                return Some((def_idx, def.clone(), file));
            }
            // If it's an import, let the import resolution handle it below
        }

        // Else, check imports visible from this scope
        let imports = self.get_imports_in_scope(starting_scope);
        for use_def_ref in imports {
            if use_def_ref.item.value() == name {
                let imported_module_index = module_semantic_index(
                    db,
                    crate_id,
                    use_def_ref.imported_module.value().clone(),
                )
                .ok()?;
                if let Some(imported_root) = imported_module_index.root_scope() {
                    if let Some(imported_def_idx) =
                        imported_module_index.latest_definition_index_by_name(imported_root, name)
                    {
                        let imported_def = imported_module_index
                            .definition(imported_def_idx)
                            .expect("Definition should exist in imported module");
                        if let Some(imported_file) = crate_id
                            .modules(db)
                            .get(use_def_ref.imported_module.value())
                        {
                            return Some((imported_def_idx, imported_def.clone(), *imported_file));
                        }
                    }
                }
            }
        }

        None
    }

    /// Get imports visible from a specific scope
    pub(crate) fn get_imports_in_scope(&self, scope_id: FileScopeId) -> Vec<&UseDefRef> {
        // Get all imports in the current scope and parent scopes
        let mut imports = Vec::new();
        let mut current_scope = Some(scope_id);

        while let Some(scope) = current_scope {
            // Add imports from current scope
            for (import_scope, use_def_ref) in &self.imports {
                if *import_scope == scope {
                    imports.push(use_def_ref);
                }
            }

            // Move to parent scope
            current_scope = self.scope(scope).and_then(|s| s.parent);
        }

        imports
    }

    /// Add a definition to the index
    pub(crate) fn add_definition(&mut self, definition: Definition) -> DefinitionIndex {
        self.definitions.push(definition)
    }

    /// Get all definitions in a specific scope
    pub fn definitions_in_scope(
        &self,
        scope_id: FileScopeId,
    ) -> impl Iterator<Item = (DefinitionIndex, &Definition)> + '_ {
        self.definitions
            .iter_enumerated()
            .filter(move |(_, def)| def.scope_id == scope_id)
    }

    /// Get all definitions in the file
    pub fn all_definitions(&self) -> impl Iterator<Item = (DefinitionIndex, &Definition)> + '_ {
        self.definitions.iter_enumerated()
    }

    // Definition-based name lookup helpers
    /// Return the most recent (shadowing-aware) DefinitionIndex for `name` in `scope_id`.
    /// Backed by the per-scope `name_index` (oldest → newest), iterating newest-first.
    pub fn latest_definition_index_by_name(
        &self,
        scope_id: FileScopeId,
        name: &str,
    ) -> Option<DefinitionIndex> {
        self.name_index
            .get(scope_id.as_usize())
            .and_then(|m| m.get(name))
            .and_then(|v| v.last().copied())
    }

    /// Return all DefinitionIndices for `name` in `scope_id`, ordered oldest to newest.
    pub fn all_definition_indices_by_name(
        &self,
        scope_id: FileScopeId,
        name: &str,
    ) -> Vec<DefinitionIndex> {
        self.name_index
            .get(scope_id.as_usize())
            .and_then(|m| m.get(name).cloned())
            .unwrap_or_default()
    }

    /// Walk the scope chain from `starting_scope` upwards and return the most recent
    /// (shadowing-aware) DefinitionIndex for `name` in the first scope that contains it.
    pub fn latest_definition_index_by_name_in_chain(
        &self,
        starting_scope: FileScopeId,
        name: &str,
    ) -> Option<DefinitionIndex> {
        let mut current = Some(starting_scope);
        while let Some(scope_id) = current {
            if let Some(idx) = self.latest_definition_index_by_name(scope_id, name) {
                return Some(idx);
            }
            current = self.scope(scope_id).and_then(|s| s.parent);
        }
        None
    }

    /// Find definition by ID
    pub fn definition(&self, id: DefinitionIndex) -> Option<&Definition> {
        self.definitions.get(id)
    }

    /// Check if a definition has been used (reads or writes).
    /// Uses the centralized `used_definitions` set.
    pub fn is_definition_used(&self, def_idx: DefinitionIndex) -> bool {
        self.used_definitions.contains(&def_idx)
    }

    /// Mark a definition as used (read or write).
    pub(crate) fn mark_definition_used(&mut self, def_idx: DefinitionIndex) {
        self.used_definitions.insert(def_idx);
    }

    // definition_for_place was removed; clients should use DefinitionIndex helpers.

    /// Add an identifier usage
    pub(crate) fn add_identifier_usage(&mut self, usage: IdentifierUsage) -> usize {
        let index = self.identifier_usages.len();
        self.identifier_usages.push(usage);
        index
    }

    /// Map an identifier expression to its usage index
    pub(crate) fn map_identifier_expr_to_usage(
        &mut self,
        expr_id: ExpressionId,
        usage_index: usize,
    ) {
        self.identifier_expr_to_usage.insert(expr_id, usage_index);
    }

    /// Add a type usage
    pub(crate) fn add_type_usage(&mut self, usage: TypeUsage) -> usize {
        let index = self.type_usages.len();
        self.type_usages.push(usage);
        index
    }

    /// Add a use-def relationship
    pub(crate) fn add_use(&mut self, usage_index: usize, definition_id: DefinitionIndex) {
        self.uses.insert(usage_index, definition_id);
    }

    /// Add a type usage to definition relationship
    pub(crate) fn add_type_usage_to_definition(
        &mut self,
        usage_index: usize,
        definition_id: DefinitionIndex,
    ) {
        self.type_usage_to_definition
            .insert(usage_index, definition_id);
    }

    /// Get all identifier usages
    pub fn identifier_usages(&self) -> &[IdentifierUsage] {
        &self.identifier_usages
    }

    /// Get all type usages
    pub(crate) fn type_usages(&self) -> &[TypeUsage] {
        &self.type_usages
    }

    /// Check if an identifier usage has a corresponding definition
    pub fn is_usage_resolved(&self, usage_index: usize) -> bool {
        self.uses.contains_key(&usage_index)
    }

    /// Check if a type usage has a corresponding definition
    pub(crate) fn is_type_usage_resolved(&self, usage_index: usize) -> bool {
        self.type_usage_to_definition.contains_key(&usage_index)
    }

    /// Get the definition for a specific identifier usage
    pub fn get_use_definition(&self, usage_index: usize) -> Option<&Definition> {
        self.uses
            .get(&usage_index)
            .and_then(|def_id| self.definitions.get(*def_id))
    }

    /// Get the definition resolved for a specific identifier expression
    pub fn definition_for_identifier_expr(
        &self,
        expr_id: ExpressionId,
    ) -> Option<(DefinitionIndex, &Definition)> {
        let usage_index = *self.identifier_expr_to_usage.get(&expr_id)?;
        let def_index = *self.uses.get(&usage_index)?;
        self.definitions.get(def_index).map(|d| (def_index, d))
    }

    /// Add an expression and return its ID
    pub(crate) fn add_expression(&mut self, expression_info: ExpressionInfo) -> ExpressionId {
        let expr_id = self.expressions.push(expression_info.clone());
        self.span_to_expression_id
            .insert(expression_info.ast_span, expr_id);
        expr_id
    }

    /// Get expression info by ID
    pub fn expression(&self, id: ExpressionId) -> Option<&ExpressionInfo> {
        self.expressions.get(id)
    }

    /// Get expression ID by span
    pub fn expression_id_by_span(&self, span: SimpleSpan<usize>) -> Option<ExpressionId> {
        self.span_to_expression_id.get(&span).copied()
    }

    /// Read-only view over span→expression mappings (for tests and IDE).
    pub const fn span_expression_mappings(&self) -> &FxHashMap<SimpleSpan<usize>, ExpressionId> {
        &self.span_to_expression_id
    }

    /// Get all expressions
    pub fn all_expressions(&self) -> impl Iterator<Item = (ExpressionId, &ExpressionInfo)> + '_ {
        self.expressions.iter_enumerated()
    }
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Build semantic index from an already-parsed module
///
/// This function is useful when you already have a parsed module and want to
/// perform semantic analysis without re-parsing. It's also used internally
/// by the main `semantic_index` function.
///
/// # Note
///
/// This function performs the actual semantic analysis work by building
/// scope trees, symbol tables, and use-def chains.
pub fn semantic_index_from_module(
    db: &dyn SemanticDb,
    module: &ParsedModule,
    file: File,
    crate_id: Crate,
) -> SemanticIndex {
    let sink = VecSink::new();
    let builder = SemanticIndexBuilder::new(db, file, module, crate_id, &sink);
    let mut index = builder.build();
    // Transfer collected diagnostics to the index
    index.semantic_syntax_errors = DiagnosticCollection::new(sink.into_diagnostics());
    index
}

/// Internal builder for constructing semantic indices
///
/// This struct encapsulates the state needed during semantic analysis,
/// including the current scope stack and the index being built.
/// It implements a visitor pattern to traverse the AST and build
/// the semantic model incrementally.
///
/// # Design Notes
///
/// The builder uses a stack-based approach to track nested scopes,
/// which simplifies the implementation of scope-aware analysis.
pub(crate) struct SemanticIndexBuilder<'db, 'sink> {
    db: &'db dyn SemanticDb,
    file: File,
    module: &'db ParsedModule,
    crate_id: Crate,

    // Current building state
    index: SemanticIndex,
    /// Stack of scope IDs representing the current nesting level
    /// The top of the stack is the currently active scope
    scope_stack: Vec<FileScopeId>,
    /// Current loop nesting depth for tracking break/continue validity
    loop_depth: usize,
    /// Current expected type hint for expression inference
    expected_type_hint: Option<Spanned<TypeExpr>>,

    /// Sink for collecting diagnostics while building the semantic index.
    diagnostic_sink: &'sink dyn DiagnosticSink,
    semantic_syntax_checker: SemanticSyntaxChecker,
}

impl<'db, 'sink> SemanticSyntaxContext for SemanticIndexBuilder<'db, 'sink> {
    fn path(&self) -> &str {
        self.file.file_path(self.db)
    }
    fn report_semantic_error(&self, error: Diagnostic) {
        self.diagnostic_sink.push(error);
    }
}

impl<'db, 'sink> SemanticIndexBuilder<'db, 'sink> {
    pub(crate) fn new(
        db: &'db dyn SemanticDb,
        file: File,
        module: &'db ParsedModule,
        crate_id: Crate,
        diagnostic_sink: &'sink dyn DiagnosticSink,
    ) -> Self {
        let mut builder = Self {
            db,
            file,
            module,
            crate_id,
            index: SemanticIndex::new(),
            scope_stack: Vec::new(),
            loop_depth: 0,
            expected_type_hint: None,
            diagnostic_sink,
            semantic_syntax_checker: SemanticSyntaxChecker::default(),
        };

        // Create the root module scope
        let root_scope = Scope::new(None, crate::place::ScopeKind::Module);
        let root_scope_id = builder.index.add_scope(root_scope);
        builder.scope_stack.push(root_scope_id);

        builder
    }

    /// Build the semantic index from the module.
    /// Processes recursively all items from the root scope.
    pub(crate) fn build(mut self) -> SemanticIndex {
        // Pass 1: Declare functions and structs for forward references
        // Only functions and structs need forward declarations
        {
            for item in self.module.items() {
                match item {
                    TopLevelItem::Function(func) => self.declare_function(func),
                    TopLevelItem::Struct(struct_def) => self.declare_struct(struct_def),
                    TopLevelItem::Use(use_stmt) => self.declare_use(use_stmt),
                    // Structs, use statements, and consts will be handled in pass 2
                    _ => {}
                }
            }
        }

        // Pass 2: Process function bodies and other content
        self.visit_top_level_items(self.module.items());

        // Pop the root scope
        self.scope_stack.pop();

        self.index
    }

    /// Returns the current, last active scope.
    fn current_scope(&self) -> FileScopeId {
        *self
            .scope_stack
            .last()
            .expect("scope stack should never be empty")
    }

    /// Push a new scope onto the scope stack, which becomes the active scope.
    fn push_scope(&mut self, kind: crate::place::ScopeKind) -> FileScopeId {
        let parent = Some(self.current_scope());
        let scope = Scope::new(parent, kind);
        let scope_id = self.index.add_scope(scope);
        self.scope_stack.push(scope_id);
        scope_id
    }

    /// Pop the current scope from the scope stack. The parent scope becomes the active scope.
    fn pop_scope(&mut self) {
        self.scope_stack
            .pop()
            .expect("tried to pop from empty scope stack");
    }

    /// Execute a function with a specific expected type hint
    fn with_expected_type<F>(&mut self, hint: Option<Spanned<TypeExpr>>, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let old_hint = self.expected_type_hint.clone();
        self.expected_type_hint = hint;
        f(self);
        self.expected_type_hint = old_hint;
    }

    /// Create a new scope and execute the given function within it.
    /// The scope is popped after the function returns.
    fn with_new_scope<F>(&mut self, kind: crate::place::ScopeKind, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let _scope_id = self.push_scope(kind);
        f(self);
        self.pop_scope();
    }

    /// Add a new place to the current scope along with a definition.
    fn add_place_with_definition(
        &mut self,
        name: &str,
        def_kind: DefinitionKind,
        name_span: SimpleSpan<usize>,
        full_span: SimpleSpan<usize>,
    ) -> DefinitionIndex {
        let scope_id = self.current_scope();

        let definition = Definition {
            file: self.file,
            scope_id,
            name: name.to_string(),
            name_span,
            full_span,
            kind: def_kind,
        };

        let def_id = self.index.add_definition(definition);
        // Update name index for shadowing-aware lookups
        if let Some(map) = self.index.name_index.get_mut(scope_id.as_usize()) {
            map.entry(name.to_string()).or_default().push(def_id);
        }
        def_id
    }

    /// Declare a function without processing its body (for forward references)
    fn declare_function(&mut self, func: &Spanned<FunctionDef>) {
        use crate::definition::{DefinitionKind, FunctionDefRef};
        let func_def = func.value();
        let func_span = func.span();

        // Define the function in the current scope
        let def_kind = DefinitionKind::Function(FunctionDefRef::from_ast(func));
        self.add_place_with_definition(
            func_def.name.value(),
            def_kind,
            func_def.name.span(),
            func_span,
        );
    }

    fn declare_struct(&mut self, struct_def: &Spanned<StructDef>) {
        use crate::definition::{DefinitionKind, StructDefRef};
        let struct_def_inner = struct_def.value();
        let struct_span = struct_def.span();

        // Define the struct in the current scope
        let def_kind = DefinitionKind::Struct(StructDefRef::from_ast(struct_def));
        self.add_place_with_definition(
            struct_def_inner.name.value(),
            def_kind,
            struct_def_inner.name.span(),
            struct_span,
        );
    }

    fn declare_use(&mut self, use_stmt: &Spanned<UseStmt>) {
        use crate::definition::{DefinitionKind, UseDefRef};
        let use_inner = use_stmt.value();
        let use_span = use_stmt.span();

        let path_len = use_inner.path.len();
        if path_len < 1 {
            return;
        }

        // Get the module name from the path
        let imported_module = Spanned::new(
            use_inner
                .path
                .iter()
                .map(|s| s.value().clone())
                .collect::<Vec<_>>()
                .join("::"),
            use_span,
        );

        // Process the imported items
        match &use_inner.items {
            UseItems::Single(item_spanned) => {
                let item = item_spanned.value().clone();
                let item_span = item_spanned.span();

                let use_def_ref = UseDefRef {
                    imported_module,
                    item: item_spanned.clone(),
                };
                let def_kind = DefinitionKind::Use(use_def_ref.clone());
                let current_scope = self.current_scope();
                self.add_place_with_definition(&item, def_kind, item_span, use_span);

                // Store the import for cross-module resolution
                self.index.imports.push((current_scope, use_def_ref));
            }
            UseItems::List(items) => {
                for item_spanned in items {
                    let item = item_spanned.value().clone();
                    let item_span = item_spanned.span();

                    let use_def_ref = UseDefRef {
                        imported_module: imported_module.clone(),
                        item: item_spanned.clone(),
                    };
                    let def_kind = DefinitionKind::Use(use_def_ref.clone());
                    let current_scope = self.current_scope();
                    self.add_place_with_definition(&item, def_kind, item_span, use_span);

                    self.index.imports.push((current_scope, use_def_ref));
                }
            }
        }
    }

    fn with_semantic_checker<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SemanticSyntaxChecker, &mut Self),
    {
        let mut checker = std::mem::take(&mut self.semantic_syntax_checker);
        f(&mut checker, self);
        self.semantic_syntax_checker = checker;
    }

    /// Helper method to visit an expression with a specific origin
    fn visit_expr_with_origin<'ast>(&mut self, expr: &'ast Spanned<Expression>, origin: Origin)
    where
        'ast: 'db,
    {
        // Track expression for type inference
        let expr_info = ExpressionInfo {
            file: self.file,
            ast_node: expr.value().clone(),
            ast_span: expr.span(),
            scope_id: self.current_scope(),
            expected_type_ast: self.expected_type_hint.clone(),
            origin,
        };
        let expr_id = self.index.add_expression(expr_info);

        self.visit_expr_contents(expr, expr_id);
    }

    /// Helper method to visit the contents of an expression (the match statement part)
    fn visit_expr_contents<'ast>(&mut self, expr: &'ast Spanned<Expression>, expr_id: ExpressionId)
    where
        'ast: 'db,
    {
        match expr.value() {
            Expression::Identifier(name) => {
                let current_scope = self.current_scope();

                let usage = IdentifierUsage {
                    name: name.value().clone(),
                    span: name.span(),
                    scope_id: current_scope,
                };

                let usage_index = self.index.add_identifier_usage(usage);
                // Link this identifier expression to its usage for later type inference
                self.index
                    .map_identifier_expr_to_usage(expr_id, usage_index);

                // This is a use of an identifier - mark it as used if we can resolve it
                if let Some(def_idx) = self
                    .index
                    .latest_definition_index_by_name_in_chain(current_scope, name.value())
                {
                    // Record the use for unused-variable analysis
                    self.index.mark_definition_used(def_idx);
                    // Record the use-def relationship
                    self.index.add_use(usage_index, def_idx);
                }
                // Note: Unresolved symbols will be detected in the validation pass
            }
            Expression::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expression::UnaryOp { expr, .. } => {
                self.visit_expr(expr);
            }
            Expression::Parenthesized(inner) => {
                self.visit_expr(inner);
            }
            Expression::FunctionCall { callee, args } => {
                self.visit_expr(callee);
                // Get the callee expression ID for context
                if let Some(callee_expr_id) = self.index.expression_id_by_span(callee.span()) {
                    for (index, arg) in args.iter().enumerate() {
                        let arg_origin = Origin::Arg {
                            callee: callee_expr_id,
                            index,
                        };
                        self.visit_expr_with_origin(arg, arg_origin);
                    }
                } else {
                    // Fallback: visit arguments without special origin if callee not found
                    for arg in args {
                        self.visit_expr(arg);
                    }
                }
            }
            Expression::MemberAccess { object, .. } => {
                self.visit_expr(object);
            }
            Expression::IndexAccess { array, index } => {
                self.visit_expr(array);
                self.with_expected_type(None, |builder| {
                    builder.visit_expr(index);
                });
            }
            Expression::StructLiteral { name, fields, .. } => {
                let type_usage = TypeUsage {
                    name: name.value().to_string(),
                    span: name.span(),
                    scope_id: self.current_scope(),
                };
                let usage_index = self.index.add_type_usage(type_usage);

                // Attempt to link this type usage to its locally visible definition (may be a `use`).
                // Use non-position-aware resolution here to support forward-declared top-level items
                // until phase 1 (forward declarations) is fully in place.
                if let Some(def_idx) = self
                    .index
                    .latest_definition_index_by_name(self.current_scope(), name.value())
                {
                    self.index
                        .add_type_usage_to_definition(usage_index, def_idx);
                }

                // Get struct def to get the types of the fields.
                let maybe_def = self.index.resolve_name_with_imports_at_position(
                    self.db,
                    self.crate_id,
                    self.file,
                    name.value(),
                    self.current_scope(),
                    name.span(),
                );

                // Process fields with proper type context
                if let Some((_, def, _)) = maybe_def {
                    if let Some(struct_def) = def.kind.struct_def() {
                        // Create lookup map from field names to types
                        let struct_fields: HashMap<String, Spanned<TypeExpr>> = struct_def
                            .fields_ast
                            .iter()
                            .map(|(name, type_expr)| (name.clone(), type_expr.clone()))
                            .collect();

                        // Visit each field with its expected type and proper origin
                        for (field_name, value) in fields.iter() {
                            let field_origin = Origin::StructField {
                                parent: expr_id,
                                field: field_name.value().clone(),
                                field_span: field_name.span(),
                            };

                            if let Some(field_type) = struct_fields.get(field_name.value()) {
                                // Use with_expected_type to propagate the field type
                                self.with_expected_type(Some(field_type.clone()), |builder| {
                                    builder.visit_expr_with_origin(value, field_origin);
                                });

                                // Type cohesion check moved to type validator
                            } else {
                                // Field not found in struct - just visit with origin
                                self.visit_expr_with_origin(value, field_origin);

                                // Report field not found error
                                self.diagnostic_sink.push(
                                    Diagnostic::error(
                                        DiagnosticCode::TypeMismatch,
                                        format!(
                                            "Field `{}` not found in struct `{}`",
                                            field_name.value(),
                                            name.value()
                                        ),
                                    )
                                    .with_location(
                                        self.file.file_path(self.db).to_string(),
                                        field_name.span(),
                                    ),
                                );
                            }
                        }
                    } else {
                        // Not a struct definition - visit fields with origin but no type hints
                        for (field_name, value) in fields.iter() {
                            let field_origin = Origin::StructField {
                                parent: expr_id,
                                field: field_name.value().clone(),
                                field_span: field_name.span(),
                            };
                            self.visit_expr_with_origin(value, field_origin);
                        }
                    }
                } else {
                    // Definition not found - visit fields with origin but no type hints
                    for (field_name, value) in fields.iter() {
                        let field_origin = Origin::StructField {
                            parent: expr_id,
                            field: field_name.value().clone(),
                            field_span: field_name.span(),
                        };
                        self.visit_expr_with_origin(value, field_origin);
                    }
                }
            }
            Expression::Tuple(exprs) => {
                // If we have a tuple type hint, propagate individual element types
                let has_matching_hint =
                    self.expected_type_hint
                        .as_ref()
                        .and_then(|hint| match hint.value() {
                            TypeExpr::Tuple(element_types)
                                if element_types.len() == exprs.len() =>
                            {
                                Some(element_types.clone())
                            }
                            _ => None,
                        });

                if let Some(element_types) = has_matching_hint {
                    // Visit each element with its specific type hint and origin
                    for (index, (expr, element_type)) in
                        exprs.iter().zip(element_types.iter()).enumerate()
                    {
                        let elem_origin = Origin::TupleElem {
                            parent: expr_id,
                            index,
                        };
                        self.with_expected_type(Some(element_type.clone()), |builder| {
                            builder.visit_expr_with_origin(expr, elem_origin);
                        });
                    }
                } else {
                    // No matching tuple hint or mismatched lengths - visit with origin but no type hints
                    for (index, expr) in exprs.iter().enumerate() {
                        let elem_origin = Origin::TupleElem {
                            parent: expr_id,
                            index,
                        };
                        self.visit_expr_with_origin(expr, elem_origin);
                    }
                }
            }
            Expression::TupleIndex { tuple, .. } => {
                self.visit_expr(tuple);
            }
            Expression::ArrayLiteral(elements) => {
                // If we have an array type hint, propagate the element type to all elements
                let element_type_hint =
                    self.expected_type_hint
                        .as_ref()
                        .and_then(|hint| match hint.value() {
                            TypeExpr::FixedArray { element_type, size }
                                if *size.value() == elements.len() as u64 =>
                            {
                                Some(element_type.as_ref().clone())
                            }
                            _ => None,
                        });

                if let Some(element_type) = element_type_hint {
                    // Visit each element with the element type hint
                    for (index, elem) in elements.iter().enumerate() {
                        let elem_origin = Origin::ArrayElem {
                            parent: expr_id,
                            index,
                        };
                        self.with_expected_type(Some(element_type.clone()), |builder| {
                            builder.visit_expr_with_origin(elem, elem_origin);
                        });
                    }
                } else {
                    // No matching array hint or mismatched sizes - visit with origin but no type hints
                    for (index, elem) in elements.iter().enumerate() {
                        let elem_origin = Origin::ArrayElem {
                            parent: expr_id,
                            index,
                        };
                        self.visit_expr_with_origin(elem, elem_origin);
                    }
                }
            }
            Expression::ArrayRepeat { element, count } => {
                // If we have an array type hint with matching size, propagate element type
                let element_type_hint =
                    self.expected_type_hint
                        .as_ref()
                        .and_then(|hint| match hint.value() {
                            TypeExpr::FixedArray { element_type, size }
                                if *size.value() as usize == *count.value() as usize =>
                            {
                                Some(element_type.as_ref().clone())
                            }
                            _ => None,
                        });

                let elem_origin = Origin::ArrayElem {
                    parent: expr_id,
                    index: 0,
                };

                if let Some(elem_ty) = element_type_hint {
                    self.with_expected_type(Some(elem_ty), |builder| {
                        builder.visit_expr_with_origin(element, elem_origin);
                    });
                } else {
                    self.visit_expr_with_origin(element, elem_origin);
                }
            }
            Expression::New { elem_type, count } => {
                // Leverage existing TypeExpr visitor to record type usages (and nested types)
                self.visit_type_expr(elem_type);
                // Visit count expression normally
                self.visit_expr(count);
            }
            Expression::Cast {
                expr,
                target_type: _,
            } => {
                // Visit the expression being cast
                self.visit_expr(expr);
            }
            Expression::Literal(_, _) | Expression::BooleanLiteral(_) => {
                // Leaf nodes - no sub-expressions
            }
        }
    }
}

/// Implement the Visitor trait for SemanticIndexBuilder
impl<'db, 'sink, 'ast> Visitor<'ast> for SemanticIndexBuilder<'db, 'sink>
where
    'ast: 'db,
{
    fn visit_top_level_items(&mut self, items: &'ast [TopLevelItem]) {
        self.with_semantic_checker(|checker, builder| {
            checker.check_top_level_items(builder, items);
        });
        for item in items {
            self.visit_top_level_item(item);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
        // Map statement span to scope for IDE features
        let current_scope = self.current_scope();
        self.index.set_scope_for_span(stmt.span(), current_scope);

        match stmt.value() {
            Statement::Let {
                pattern,
                value,
                statement_type,
            } => {
                use crate::definition::{DefinitionKind, LetDefRef};
                // Visit the value expression with expected type hint
                self.with_expected_type(statement_type.clone(), |builder| {
                    builder.visit_expr(value);
                });

                // Add type info, if present
                let value_expr_id = self
                    .index
                    .expression_id_by_span(value.span())
                    .expect("expression should have been registered");

                // Visit the type expression if present
                if let Some(ty) = statement_type {
                    self.visit_type_expr(ty);
                }

                // Type cohesion and pattern validation moved to validators

                // Then handle the pattern binding
                match pattern {
                    Pattern::Identifier(name) => {
                        let def_kind = DefinitionKind::Let(LetDefRef::from_let_statement(
                            name.value(),
                            statement_type.clone(),
                            Some(value_expr_id),
                        ));
                        self.add_place_with_definition(
                            name.value(),
                            def_kind,
                            name.span(),
                            stmt.span(),
                        );
                    }
                    Pattern::Tuple(patterns) => {
                        // Helper to flatten nested patterns and collect all identifiers with their paths
                        fn collect_pattern_identifiers(
                            pattern: &Pattern,
                            path: Vec<usize>,
                        ) -> Vec<(Spanned<String>, Vec<usize>)> {
                            match pattern {
                                Pattern::Identifier(name) => vec![(name.clone(), path)],
                                Pattern::Tuple(patterns) => {
                                    let mut result = Vec::new();
                                    for (i, p) in patterns.iter().enumerate() {
                                        let mut new_path = path.clone();
                                        new_path.push(i);
                                        result.extend(collect_pattern_identifiers(p, new_path));
                                    }
                                    result
                                }
                            }
                        }

                        let identifiers =
                            collect_pattern_identifiers(&Pattern::Tuple(patterns.clone()), vec![]);
                        for (name, path) in identifiers.iter() {
                            let def_kind =
                                DefinitionKind::Let(LetDefRef::from_nested_destructuring(
                                    name.value(),
                                    statement_type.clone(),
                                    value_expr_id,
                                    path.clone(),
                                ));
                            self.add_place_with_definition(
                                name.value(),
                                def_kind,
                                name.span(),
                                stmt.span(),
                            );
                        }
                    }
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                // Handle control flow analysis
                self.visit_expr_with_origin(
                    condition,
                    Origin::Condition {
                        kind: ConditionKind::If,
                    },
                );

                // Visit then branch in a new scope
                self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                    builder.visit_stmt(then_block);
                });

                // Visit else branch in a new scope
                if let Some(else_stmt) = else_block {
                    self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                        builder.visit_stmt(else_stmt);
                    });
                }
            }
            Statement::Block(statements) => {
                // Create new scope for block statements
                self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                    let current_scope = builder.current_scope();
                    builder.index.set_scope_for_span(stmt.span(), current_scope);

                    for stmt in statements {
                        builder.visit_stmt(stmt);
                    }
                });
            }
            Statement::Loop { body } => {
                // Create a new scope for the loop body
                self.with_new_scope(
                    crate::place::ScopeKind::Loop {
                        depth: self.loop_depth,
                    },
                    |builder| {
                        let current_scope = builder.current_scope();
                        builder.index.set_scope_for_span(stmt.span(), current_scope);

                        builder.loop_depth += 1;
                        builder.visit_stmt(body);
                        builder.loop_depth -= 1;
                    },
                );
            }
            Statement::While { condition, body } => {
                // Visit the condition expression
                self.visit_expr_with_origin(
                    condition,
                    Origin::Condition {
                        kind: ConditionKind::While,
                    },
                );

                // Create a new scope for the while loop body
                self.with_new_scope(
                    crate::place::ScopeKind::Loop {
                        depth: self.loop_depth,
                    },
                    |builder| {
                        let current_scope = builder.current_scope();
                        builder.index.set_scope_for_span(stmt.span(), current_scope);

                        builder.loop_depth += 1;
                        builder.visit_stmt(body);
                        builder.loop_depth -= 1;
                    },
                );
            }
            Statement::For {
                init,
                condition,
                step,
                body,
            } => {
                // Create a new loop scope, just like for `loop` / `while`
                self.with_new_scope(
                    crate::place::ScopeKind::Loop {
                        depth: self.loop_depth,
                    },
                    |builder| {
                        let current_scope = builder.current_scope();
                        builder.index.set_scope_for_span(stmt.span(), current_scope);

                        // 1. Initialization part (executes once, vars live in loop scope)
                        builder.visit_stmt(init);

                        // 2. Condition expression, tracked as a control-flow condition
                        builder.visit_expr_with_origin(
                            condition,
                            Origin::Condition {
                                kind: ConditionKind::For,
                            },
                        );

                        // 3. Body (inside loop, so break/continue are valid)
                        builder.loop_depth += 1;
                        builder.visit_stmt(body);

                        // 4. Step statement (runs after each iteration, still in loop scope)
                        builder.visit_stmt(step);
                        builder.loop_depth -= 1;
                    },
                );
            }
            Statement::Break | Statement::Continue => {
                // Loop control flow validation moved to control flow validator
            }
            Statement::Const(const_def) => {
                use crate::definition::{ConstDefRef, DefinitionKind};
                // Handle const definitions inline like the original implementation
                // Map the const's span to its scope for IDE features
                let current_scope = self.current_scope();
                self.index.set_scope_for_span(stmt.span(), current_scope);

                // Process the value expression with optional type annotation
                if let Some(ref ty) = const_def.ty {
                    self.with_expected_type(Some(ty.clone()), |builder| {
                        builder.visit_expr(&const_def.value);
                    });
                } else {
                    self.visit_expr(&const_def.value);
                }
                let value_expr_id = self
                    .index
                    .expression_id_by_span(const_def.value.span())
                    .expect("expression should have been registered");

                // Define the constant
                let def_kind = DefinitionKind::Const(ConstDefRef {
                    name: const_def.name.value().clone(),
                    type_ast: const_def.ty.clone(),
                    value_expr_id: Some(value_expr_id),
                });
                self.add_place_with_definition(
                    const_def.name.value(),
                    def_kind,
                    const_def.name.span(),
                    stmt.span(),
                );
            }
            Statement::Assignment { lhs, rhs } => {
                self.visit_expr(lhs);
                // Get the lhs expression ID to provide context for the RHS
                if let Some(lhs_expr_id) = self.index.expression_id_by_span(lhs.span()) {
                    self.visit_expr_with_origin(rhs, Origin::AssignmentRhs { lhs: lhs_expr_id });
                } else {
                    // Fallback if lhs expression ID not found
                    self.visit_expr(rhs);
                }
            }
            Statement::Return { value } => {
                if let Some(expr) = value {
                    self.visit_expr_with_origin(expr, Origin::ReturnExpr);
                }
            }
            Statement::Expression(spanned) => {
                self.visit_expr(spanned);
            }
        }
    }

    fn visit_expr(&mut self, expr: &'ast Spanned<Expression>) {
        self.visit_expr_with_origin(expr, Origin::Plain);
    }

    /// When visiting function bodies, the builder propagates the function's
    /// return type as an expected type hint, enabling literals in return
    /// statements to infer their type from the function signature.
    fn visit_function(&mut self, func: &'ast Spanned<FunctionDef>) {
        let func_def = func.value();

        // Note: Function declaration already handled in pass 1
        // Here we process the body

        // Create a new scope for the function body
        self.push_scope(crate::place::ScopeKind::Function);
        let current_scope = self.current_scope();
        self.index.set_scope_for_span(func.span(), current_scope);

        // Parameter validation moved to validators

        // Visit the return type
        self.visit_type_expr(&func_def.return_type);

        // Visit parameters (they don't need the return type hint)
        self.visit_parameters(&func_def.params);

        // Visit function body normally - return type hint will be applied only to return statements
        self.visit_body(&func_def.body);

        self.pop_scope();
    }

    fn visit_parameter(&mut self, param: &'ast Parameter) {
        // Visit the parameter type
        self.visit_type_expr(&param.type_expr);

        let def_kind = DefinitionKind::Parameter(ParameterDefRef::from_ast(param));
        self.add_place_with_definition(
            param.name.value(),
            def_kind,
            param.name.span(),
            param.name.span(),
        );
    }

    fn visit_struct(&mut self, struct_def: &'ast Spanned<StructDef>) {
        // The struct is forward-declared - so we don't need to add it to definitions.
        let struct_def_inner = struct_def.value();

        // Struct field validation moved to validators

        // Visit type expressions for all fields
        for (_, type_expr) in &struct_def_inner.fields {
            self.visit_type_expr(type_expr);
        }
    }

    // TODO: not ideal design?
    // Empty impl as the use statements must be processed in pass 1.
    fn visit_use(&mut self, _use_stmt: &'ast Spanned<UseStmt>) {}

    fn visit_const(&mut self, const_def: &'ast Spanned<ConstDef>) {
        use crate::definition::{ConstDefRef, DefinitionKind};

        let const_def_inner = const_def.value();
        let const_span = const_def.span();

        // Map the const's span to its scope for IDE features
        let current_scope = self.current_scope();
        self.index.set_scope_for_span(const_span, current_scope);

        // Visit the value expression with optional type annotation
        if let Some(ref ty) = const_def_inner.ty {
            self.with_expected_type(Some(ty.clone()), |builder| {
                builder.visit_expr(&const_def_inner.value);
            });
        } else {
            self.visit_expr(&const_def_inner.value);
        }
        let value_expr_id = self
            .index
            .expression_id_by_span(const_def_inner.value.span())
            .expect("expression should have been registered");

        // Define the constant in the current scope
        let def_kind = DefinitionKind::Const(ConstDefRef::from_ast(const_def, Some(value_expr_id)));
        self.add_place_with_definition(
            const_def_inner.name.value(),
            def_kind,
            const_def_inner.name.span(),
            const_span,
        );
    }

    fn visit_type_expr(&mut self, type_expr: &'ast Spanned<TypeExpr>) {
        match type_expr.value() {
            TypeExpr::Named(named_type_spanned) => {
                if let NamedType::Custom(name) = named_type_spanned.value() {
                    let current_scope = self.current_scope();
                    let usage = TypeUsage {
                        name: name.clone(),
                        span: named_type_spanned.span(),
                        scope_id: current_scope,
                    };

                    let usage_index = self.index.add_type_usage(usage);

                    if let Some(def_idx) = self
                        .index
                        .latest_definition_index_by_name_in_chain(current_scope, name)
                    {
                        // Mark the referenced type's definition as used
                        self.index.mark_definition_used(def_idx);
                        self.index
                            .add_type_usage_to_definition(usage_index, def_idx);
                    }
                }
            }
            _ => walk_type_expr(self, type_expr), // Default traversal for Pointer/Tuple
        }
    }
}

#[cfg(test)]
#[path = "./semantic_index_tests.rs"]
mod tests;
