//! # Semantic Index
//!
//! This module defines the main semantic analysis data structure and query.
//! The SemanticIndex contains all semantic information for a source file,
//! including scopes, places, and their relationships.

use crate::definition::DefinitionKind;
use crate::place::{FileScopeId, PlaceTable, Scope};
use crate::{File, SemanticDb};
use cairo_m_compiler_parser::{parse_program, ParsedModule};
use rustc_hash::FxHashMap;

/// The main semantic analysis result for a source file
///
/// This contains all semantic information derived from the AST,
/// including scopes, symbol tables, and relationships between them.
#[derive(Debug, PartialEq, Eq)]
pub struct SemanticIndex {
    /// A place table for each scope in the file
    place_tables: Vec<PlaceTable>,

    /// The scope hierarchy
    scopes: Vec<Scope>,

    /// Maps AST node positions to their containing scope
    /// For now, we'll use simple span-based mapping
    scopes_by_span: FxHashMap<(usize, usize), FileScopeId>,

    /// All definitions in the file
    /// For now, we'll store them in a simple Vec and provide access methods
    definitions: Vec<(FileScopeId, DefinitionKind, crate::place::ScopedPlaceId)>,

    /// Use-def tracking: maps identifier names to their resolved definitions
    /// Key: (identifier_name, scope_where_used)
    /// Value: (definition_scope, definition_place)
    /// TODO: Replace with span-based tracking when parser provides spans
    uses: FxHashMap<(String, FileScopeId), (FileScopeId, crate::place::ScopedPlaceId)>,

    /// Track all identifier usages with their containing scopes
    /// This will help implement undeclared variable detection
    identifier_usages: Vec<(String, FileScopeId)>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self {
            place_tables: Vec::new(),
            scopes: Vec::new(),
            scopes_by_span: FxHashMap::default(),
            definitions: Vec::new(),
            uses: FxHashMap::default(),
            identifier_usages: Vec::new(),
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
    pub fn set_scope_for_span(&mut self, span: (usize, usize), scope_id: FileScopeId) {
        self.scopes_by_span.insert(span, scope_id);
    }

    /// Get the scope containing a given source span
    pub fn scope_for_span(&self, span: (usize, usize)) -> Option<FileScopeId> {
        self.scopes_by_span.get(&span).copied()
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

    /// Add a definition to the index
    pub fn add_definition(
        &mut self,
        scope_id: FileScopeId,
        place_id: crate::place::ScopedPlaceId,
        kind: DefinitionKind,
    ) {
        self.definitions.push((scope_id, kind, place_id));
    }

    /// Get all definitions in a specific scope
    pub fn definitions_in_scope(
        &self,
        scope_id: FileScopeId,
    ) -> impl Iterator<Item = &(FileScopeId, DefinitionKind, crate::place::ScopedPlaceId)> {
        self.definitions
            .iter()
            .filter(move |(s, _, _)| *s == scope_id)
    }

    /// Get all definitions in the file
    pub fn all_definitions(&self) -> &[(FileScopeId, DefinitionKind, crate::place::ScopedPlaceId)] {
        &self.definitions
    }

    /// Find definition by place
    pub fn definition_for_place(
        &self,
        scope_id: FileScopeId,
        place_id: crate::place::ScopedPlaceId,
    ) -> Option<&DefinitionKind> {
        self.definitions
            .iter()
            .find(|(s, _, p)| *s == scope_id && *p == place_id)
            .map(|(_, kind, _)| kind)
    }

    /// Add a use-def relationship
    pub fn add_use(
        &mut self,
        identifier: String,
        use_scope: FileScopeId,
        def_scope: FileScopeId,
        def_place: crate::place::ScopedPlaceId,
    ) {
        self.uses
            .insert((identifier, use_scope), (def_scope, def_place));
    }

    /// Add an identifier usage (for undeclared variable detection)
    pub fn add_identifier_usage(&mut self, identifier: String, scope: FileScopeId) {
        self.identifier_usages.push((identifier, scope));
    }

    /// Get all identifier usages
    pub fn identifier_usages(&self) -> &[(String, FileScopeId)] {
        &self.identifier_usages
    }

    /// Check if an identifier usage has a corresponding definition
    pub fn is_identifier_resolved(&self, identifier: &str, scope: FileScopeId) -> bool {
        self.uses.contains_key(&(identifier.to_string(), scope))
    }

    /// Get the definition for a specific identifier usage
    pub fn get_use_definition(
        &self,
        identifier: &str,
        scope: FileScopeId,
    ) -> Option<(FileScopeId, crate::place::ScopedPlaceId)> {
        self.uses.get(&(identifier.to_string(), scope)).copied()
    }
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Main semantic analysis query
///
/// This is the primary Salsa-tracked query that builds the complete semantic
/// index for a source file. It takes a parsed module and produces the semantic
/// analysis result.
#[salsa::tracked(returns(ref))]
pub fn semantic_index(db: &dyn SemanticDb, file: File) -> SemanticIndex {
    // Get the parsed module from the parser
    let module = parse_program(db, file);

    // Create the semantic index builder and build the index
    let builder = SemanticIndexBuilder::new(db, file, module);
    builder.build()
}

#[salsa::tracked(returns(ref))]
pub fn semantic_index_from_module<'a>(
    db: &'a dyn SemanticDb,
    module: &'a ParsedModule,
    file: File,
) -> SemanticIndex {
    let builder = SemanticIndexBuilder::new(db, file, module);
    builder.build()
}

/// Validate semantic analysis and return diagnostics
///
/// This query runs all semantic validators on the semantic index
/// and returns any issues found.
#[salsa::tracked(returns(ref))]
pub fn validate_semantics<'a>(
    db: &'a dyn SemanticDb,
    module: &'a ParsedModule,
    file: File,
) -> crate::validation::DiagnosticCollection {
    let index = semantic_index_from_module(db, module, file);
    let registry = crate::validation::validator::create_default_registry();
    registry.validate_all(db, file, index)
}

/// Builder for constructing the semantic index
///
/// This follows the visitor pattern to walk the AST and build up
/// the semantic information incrementally.
pub(crate) struct SemanticIndexBuilder<'db> {
    _db: &'db dyn SemanticDb,
    _file: File,
    module: &'db ParsedModule,

    // Current building state
    index: SemanticIndex,
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
        // Visit all top-level items in the module
        for item in self.module.items() {
            self.visit_top_level_item(item);
        }

        // Pop the root scope
        self.scope_stack.pop();

        self.index
    }

    fn current_scope_id(&self) -> FileScopeId {
        *self
            .scope_stack
            .last()
            .expect("scope stack should never be empty")
    }

    fn push_scope(&mut self, kind: crate::place::ScopeKind) -> FileScopeId {
        let parent = Some(self.current_scope_id());
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
        let scope_id = self.current_scope_id();
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
    ) -> crate::place::ScopedPlaceId {
        let place_id = self.add_place(name, flags);
        let scope_id = self.current_scope_id();
        self.index.add_definition(scope_id, place_id, def_kind);
        place_id
    }

    fn visit_top_level_item(&mut self, item: &cairo_m_compiler_parser::parser::TopLevelItem) {
        use cairo_m_compiler_parser::parser::TopLevelItem;

        match item {
            TopLevelItem::Function(func) => self.visit_function(func),
            TopLevelItem::Struct(struct_def) => self.visit_struct(struct_def),
            TopLevelItem::Namespace(namespace) => self.visit_namespace(namespace),
            TopLevelItem::Import(import) => self.visit_import(import),
            TopLevelItem::Const(const_def) => self.visit_const(const_def),
        }
    }

    fn visit_function(&mut self, func: &cairo_m_compiler_parser::parser::FunctionDef) {
        use crate::definition::{DefinitionKind, FunctionDefRef};
        use crate::place::PlaceFlags;

        // Define the function in the current scope
        let def_kind = DefinitionKind::Function(FunctionDefRef::from_ast(func));
        self.add_place_with_definition(
            &func.name,
            PlaceFlags::DEFINED | PlaceFlags::FUNCTION,
            def_kind,
        );

        // Create a new scope for the function body
        self.with_new_scope(crate::place::ScopeKind::Function, |builder| {
            // Define parameters in the function scope
            for param in &func.params {
                use crate::definition::{DefinitionKind, ParameterDefRef};
                let def_kind = DefinitionKind::Parameter(ParameterDefRef::from_ast(param));
                builder.add_place_with_definition(
                    &param.name,
                    PlaceFlags::DEFINED | PlaceFlags::PARAMETER,
                    def_kind,
                );
            }

            // Visit function body statements
            for stmt in &func.body {
                builder.visit_statement(stmt);
            }
        });
    }

    fn visit_struct(&mut self, struct_def: &cairo_m_compiler_parser::parser::StructDef) {
        use crate::definition::{DefinitionKind, StructDefRef};
        use crate::place::PlaceFlags;

        // Define the struct in the current scope
        let def_kind = DefinitionKind::Struct(StructDefRef::from_ast(struct_def));
        self.add_place_with_definition(
            &struct_def.name,
            PlaceFlags::DEFINED | PlaceFlags::STRUCT,
            def_kind,
        );

        // Note: Struct fields are not separate scopes in most languages,
        // so we don't create a new scope here. The fields are part of the type system.
    }

    fn visit_namespace(&mut self, namespace: &cairo_m_compiler_parser::parser::Namespace) {
        use crate::definition::{DefinitionKind, NamespaceDefRef};
        use crate::place::PlaceFlags;

        // Define the namespace in the current scope
        let def_kind = DefinitionKind::Namespace(NamespaceDefRef::from_ast(namespace));
        self.add_place_with_definition(&namespace.name, PlaceFlags::DEFINED, def_kind);

        // Create a new scope for the namespace contents
        self.with_new_scope(crate::place::ScopeKind::Namespace, |builder| {
            for item in &namespace.body {
                builder.visit_top_level_item(item);
            }
        });
    }

    fn visit_import(&mut self, import: &cairo_m_compiler_parser::parser::ImportStmt) {
        use crate::definition::{DefinitionKind, ImportDefRef};
        use crate::place::PlaceFlags;

        // The imported name (or alias) is defined in the current scope
        let name = import.alias.as_ref().unwrap_or(&import.item);
        let def_kind = DefinitionKind::Import(ImportDefRef::from_ast(import));
        self.add_place_with_definition(name, PlaceFlags::DEFINED, def_kind);
    }

    fn visit_const(&mut self, const_def: &cairo_m_compiler_parser::parser::ConstDef) {
        use crate::definition::{ConstDefRef, DefinitionKind};
        use crate::place::PlaceFlags;

        // Define the constant in the current scope
        let def_kind = DefinitionKind::Const(ConstDefRef::from_ast(const_def));
        self.add_place_with_definition(
            &const_def.name,
            PlaceFlags::DEFINED | PlaceFlags::CONSTANT,
            def_kind,
        );

        // Visit the value expression to find any identifier uses
        self.visit_expression(&const_def.value);
    }

    fn visit_statement(&mut self, stmt: &cairo_m_compiler_parser::parser::Statement) {
        use crate::place::PlaceFlags;
        use cairo_m_compiler_parser::parser::Statement;

        match stmt {
            Statement::Let { name, value } => {
                use crate::definition::{DefinitionKind, LetDefRef};
                // Define the variable
                let def_kind = DefinitionKind::Let(LetDefRef::from_let_statement(name));
                self.add_place_with_definition(name, PlaceFlags::DEFINED, def_kind);
                // Visit the value expression
                self.visit_expression(value);
            }
            Statement::Local { name, value, ty } => {
                use crate::definition::{DefinitionKind, LocalDefRef};
                // Define the local variable
                let def_kind =
                    DefinitionKind::Local(LocalDefRef::from_local_statement(name, ty.is_some()));
                self.add_place_with_definition(name, PlaceFlags::DEFINED, def_kind);
                // Visit the value expression
                self.visit_expression(value);
            }
            Statement::Const(const_def) => {
                self.visit_const(const_def);
            }
            Statement::Assignment { lhs, rhs } => {
                // Visit both sides - assignment targets and values
                self.visit_expression(lhs);
                self.visit_expression(rhs);
            }
            Statement::Return { value } => {
                if let Some(expr) = value {
                    self.visit_expression(expr);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.visit_expression(condition);
                self.visit_statement(then_block);
                if let Some(else_stmt) = else_block {
                    self.visit_statement(else_stmt);
                }
            }
            Statement::Expression(expr) => {
                self.visit_expression(expr);
            }
            Statement::Block(statements) => {
                // For now, blocks don't create new scopes
                // This could be changed later for block-scoped variables
                for stmt in statements {
                    self.visit_statement(stmt);
                }
            }
        }
    }

    fn visit_expression(&mut self, expr: &cairo_m_compiler_parser::parser::Expression) {
        use cairo_m_compiler_parser::parser::Expression;

        match expr {
            Expression::Identifier(name) => {
                let current_scope = self.current_scope_id();

                // Record this identifier usage for undeclared variable detection
                self.index.add_identifier_usage(name.clone(), current_scope);

                // This is a use of an identifier - mark it as used if we can resolve it
                if let Some((def_scope_id, place_id)) = self.index.resolve_name(name, current_scope)
                    && let Some(place_table) = self.index.place_table_mut(def_scope_id)
                {
                    // Mark the place as used
                    place_table.mark_as_used(place_id);

                    // Record the use-def relationship
                    self.index
                        .add_use(name.clone(), current_scope, def_scope_id, place_id);
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
            }
            Expression::MemberAccess { object, .. } => {
                self.visit_expression(object);
                // Field access doesn't introduce new scope issues for now
            }
            Expression::IndexAccess { array, index } => {
                self.visit_expression(array);
                self.visit_expression(index);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_value) in fields {
                    self.visit_expression(field_value);
                }
            }
            Expression::Tuple(exprs) => {
                for expr in exprs {
                    self.visit_expression(expr);
                }
            }
            Expression::Literal(_) => {
                // Literals don't reference any symbols
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SemanticDatabaseImpl;
    use cairo_m_compiler_parser::SourceProgram;

    #[test]
    fn test_empty_program() {
        let db = SemanticDatabaseImpl::default();
        let source = SourceProgram::new(&db, "".to_string());
        let index = semantic_index(&db, source);

        // Should have a root module scope
        let root = index.root_scope().expect("should have root scope");
        let scope = index.scope(root).unwrap();
        assert_eq!(scope.kind, crate::place::ScopeKind::Module);
        assert_eq!(scope.parent, None);
    }

    #[test]
    fn test_simple_function() {
        let db = SemanticDatabaseImpl::default();
        let source = SourceProgram::new(&db, "func test() { }".to_string());
        let index = semantic_index(&db, source);

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
        let db = SemanticDatabaseImpl::default();
        let source = SourceProgram::new(&db, "func add(a: felt, b: felt) { }".to_string());
        let index = semantic_index(&db, source);

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
        let db = SemanticDatabaseImpl::default();
        let source = SourceProgram::new(
            &db,
            "func test(param: felt) { let local_var = param; }".to_string(),
        );
        let index = semantic_index(&db, source);

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
        let db = SemanticDatabaseImpl::default();
        let source = SourceProgram::new(
            &db,
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
        "#
            .to_string(),
        );

        let index = semantic_index(&db, source);

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
        let all_definitions = index.all_definitions();
        // 1 const, 1 struct, 2 functions, 1 namespace, 3 function params, 2 inner fn variables
        assert_eq!(all_definitions.len(), 10,);

        // Find function definition
        let distance_def =
            index.definition_for_place(root, root_table.place_id_by_name("distance").unwrap());
        assert!(matches!(
            distance_def,
            Some(crate::definition::DefinitionKind::Function(_))
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
}
