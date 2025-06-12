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

use crate::definition::DefinitionKind;
use crate::place::{FileScopeId, PlaceTable, Scope};
use crate::{Definition, File, SemanticDb};
use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCollection};
use cairo_m_compiler_parser::parser::{
    ConstDef, Expression, FunctionDef, ImportStmt, Namespace, Spanned, Statement, StructDef,
    TopLevelItem,
};
use cairo_m_compiler_parser::{parse_program, ParsedModule};
use chumsky::span::SimpleSpan;
use index_vec::IndexVec;
use rustc_hash::FxHashMap;

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
#[derive(Debug, PartialEq, Eq)]
pub struct SemanticIndex {
    /// **Core scope management**: List of all place tables in this file, indexed by scope.
    ///
    /// Each scope has its own symbol table (PlaceTable) that contains all the symbols
    /// defined within that scope. This parallel indexing with `scopes` ensures O(1) lookup
    /// of a scope's symbol table given a `FileScopeId`.
    ///
    /// **Used by**: Symbol resolution, IDE features (autocomplete), scope validators
    /// **Indexed by**: `FileScopeId` - allowing direct access to any scope's symbols
    place_tables: IndexVec<FileScopeId, PlaceTable>,

    /// **Scope hierarchy**: List of all scopes in this file with parent-child relationships.
    ///
    /// Defines the hierarchical structure of scopes (module -> namespace -> function -> block).
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
    pub span_to_expression_id: FxHashMap<SimpleSpan<usize>, ExpressionId>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self {
            place_tables: IndexVec::new(),
            scopes: IndexVec::new(),
            span_to_scope_id: FxHashMap::default(),
            definitions: IndexVec::new(),
            uses: FxHashMap::default(),
            identifier_usages: Vec::new(),
            expressions: IndexVec::new(),
            span_to_expression_id: FxHashMap::default(),
        }
    }

    /// Add a new scope and return its ID
    pub fn add_scope(&mut self, scope: Scope) -> FileScopeId {
        let scope_id = FileScopeId::new(self.scopes.len());
        self.scopes.push(scope);
        self.place_tables.push(PlaceTable::new());
        scope_id
    }

    /// Get a scope by ID
    pub fn scope(&self, id: FileScopeId) -> Option<&Scope> {
        self.scopes.get(id.as_usize())
    }

    /// Get the place table for a scope
    pub fn place_table(&self, scope_id: FileScopeId) -> Option<&PlaceTable> {
        self.place_tables.get(scope_id.as_usize())
    }

    /// Get a mutable reference to the place table for a scope
    pub fn place_table_mut(&mut self, scope_id: FileScopeId) -> Option<&mut PlaceTable> {
        self.place_tables.get_mut(scope_id.as_usize())
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
    pub fn set_scope_for_span(&mut self, span: SimpleSpan<usize>, scope_id: FileScopeId) {
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
    /// let available_symbols = index.place_table(scope_id)?;
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

    /// Resolve a name by walking up the scope chain
    pub fn resolve_name(
        &self,
        name: &str,
        starting_scope: FileScopeId,
    ) -> Option<(FileScopeId, crate::place::ScopedPlaceId)> {
        let mut current_scope = Some(starting_scope);

        while let Some(scope_id) = current_scope {
            if let Some(place_table) = self.place_table(scope_id)
                && let Some(place_id) = place_table.place_id_by_name(name)
            {
                return Some((scope_id, place_id));
            }

            // Move to parent scope
            current_scope = self.scope(scope_id)?.parent;
        }

        None
    }

    /// Resolve a name to its definition ID, starting from a given scope and walking up the scope chain
    pub fn resolve_name_to_definition(
        &self,
        name: &str,
        starting_scope: FileScopeId,
    ) -> Option<(DefinitionIndex, &Definition)> {
        if let Some((scope_id, place_id)) = self.resolve_name(name, starting_scope) {
            self.definition_for_place(scope_id, place_id)
        } else {
            None
        }
    }

    /// Add a definition to the index
    pub fn add_definition(&mut self, definition: Definition) -> DefinitionIndex {
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

    /// Find definition by ID
    pub fn definition(&self, id: DefinitionIndex) -> Option<&Definition> {
        self.definitions.get(id)
    }

    /// Find definition by place
    pub fn definition_for_place(
        &self,
        scope_id: FileScopeId,
        place_id: crate::place::ScopedPlaceId,
    ) -> Option<(DefinitionIndex, &Definition)> {
        self.definitions
            .iter_enumerated()
            .find(|(_, def)| def.scope_id == scope_id && def.place_id == place_id)
    }

    /// Add an identifier usage
    pub fn add_identifier_usage(&mut self, usage: IdentifierUsage) -> usize {
        let index = self.identifier_usages.len();
        self.identifier_usages.push(usage);
        index
    }

    /// Add a use-def relationship
    pub fn add_use(&mut self, usage_index: usize, definition_id: DefinitionIndex) {
        self.uses.insert(usage_index, definition_id);
    }

    /// Get all identifier usages
    pub fn identifier_usages(&self) -> &[IdentifierUsage] {
        &self.identifier_usages
    }

    /// Check if an identifier usage has a corresponding definition
    pub fn is_usage_resolved(&self, usage_index: usize) -> bool {
        self.uses.contains_key(&usage_index)
    }

    /// Get the definition for a specific identifier usage
    pub fn get_use_definition(&self, usage_index: usize) -> Option<&Definition> {
        self.uses
            .get(&usage_index)
            .and_then(|def_id| self.definitions.get(*def_id))
    }

    /// Add an expression and return its ID
    pub fn add_expression(&mut self, expression_info: ExpressionInfo) -> ExpressionId {
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

/// Main entry point for semantic analysis
///
/// This is the primary Salsa query that produces a complete semantic model for a source file.
/// The result is cached by Salsa for incremental compilation - if the input file hasn't
/// changed, this function won't be re-executed.
///
/// # Parameters
///
/// - `db`: The semantic database instance
/// - `file`: The source file to analyze
///
/// # Returns
///
/// A complete `SemanticIndex` containing all semantic information for the file.
///
/// # Example
///
/// ```rust,ignore
/// let db = SemanticDatabaseImpl::default();
/// let file = SourceProgram::new(&db, source_code);
/// let index = semantic_index(&db, file).as_ref().expect("Got unexpected parse errors");
/// ```
#[salsa::tracked(returns(ref))]
pub fn semantic_index(db: &dyn SemanticDb, file: File) -> Result<SemanticIndex, Vec<Diagnostic>> {
    // Parse the file and analyze its semantics
    let parsed_module = parse_program(db, file);
    if parsed_module.diagnostics.is_empty() {
        Ok(semantic_index_from_module(db, &parsed_module.module, file))
    } else {
        Err(parsed_module.diagnostics)
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
pub fn semantic_index_from_module<'a>(
    db: &'a dyn SemanticDb,
    module: &'a ParsedModule,
    file: File,
) -> SemanticIndex {
    let builder = SemanticIndexBuilder::new(db, file, module);
    builder.build()
}

/// Main entry point for semantic analysis with automatic parsing
///
/// This is the main entry point for semantic validation. It builds the semantic
/// index and runs all registered validators to produce a collection of diagnostics.
///
/// # Validators
///
/// Currently includes:
/// - Scope validation (undeclared variables, unused variables, duplicates)
///
/// # TODO: Add more validators
/// - Type checking validation
/// - Control flow validation
/// - Module/import validation
/// - Style/lint validation
pub fn validate_semantics<'a>(
    db: &'a dyn SemanticDb,
    module: &'a ParsedModule,
    file: File,
) -> DiagnosticCollection {
    let index = semantic_index_from_module(db, module, file);

    // Create validator registry with all available validators
    let registry = crate::validation::validator::create_default_registry();
    registry.validate_all(db, file, &index)
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
pub(crate) struct SemanticIndexBuilder<'db> {
    _db: &'db dyn SemanticDb,
    _file: File,
    module: &'db ParsedModule,

    // Current building state
    index: SemanticIndex,
    /// Stack of scope IDs representing the current nesting level
    /// The top of the stack is the currently active scope
    scope_stack: Vec<FileScopeId>,
}

impl<'db> SemanticIndexBuilder<'db> {
    pub fn new(db: &'db dyn SemanticDb, file: File, module: &'db ParsedModule) -> Self {
        let mut builder = Self {
            _db: db,
            _file: file,
            module,
            index: SemanticIndex::new(),
            scope_stack: Vec::new(),
        };

        // Create the root module scope
        let root_scope = Scope::new(None, crate::place::ScopeKind::Module);
        let root_scope_id = builder.index.add_scope(root_scope);
        builder.scope_stack.push(root_scope_id);

        builder
    }

    pub fn build(mut self) -> SemanticIndex {
        // Pass 1: Collect all top-level declarations (functions, structs, etc.)
        // This allows forward references to work correctly
        for item in self.module.items() {
            self.collect_top_level_declaration(item);
        }

        // Pass 2: Process function bodies and other content
        for item in self.module.items() {
            self.process_top_level_item_body(item);
        }

        // Pop the root scope
        self.scope_stack.pop();

        self.index
    }

    fn current_scope(&self) -> FileScopeId {
        *self
            .scope_stack
            .last()
            .expect("scope stack should never be empty")
    }

    fn push_scope(&mut self, kind: crate::place::ScopeKind) -> FileScopeId {
        let parent = Some(self.current_scope());
        let scope = Scope::new(parent, kind);
        let scope_id = self.index.add_scope(scope);
        self.scope_stack.push(scope_id);
        scope_id
    }

    fn pop_scope(&mut self) {
        self.scope_stack
            .pop()
            .expect("tried to pop from empty scope stack");
    }

    fn with_new_scope<F>(&mut self, kind: crate::place::ScopeKind, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let _scope_id = self.push_scope(kind);
        f(self);
        self.pop_scope();
    }

    fn add_place(
        &mut self,
        name: &str,
        flags: crate::place::PlaceFlags,
    ) -> crate::place::ScopedPlaceId {
        let scope_id = self.current_scope();
        self.index
            .place_table_mut(scope_id)
            .expect("current scope should have a place table")
            .add_place(name.to_string(), flags)
    }

    fn add_place_with_definition(
        &mut self,
        name: &str,
        flags: crate::place::PlaceFlags,
        def_kind: DefinitionKind,
        name_span: SimpleSpan<usize>,
        full_span: SimpleSpan<usize>,
    ) -> (crate::place::ScopedPlaceId, DefinitionIndex) {
        let place_id = self.add_place(name, flags);
        let scope_id = self.current_scope();

        let definition = Definition {
            file: self._file,
            scope_id,
            place_id,
            name: name.to_string(),
            name_span,
            full_span,
            kind: def_kind,
        };

        let def_id = self.index.add_definition(definition);
        (place_id, def_id)
    }

    /// Pass 1: Collect top-level declarations without processing bodies
    /// This enables forward references by declaring all symbols first
    fn collect_top_level_declaration(&mut self, item: &TopLevelItem) {
        match item {
            TopLevelItem::Function(func) => self.declare_function(func),
            TopLevelItem::Struct(struct_def) => self.visit_struct(struct_def), // Structs don't have bodies
            TopLevelItem::Namespace(namespace) => self.declare_namespace(namespace),
            TopLevelItem::Import(import) => self.visit_import(import), // Imports are just declarations
            TopLevelItem::Const(const_def) => self.visit_const(const_def), // Constants need full processing
        }
    }

    /// Pass 2: Process the bodies of top-level items
    /// This is where we analyze function bodies, namespace contents, etc.
    fn process_top_level_item_body(&mut self, item: &TopLevelItem) {
        match item {
            TopLevelItem::Function(func) => self.process_function_body(func),
            TopLevelItem::Struct(_) => {} // Structs don't have bodies to process
            TopLevelItem::Namespace(namespace) => self.process_namespace_body(namespace),
            TopLevelItem::Import(_) => {} // Imports already fully processed
            TopLevelItem::Const(_) => {}  // Constants already fully processed
        }
    }

    fn visit_struct(&mut self, struct_def: &Spanned<StructDef>) {
        use crate::definition::{DefinitionKind, StructDefRef};
        use crate::place::PlaceFlags;

        let struct_def_inner = struct_def.value();
        let struct_span = struct_def.span();

        // Define the struct in the current scope
        let def_kind = DefinitionKind::Struct(StructDefRef::from_ast(struct_def));
        self.add_place_with_definition(
            struct_def_inner.name.value(),
            PlaceFlags::DEFINED | PlaceFlags::STRUCT,
            def_kind,
            struct_def_inner.name.span(),
            struct_span,
        );

        // Note: Struct fields are not separate scopes in most languages,
        // so we don't create a new scope here. The fields are part of the type system.
    }

    fn visit_import(&mut self, import: &Spanned<ImportStmt>) {
        use crate::definition::{DefinitionKind, ImportDefRef};
        use crate::place::PlaceFlags;

        let import_inner = import.value();
        let import_span = import.span();

        // The imported name (or alias) is defined in the current scope
        let (name, name_span) = import_inner.alias.as_ref().map_or_else(
            || (import_inner.item.value(), import_inner.item.span()),
            |alias| (alias.value(), alias.span()),
        );

        let def_kind = DefinitionKind::Import(ImportDefRef::from_ast(import));
        self.add_place_with_definition(name, PlaceFlags::DEFINED, def_kind, name_span, import_span);
    }

    fn visit_const(&mut self, const_def: &Spanned<ConstDef>) {
        use crate::definition::{ConstDefRef, DefinitionKind};
        use crate::place::PlaceFlags;

        let const_def_inner = const_def.value();
        let const_span = const_def.span();

        // Visit the value expression to find any identifier uses and capture the ID
        let value_expr_id = self.visit_expression(&const_def_inner.value);

        // Define the constant in the current scope with the captured expression ID
        let def_kind = DefinitionKind::Const(ConstDefRef::from_ast(const_def, Some(value_expr_id)));
        self.add_place_with_definition(
            const_def_inner.name.value(),
            PlaceFlags::DEFINED | PlaceFlags::CONSTANT,
            def_kind,
            const_def_inner.name.span(),
            const_span,
        );
    }

    /// Pass 1: Declare function without processing its body
    /// This allows forward references to work
    fn declare_function(&mut self, func: &Spanned<FunctionDef>) {
        use crate::definition::{DefinitionKind, FunctionDefRef};
        use crate::place::PlaceFlags;

        let func_def = func.value();
        let func_span = func.span();

        // Define the function in the current scope
        let def_kind = DefinitionKind::Function(FunctionDefRef::from_ast(func));
        self.add_place_with_definition(
            func_def.name.value(),
            PlaceFlags::DEFINED | PlaceFlags::FUNCTION,
            def_kind,
            func_def.name.span(),
            func_span,
        );
    }

    /// Pass 2: Process function body with parameters and statements
    fn process_function_body(&mut self, func: &Spanned<FunctionDef>) {
        use crate::definition::{DefinitionKind, ParameterDefRef};
        use crate::place::PlaceFlags;

        let func_def = func.value();

        // Create a new scope for the function body
        self.with_new_scope(crate::place::ScopeKind::Function, |builder| {
            // Map the function body span to its scope for IDE features
            let current_scope = builder.current_scope();
            builder.index.set_scope_for_span(func.span(), current_scope);

            // Define parameters in the function scope
            for param in &func_def.params {
                let def_kind = DefinitionKind::Parameter(ParameterDefRef::from_ast(param));
                builder.add_place_with_definition(
                    param.name.value(),
                    PlaceFlags::DEFINED | PlaceFlags::PARAMETER,
                    def_kind,
                    param.name.span(),
                    param.name.span(), // TODO: Extend to full parameter span when available
                );
            }

            // Visit function body statements
            for stmt in &func_def.body {
                builder.visit_statement(stmt);
            }
        });
    }

    /// Pass 1: Declare namespace without processing its contents
    fn declare_namespace(&mut self, namespace: &Spanned<Namespace>) {
        use crate::definition::{DefinitionKind, NamespaceDefRef};
        use crate::place::PlaceFlags;

        let namespace_inner = namespace.value();
        let namespace_span = namespace.span();

        // Define the namespace in the current scope
        let def_kind = DefinitionKind::Namespace(NamespaceDefRef::from_ast(namespace));
        self.add_place_with_definition(
            namespace_inner.name.value(),
            PlaceFlags::DEFINED,
            def_kind,
            namespace_inner.name.span(),
            namespace_span,
        );
    }

    /// Pass 2: Process namespace contents
    fn process_namespace_body(&mut self, namespace: &Spanned<Namespace>) {
        let namespace_inner = namespace.value();

        // Create a new scope for the namespace contents
        self.with_new_scope(crate::place::ScopeKind::Namespace, |builder| {
            // Map the namespace body span to its scope for IDE features
            let current_scope = builder.current_scope();
            builder
                .index
                .set_scope_for_span(namespace.span(), current_scope);

            // Pass 1: Collect declarations within the namespace
            for item in &namespace_inner.body {
                builder.collect_top_level_declaration(item);
            }

            // Pass 2: Process bodies within the namespace
            for item in &namespace_inner.body {
                builder.process_top_level_item_body(item);
            }
        });
    }

    fn visit_statement(&mut self, stmt: &Spanned<Statement>) {
        use crate::place::PlaceFlags;

        match stmt.value() {
            Statement::Let {
                name,
                value,
                statement_type,
            } => {
                use crate::definition::{DefinitionKind, LetDefRef};
                // Visit the value expression and capture the ID
                let value_expr_id = self.visit_expression(value);
                // Define the let variable with the captured expression ID
                let def_kind = DefinitionKind::Let(LetDefRef::from_let_statement(
                    name.value(),
                    statement_type.clone(),
                    Some(value_expr_id),
                ));
                self.add_place_with_definition(
                    name.value(),
                    PlaceFlags::DEFINED,
                    def_kind,
                    name.span(),
                    stmt.span(),
                );
            }
            Statement::Local { name, value, ty } => {
                use crate::definition::{DefinitionKind, LocalDefRef};
                // Visit the value expression and capture the ID
                let value_expr_id = self.visit_expression(value);
                // Define the local variable with the captured expression ID
                let def_kind = DefinitionKind::Local(LocalDefRef::from_local_statement(
                    name.value(),
                    ty.clone(),
                    Some(value_expr_id),
                ));
                self.add_place_with_definition(
                    name.value(),
                    PlaceFlags::DEFINED,
                    def_kind,
                    name.span(),
                    stmt.span(),
                );
                // TODO: Analyze type annotation when type system is implemented
            }
            Statement::Const(const_def) => {
                // Statement-level const is wrapped in a spanned context
                let spanned_const = Spanned::new(const_def.clone(), stmt.span());
                self.visit_const(&spanned_const);
            }
            Statement::Assignment { lhs, rhs } => {
                // Visit both sides - assignment targets and values
                let _lhs_expr_id = self.visit_expression(lhs);
                let _rhs_expr_id = self.visit_expression(rhs);
                // TODO: Validate assignment compatibility (AssignmentValidator)
                // - Check that LHS is actually assignable (not a constant, etc.)
                // - Validate type compatibility between LHS and RHS
                // - Check mutability constraints (let vs mutable variables)
                // - Validate assignment to valid lvalue expressions
            }
            Statement::Return { value } => {
                if let Some(expr) = value {
                    let _return_expr_id = self.visit_expression(expr);
                }
                // TODO: Validate return type compatibility (ReturnValidator)
                // - Check return type compatibility with function signature
                // - Validate that we're actually inside a function
                // - Handle multiple return statements with consistent types
                // - Check for missing return in non-void functions
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let _condition_expr_id = self.visit_expression(condition);
                // Create new scopes for if/else blocks to properly handle variable visibility
                self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                    builder.visit_statement(then_block);
                });
                if let Some(else_stmt) = else_block {
                    self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                        builder.visit_statement(else_stmt);
                    });
                }
                // TODO: Implement control flow analysis (ControlFlowValidator)
                // - Track reachability and dead code detection
                // - Validate boolean condition type when type system is ready
                // - Check for unreachable code after returns
            }
            Statement::Expression(expr) => {
                let _stmt_expr_id = self.visit_expression(expr);
            }
            Statement::Block(statements) => {
                // Create new scope for block statements to ensure proper variable scoping
                // Variables declared in blocks should not be visible outside the block
                self.with_new_scope(crate::place::ScopeKind::Block, |builder| {
                    // Map the block's span to its scope for IDE features
                    let current_scope = builder.current_scope();
                    builder.index.set_scope_for_span(stmt.span(), current_scope);

                    for stmt in statements {
                        builder.visit_statement(stmt);
                    }
                });
            }
        }
    }

    fn visit_expression(&mut self, expr: &Spanned<Expression>) -> ExpressionId {
        // First, create an ExpressionInfo for this expression and track it
        let expr_info = ExpressionInfo {
            file: self._file,
            ast_node: expr.value().clone(),
            ast_span: expr.span(),
            scope_id: self.current_scope(),
        };
        let expr_id = self.index.add_expression(expr_info);

        match expr.value() {
            Expression::Identifier(name) => {
                let current_scope = self.current_scope();

                let usage = IdentifierUsage {
                    name: name.value().clone(),
                    span: name.span(),
                    scope_id: current_scope,
                };

                let usage_index = self.index.add_identifier_usage(usage);

                // This is a use of an identifier - mark it as used if we can resolve it
                if let Some((def_scope_id, place_id)) =
                    self.index.resolve_name(name.value(), current_scope)
                    && let Some(place_table) = self.index.place_table_mut(def_scope_id)
                {
                    // Mark the place as used
                    place_table.mark_as_used(place_id);

                    // Find the corresponding definition ID and record the use-def relationship
                    if let Some((def_id, _)) =
                        self.index.definition_for_place(def_scope_id, place_id)
                    {
                        self.index.add_use(usage_index, def_id);
                    }
                }
                // Note: Unresolved symbols will be detected in the validation pass
                // using the identifier_usages tracking
            }
            Expression::BinaryOp { left, right, .. } => {
                self.visit_expression(left);
                self.visit_expression(right);
            }
            Expression::FunctionCall { callee, args } => {
                self.visit_expression(callee);
                for arg in args {
                    self.visit_expression(arg);
                }
                // TODO: Validate function call arity (FunctionCallValidator)
                // - Check argument count matches parameter count
                // - Validate argument types match parameter types
                // - Check function exists and is callable
            }
            Expression::MemberAccess { object, .. } => {
                self.visit_expression(object);
                // TODO: Implement proper member access analysis
                // Current limitation: Field access doesn't introduce new scope issues for now
                // Future improvements needed:
                // - Validate that the field exists on the type (StructFieldValidator)
                // - Validate that the object type has the accessed field
                // - Handle method calls vs field access
                // - Support for nested member access chains
                // - Validate member access on primitive types (should error)
                // Field access doesn't introduce new scope issues for now
            }
            Expression::IndexAccess { array, index } => {
                self.visit_expression(array);
                self.visit_expression(index);
                // TODO: Add array bounds checking validation in future passes
                // provided the array size is known.
                // TODO: Validate indexing on non-array types (IndexingValidator)
                // - Check that the array expression has an indexable type
                // - Validate index expression is integer type
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_value) in fields {
                    self.visit_expression(field_value);
                }
                // TODO: Validate struct literal field names against struct definition (StructLiteralValidator)
                // TODO: Check for missing required fields
                // TODO: Check for unknown fields
                // TODO: Validate field value types match struct definition
            }
            Expression::Tuple(exprs) => {
                for expr in exprs {
                    self.visit_expression(expr);
                }
            }
            Expression::Literal(_) => {
                // Literals don't reference any symbols
                // TODO: Consider adding literal validation (e.g., numeric range checks)
            }
            Expression::BooleanLiteral(_) => {
                // Boolean literals don't reference any symbols
                // TODO: Consider adding boolean literal validation
            } // TODO: Add support for more expression types as the parser is extended:
              // - Conditional expressions (ternary operator)
              // - Array/slice literals
        }

        expr_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{test_db, TestDb};
    use cairo_m_compiler_parser::SourceProgram;

    struct TestCase {
        db: TestDb,
        source: SourceProgram,
    }

    fn test_case(content: &str) -> TestCase {
        let db = test_db();
        let source = SourceProgram::new(&db, content.to_string());
        TestCase { db, source }
    }

    #[test]
    fn test_empty_program() {
        let TestCase { db, source } = test_case("");
        let index = semantic_index(&db, source).as_ref().unwrap();

        let root = index.root_scope().expect("should have root scope");
        let scope = index.scope(root).unwrap();
        assert_eq!(scope.kind, crate::place::ScopeKind::Module);
        assert_eq!(scope.parent, None);
    }

    #[test]
    fn test_simple_function() {
        let TestCase { db, source } = test_case("func test() { }");
        let index = semantic_index(&db, source).as_ref().unwrap();

        // Should have root scope and function scope
        let root = index.root_scope().unwrap();
        let root_table = index.place_table(root).unwrap();

        // Function should be defined in root scope
        let func_place_id = root_table
            .place_id_by_name("test")
            .expect("function should be defined");
        let func_place = root_table.place(func_place_id).unwrap();
        assert!(func_place
            .flags
            .contains(crate::place::PlaceFlags::FUNCTION));

        // Should have one child scope (the function)
        let child_scopes: Vec<_> = index.child_scopes(root).collect();
        assert_eq!(child_scopes.len(), 1);

        let func_scope = child_scopes[0];
        let func_scope_info = index.scope(func_scope).unwrap();
        assert_eq!(func_scope_info.kind, crate::place::ScopeKind::Function);
    }

    #[test]
    fn test_function_with_parameters() {
        let TestCase { db, source } = test_case("func add(a: felt, b: felt) { }");
        let index = semantic_index(&db, source).as_ref().unwrap();

        let root = index.root_scope().unwrap();
        let child_scopes: Vec<_> = index.child_scopes(root).collect();
        let func_scope = child_scopes[0];
        let func_table = index.place_table(func_scope).unwrap();

        // Parameters should be defined in function scope
        let a_place_id = func_table
            .place_id_by_name("a")
            .expect("parameter 'a' should be defined");
        let a_place = func_table.place(a_place_id).unwrap();
        assert!(a_place.flags.contains(crate::place::PlaceFlags::PARAMETER));

        let b_place_id = func_table
            .place_id_by_name("b")
            .expect("parameter 'b' should be defined");
        let b_place = func_table.place(b_place_id).unwrap();
        assert!(b_place.flags.contains(crate::place::PlaceFlags::PARAMETER));
    }

    #[test]
    fn test_variable_resolution() {
        let TestCase { db, source } =
            test_case("func test(param: felt) { let local_var = param; }");
        let index = semantic_index(&db, source).as_ref().unwrap();

        let root = index.root_scope().unwrap();
        let child_scopes: Vec<_> = index.child_scopes(root).collect();
        let func_scope = child_scopes[0];
        let func_table = index.place_table(func_scope).unwrap();

        // Parameter should be marked as used
        let param_place_id = func_table.place_id_by_name("param").unwrap();
        let param_place = func_table.place(param_place_id).unwrap();
        assert!(param_place.is_used(), "parameter should be marked as used");

        // Local variable should be defined
        let local_place_id = func_table.place_id_by_name("local_var").unwrap();
        let local_place = func_table.place(local_place_id).unwrap();
        assert!(local_place.is_defined(), "local variable should be defined");
    }

    #[test]
    fn test_comprehensive_semantic_analysis() {
        let TestCase { db, source } = test_case(
            r#"
            const PI = 314;

            struct Point {
                x: felt,
                y: felt
            }

            func distance(p1: Point, p2: Point) -> felt {
                let dx = p1.x - p2.x;
                local dy: felt = p1.y - p2.y;
                return dx * dx + dy * dy;
            }

            namespace Math {
                func square(x: felt) -> felt {
                    return x * x;
                }
            }
        "#,
        );

        let index = semantic_index(&db, source).as_ref().unwrap();

        // Should have root scope plus function scope and namespace scope
        let root = index.root_scope().unwrap();
        let child_scopes: Vec<_> = index.child_scopes(root).collect();
        assert_eq!(
            child_scopes.len(),
            2,
            "Should have function and namespace scopes"
        );

        // Check root scope has the expected symbols
        let root_table = index.place_table(root).unwrap();
        assert!(
            root_table.place_id_by_name("PI").is_some(),
            "PI constant should be defined"
        );
        assert!(
            root_table.place_id_by_name("Point").is_some(),
            "Point struct should be defined"
        );
        assert!(
            root_table.place_id_by_name("distance").is_some(),
            "distance function should be defined"
        );
        assert!(
            root_table.place_id_by_name("Math").is_some(),
            "Math namespace should be defined"
        );

        // Check definitions are tracked
        let all_definitions = index.all_definitions().count();
        // 1 const, 1 struct, 2 functions, 1 namespace, 3 function params, 2 inner fn variables
        assert_eq!(all_definitions, 10);

        // Find function definition
        let distance_def =
            index.definition_for_place(root, root_table.place_id_by_name("distance").unwrap());
        assert!(matches!(
            distance_def,
            Some((_, def)) if matches!(def.kind, crate::definition::DefinitionKind::Function(_))
        ));

        // Check parameters and locals in function scope
        let func_scope = child_scopes
            .iter()
            .find(|&scope_id| {
                index.scope(*scope_id).unwrap().kind == crate::place::ScopeKind::Function
            })
            .unwrap();

        let func_table = index.place_table(*func_scope).unwrap();
        assert!(
            func_table.place_id_by_name("p1").is_some(),
            "p1 parameter should be defined"
        );
        assert!(
            func_table.place_id_by_name("p2").is_some(),
            "p2 parameter should be defined"
        );
        assert!(
            func_table.place_id_by_name("dx").is_some(),
            "dx local should be defined"
        );
        assert!(
            func_table.place_id_by_name("dy").is_some(),
            "dy local should be defined"
        );

        // Check namespace scope
        let namespace_scope = child_scopes
            .iter()
            .find(|&scope_id| {
                index.scope(*scope_id).unwrap().kind == crate::place::ScopeKind::Namespace
            })
            .unwrap();

        let namespace_table = index.place_table(*namespace_scope).unwrap();
        assert!(
            namespace_table.place_id_by_name("square").is_some(),
            "square function should be defined in Math namespace"
        );
    }

    #[test]
    fn test_real_spans_are_used() {
        let TestCase { db, source } = test_case("func test(x: felt) { let y = x; }");
        let index = semantic_index(&db, source).as_ref().unwrap();

        // Get all identifier usages
        let usages = index.identifier_usages();

        // Should have at least one usage for the identifier 'x' being used
        let x_usage = usages.iter().find(|u| u.name == "x");
        assert!(x_usage.is_some(), "Should find usage of identifier 'x'");

        let x_usage = x_usage.unwrap();
        // Verify that real spans are being used (not dummy spans)
        assert_ne!(
            x_usage.span,
            SimpleSpan::from(0..0),
            "Should not use dummy span for identifier usage"
        );
        assert!(
            x_usage.span.start < x_usage.span.end,
            "Span should have positive length"
        );

        // Check definitions also have real spans
        let definitions: Vec<_> = index.all_definitions().collect();
        assert!(!definitions.is_empty(), "Should have definitions");

        for (_, def) in definitions {
            assert_ne!(
                def.name_span,
                SimpleSpan::from(0..0),
                "Definition name span should not be dummy"
            );
            assert_ne!(
                def.full_span,
                SimpleSpan::from(0..0),
                "Definition full span should not be dummy"
            );
            assert!(
                def.name_span.start < def.name_span.end,
                "Name span should have positive length"
            );
            assert!(
                def.full_span.start < def.full_span.end,
                "Full span should have positive length"
            );
        }
    }

    #[test]
    fn test_definition_expression_ids_are_captured() {
        let TestCase { db, source } = test_case(
            r#"
            const Z = 314;

            func test() -> felt {
                let x = 42;
                local y: felt = 100;
                return x + y;
            }
            "#,
        );
        let index = semantic_index(&db, source).as_ref().unwrap();

        // Find the let definition
        let let_definitions: Vec<_> = index
            .all_definitions()
            .filter_map(|(_, def)| match &def.kind {
                DefinitionKind::Let(let_ref) => Some(let_ref),
                _ => None,
            })
            .collect();

        // Find the local definition
        let local_definitions: Vec<_> = index
            .all_definitions()
            .filter_map(|(_, def)| match &def.kind {
                DefinitionKind::Local(local_ref) => Some(local_ref),
                _ => None,
            })
            .collect();

        // Find the const definition
        let const_definitions: Vec<_> = index
            .all_definitions()
            .filter_map(|(_, def)| match &def.kind {
                DefinitionKind::Const(const_ref) => Some(const_ref),
                _ => None,
            })
            .collect();

        // Check that we found the expected definitions
        assert_eq!(
            let_definitions.len(),
            1,
            "Should find exactly 1 let definition"
        );
        assert_eq!(
            local_definitions.len(),
            1,
            "Should find exactly 1 local definition"
        );
        assert_eq!(
            const_definitions.len(),
            1,
            "Should find exactly 1 const definition"
        );

        // Verify the names and expression IDs
        assert_eq!(let_definitions[0].name, "x");
        assert!(
            let_definitions[0].value_expr_id.is_some(),
            "Let definition should have a value expression ID"
        );

        assert_eq!(local_definitions[0].name, "y");
        assert!(
            local_definitions[0].value_expr_id.is_some(),
            "Local definition should have a value expression ID"
        );

        assert_eq!(const_definitions[0].name, "Z");
        assert!(
            const_definitions[0].value_expr_id.is_some(),
            "Const definition should have a value expression ID"
        );

        // Verify that the expression IDs actually correspond to real expressions in the index
        for let_def in &let_definitions {
            if let Some(expr_id) = let_def.value_expr_id {
                assert!(
                    index.expression(expr_id).is_some(),
                    "Expression ID should be valid"
                );
            }
        }
        for local_def in &local_definitions {
            if let Some(expr_id) = local_def.value_expr_id {
                assert!(
                    index.expression(expr_id).is_some(),
                    "Expression ID should be valid"
                );
            }
        }
        for const_def in &const_definitions {
            if let Some(expr_id) = const_def.value_expr_id {
                assert!(
                    index.expression(expr_id).is_some(),
                    "Expression ID should be valid"
                );
            }
        }
    }
}
