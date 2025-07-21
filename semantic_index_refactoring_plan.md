# Cairo-M Semantic Index Refactoring Plan

This document outlines a comprehensive refactoring plan for the Cairo-M semantic
index, drawing inspiration from ruff's proven architecture. The refactoring aims
to improve maintainability, performance, and extensibility while maintaining the
current functionality.

## Overview

The current Cairo-M semantic index implementation is functional but could
benefit from architectural improvements inspired by ruff's approach:

1. **Separation of Concerns**: Ruff clearly separates place management, scope
   tracking, and use-def analysis
2. **Builder Pattern**: Ruff uses a sophisticated builder pattern with clear
   state management
3. **Visitor Pattern**: Ruff implements a proper AST visitor with systematic
   traversal
4. **Type Safety**: Ruff uses newtype patterns and proper encapsulation for IDs
5. **Performance**: Ruff uses specialized data structures for efficient lookups

## Issue 1: Refactor Place Management System

### What

Refactor the place management system to adopt ruff's `PlaceExpr` and
`PlaceTable` architecture, which provides a more flexible and type-safe way to
handle symbols, attributes, and complex expressions.

### Why

The current implementation mixes simple symbols with place tracking in a way
that limits extensibility. Ruff's approach separates:

- `PlaceExpr`: Represents any assignable expression (names, attributes,
  subscripts)
- `PlaceTable`: Efficient storage and lookup of places within a scope
- `PlaceFlags`: Type-safe representation of place properties

This separation enables better support for complex assignment targets and more
efficient lookups.

### How

#### 1. Current Cairo-M Implementation

```rust
// Current simple implementation
pub struct Place {
    pub name: String,
    pub flags: PlaceFlags,
}
```

#### 2. Ruff's PlaceExpr Architecture

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/place.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceExprSubSegment {
    /// A member access, e.g. `.y` in `x.y`
    Member(String),
    /// An integer-based index access, e.g. `[1]` in `x[1]`
    IntSubscript(i64),
}

/// An expression that can be the target of a `Definition`.
#[derive(Eq, PartialEq, Debug)]
pub struct PlaceExpr {
    root_name: String,
    sub_segments: SmallVec<[PlaceExprSubSegment; 1]>,
}

impl PlaceExpr {
    pub const fn name(name: String) -> Self {
        Self {
            root_name: name,
            sub_segments: SmallVec::new_const(),
        }
    }
}
```

#### 3. Adapt for Cairo-M

```rust
// New place/expr.rs for Cairo-M
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceExprSubSegment {
    /// Struct field access, e.g. `.field` in `point.field`
    Member(String),
    /// Array index access, e.g. `[0]` in `arr[0]`
    IntSubscript(i64),
}

/// A place expression that can be assigned to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceExpr {
    root_name: String,
    sub_segments: SmallVec<[PlaceExprSubSegment; 1]>,
}

impl PlaceExpr {
    /// Create a simple name place
    pub const fn name(name: String) -> Self {
        Self {
            root_name: name,
            sub_segments: SmallVec::new_const(),
        }
    }

    /// Check if this is just a simple name
    pub fn is_name(&self) -> bool {
        self.sub_segments.is_empty()
    }

    /// Get the name if this is a simple name place
    pub fn as_name(&self) -> Option<&str> {
        if self.is_name() {
            Some(&self.root_name)
        } else {
            None
        }
    }
}

// Convert Cairo-M AST expressions to PlaceExpr
impl TryFrom<&Expression> for PlaceExpr {
    type Error = ();

    fn try_from(expr: &Expression) -> Result<Self, ()> {
        match expr {
            Expression::Identifier(name) => Ok(PlaceExpr::name(name.value().clone())),
            Expression::MemberAccess { object, field } => {
                let mut place = PlaceExpr::try_from(object.as_ref())?;
                place.sub_segments.push(PlaceExprSubSegment::Member(field.value().clone()));
                Ok(place)
            }
            Expression::IndexAccess { array, index } => {
                // Only support literal integer indices for now
                if let Expression::Literal(Literal::Integer(idx)) = index.as_ref() {
                    let mut place = PlaceExpr::try_from(array.as_ref())?;
                    place.sub_segments.push(PlaceExprSubSegment::IntSubscript(*idx));
                    Ok(place)
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }
}
```

#### 4. Ruff's PlaceTable with Efficient Lookups

```rust
// From ruff: Efficient place storage and lookup
use hashbrown::HashTable;
use rustc_hash::FxHasher;

#[derive(Default)]
pub struct PlaceTable {
    /// The place expressions in this scope
    places: IndexVec<ScopedPlaceId, PlaceExprWithFlags>,
    /// Hash table for O(1) lookups
    place_set: HashTable<ScopedPlaceId>,
}

impl PlaceTable {
    /// Returns the [`ScopedPlaceId`] of the place named `name`
    pub fn place_id_by_name(&self, name: &str) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_name(name), |id| {
                self.place_expr(*id).as_name() == Some(name)
            })
            .copied()
    }

    fn hash_name(name: &str) -> u64 {
        let mut hasher = FxHasher::default();
        name.hash(&mut hasher);
        hasher.finish()
    }
}
```

#### 5. Adapt PlaceTable for Cairo-M

```rust
// New place/table.rs for Cairo-M
use hashbrown::HashTable;
use index_vec::IndexVec;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Default)]
pub struct PlaceTable {
    /// All places in this scope, indexed by ScopedPlaceId
    places: IndexVec<ScopedPlaceId, PlaceExprWithFlags>,
    /// Hash table for O(1) lookups by name or expression
    place_set: HashTable<ScopedPlaceId>,
}

impl PlaceTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new place and return its ID
    pub fn add_place(&mut self, place: PlaceExpr, flags: PlaceFlags) -> ScopedPlaceId {
        let hash = Self::hash_place_expr(&place);

        // Check if place already exists
        if let Some(&existing_id) = self.place_set.find(hash, |id| {
            self.places[*id].expr == place
        }) {
            // Update flags for existing place
            self.places[existing_id].flags |= flags;
            return existing_id;
        }

        // Add new place
        let place_with_flags = PlaceExprWithFlags { expr: place, flags };
        let id = self.places.push(place_with_flags);
        self.place_set.insert_unique(hash, id, |id| {
            Self::hash_place_expr(&self.places[*id].expr)
        });
        id
    }

    /// Look up a place by name (for simple identifiers)
    pub fn place_id_by_name(&self, name: &str) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_name(name), |id| {
                self.places[*id].expr.as_name() == Some(name)
            })
            .copied()
    }

    /// Look up a place by expression
    pub fn place_id_by_expr(&self, expr: &PlaceExpr) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_place_expr(expr), |id| {
                &self.places[*id].expr == expr
            })
            .copied()
    }

    /// Get place by ID
    pub fn place(&self, id: ScopedPlaceId) -> Option<&PlaceExprWithFlags> {
        self.places.get(id)
    }

    /// Mark a place as used
    pub fn mark_as_used(&mut self, id: ScopedPlaceId) {
        if let Some(place) = self.places.get_mut(id) {
            place.flags.insert(PlaceFlags::USED);
        }
    }

    fn hash_name(name: &str) -> u64 {
        let mut hasher = FxHasher::default();
        name.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_place_expr(expr: &PlaceExpr) -> u64 {
        let mut hasher = FxHasher::default();
        // For simple names, just hash the name so lookups by name work
        if expr.is_name() {
            expr.root_name.hash(&mut hasher);
        } else {
            // Hash the full expression
            expr.hash(&mut hasher);
        }
        hasher.finish()
    }
}
```

#### 6. Migration Strategy

```rust
// In SemanticIndexBuilder, replace current place management
impl<'db> SemanticIndexBuilder<'db> {
    fn add_place(&mut self, name: &str, flags: PlaceFlags) -> ScopedPlaceId {
        // Old code:
        // let place_id = self.current_place_table_mut()
        //     .add_place(name.to_string(), flags);

        // New code:
        let place_expr = PlaceExpr::name(name.to_string());
        let scope_id = self.current_scope();
        self.place_tables[scope_id].add_place(place_expr, flags)
    }

    fn add_place_from_expr(&mut self, expr: &Expression, flags: PlaceFlags) -> Option<ScopedPlaceId> {
        // New capability: handle complex assignment targets
        let place_expr = PlaceExpr::try_from(expr).ok()?;
        let scope_id = self.current_scope();
        Some(self.place_tables[scope_id].add_place(place_expr, flags))
    }
}
```

This refactoring enables:

- Support for `struct.field = value` assignments
- Support for `array[0] = value` assignments
- O(1) lookups with specialized hashing
- Better extensibility for future assignment target types

## Issue 2: Implement Proper Visitor Pattern for Semantic Analysis

### What

Replace the current ad-hoc AST traversal in `SemanticIndexBuilder` with a proper
visitor pattern implementation following ruff's approach.

### Why

The current implementation mixes traversal logic with semantic analysis, making
it difficult to:

- Add new analysis passes
- Maintain consistent traversal order
- Handle complex control flow correctly
- Separate concerns between AST navigation and semantic logic

Ruff's visitor pattern provides:

- Clear separation between traversal and analysis
- Consistent handling of all AST nodes
- Easy extension for new language features
- Better error recovery

### How

#### 1. Current Cairo-M Implementation

```rust
// Current ad-hoc traversal mixed with analysis
impl<'db> SemanticIndexBuilder<'db> {
    fn visit_statement(&mut self, stmt: &Spanned<Statement>) {
        match stmt.value() {
            Statement::Let { pattern, value, .. } => {
                // Traversal and analysis mixed together
                let value_expr_id = self.visit_expression(value);
                match pattern {
                    Pattern::Identifier(name) => {
                        // Direct manipulation here
                        self.add_place_with_definition(...);
                    }
                    // ...
                }
            }
            // ... lots of pattern matching
        }
    }
}
```

#### 2. Ruff's Visitor Pattern Architecture

```rust
// From ruff: ruff_python_ast/src/visitor.rs
use ruff_python_ast as ast;

pub trait Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        walk_expr(self, expr);
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        walk_pattern(self, pattern);
    }

    // ... other visit methods
}

// Default traversal implementations
pub fn walk_stmt<'a, V: Visitor<'a>>(visitor: &mut V, stmt: &'a ast::Stmt) {
    match stmt {
        ast::Stmt::Assign(node) => {
            visitor.visit_expr(&node.value);
            for target in &node.targets {
                visitor.visit_expr(target);
            }
        }
        // ... other statement types
    }
}
```

#### 3. How Ruff Uses Visitor in SemanticIndexBuilder

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/builder.rs
impl<'ast> Visitor<'ast> for SemanticIndexBuilder<'_, 'ast> {
    fn visit_stmt(&mut self, stmt: &'ast ast::Stmt) {
        match stmt {
            ast::Stmt::Assign(node) => {
                // First, use default traversal for expressions
                self.visit_expr(&node.value);
                let value = self.add_standalone_assigned_expression(&node.value, node);

                // Then handle semantic analysis
                for target in &node.targets {
                    self.add_unpackable_assignment(&Unpackable::Assign(node), target, value);
                }
            }
            _ => {
                // Delegate to default traversal
                walk_stmt(self, stmt);
            }
        }
    }
}
```

#### 4. Adapt Visitor Pattern for Cairo-M

```rust
// New visitor.rs for Cairo-M
use crate::parser::{Expression, Statement, Pattern, Spanned};

/// Core visitor trait for AST traversal
pub trait Visitor<'ast> {
    /// Visit a statement node
    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
        walk_stmt(self, stmt);
    }

    /// Visit an expression node
    fn visit_expr(&mut self, expr: &'ast Spanned<Expression>) {
        walk_expr(self, expr);
    }

    /// Visit a pattern node
    fn visit_pattern(&mut self, pattern: &'ast Pattern) {
        walk_pattern(self, pattern);
    }

    /// Visit a function body
    fn visit_body(&mut self, stmts: &'ast [Spanned<Statement>]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }
}

/// Default statement traversal
pub fn walk_stmt<'ast, V: Visitor<'ast>>(visitor: &mut V, stmt: &'ast Spanned<Statement>) {
    match stmt.value() {
        Statement::Let { pattern, value, .. } => {
            // Visit in evaluation order
            visitor.visit_expr(value);
            visitor.visit_pattern(pattern);
        }
        Statement::Assignment { lhs, rhs } => {
            visitor.visit_expr(rhs);
            visitor.visit_expr(lhs);
        }
        Statement::Return { value } => {
            if let Some(expr) = value {
                visitor.visit_expr(expr);
            }
        }
        Statement::If { condition, then_block, else_block } => {
            visitor.visit_expr(condition);
            visitor.visit_stmt(then_block);
            if let Some(else_stmt) = else_block {
                visitor.visit_stmt(else_stmt);
            }
        }
        Statement::Block(statements) => {
            visitor.visit_body(statements);
        }
        Statement::Expression(expr) => {
            visitor.visit_expr(expr);
        }
        Statement::Loop { body } => {
            visitor.visit_stmt(body);
        }
        Statement::While { condition, body } => {
            visitor.visit_expr(condition);
            visitor.visit_stmt(body);
        }
        Statement::Break | Statement::Continue => {
            // No sub-nodes to visit
        }
        Statement::Const(const_def) => {
            visitor.visit_expr(&const_def.value);
        }
    }
}

/// Default expression traversal
pub fn walk_expr<'ast, V: Visitor<'ast>>(visitor: &mut V, expr: &'ast Spanned<Expression>) {
    match expr.value() {
        Expression::BinaryOp { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        Expression::UnaryOp { expr, .. } => {
            visitor.visit_expr(expr);
        }
        Expression::FunctionCall { callee, args } => {
            visitor.visit_expr(callee);
            for arg in args {
                visitor.visit_expr(arg);
            }
        }
        Expression::MemberAccess { object, .. } => {
            visitor.visit_expr(object);
        }
        Expression::IndexAccess { array, index } => {
            visitor.visit_expr(array);
            visitor.visit_expr(index);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                visitor.visit_expr(value);
            }
        }
        Expression::Tuple(exprs) => {
            for expr in exprs {
                visitor.visit_expr(expr);
            }
        }
        Expression::Identifier(_) | Expression::Literal(_) | Expression::BooleanLiteral(_) => {
            // Leaf nodes - no sub-expressions
        }
    }
}

/// Default pattern traversal
pub fn walk_pattern<'ast, V: Visitor<'ast>>(visitor: &mut V, pattern: &'ast Pattern) {
    match pattern {
        Pattern::Identifier(_) => {
            // Leaf node
        }
        Pattern::Tuple(patterns) => {
            for pat in patterns {
                visitor.visit_pattern(pat);
            }
        }
    }
}
```

#### 5. Refactor SemanticIndexBuilder with Visitor Pattern

```rust
// Updated semantic_index/builder.rs
use crate::visitor::{Visitor, walk_stmt, walk_expr};

impl<'ast> Visitor<'ast> for SemanticIndexBuilder<'_, 'ast> {
    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
        // Map statement span to scope for IDE features
        let current_scope = self.current_scope();
        self.index.set_scope_for_span(stmt.span(), current_scope);

        match stmt.value() {
            Statement::Let { pattern, value, statement_type } => {
                // Visit expression first (evaluation order)
                self.visit_expr(value);
                let value_expr_id = self.index.add_expression(ExpressionInfo {
                    file: self.file,
                    ast_node: value.value().clone(),
                    ast_span: value.span(),
                    scope_id: current_scope,
                });

                // Then handle the pattern binding
                self.handle_pattern_binding(pattern, value_expr_id, statement_type);
            }
            Statement::If { condition, then_block, else_block } => {
                // Handle control flow analysis
                self.visit_expr(condition);
                let predicate = self.record_expression_constraint(condition);

                // Visit then branch with constraint
                self.with_constraint(predicate, |builder| {
                    builder.visit_stmt(then_block);
                });

                // Visit else branch with negated constraint
                if let Some(else_stmt) = else_block {
                    self.with_constraint(!predicate, |builder| {
                        builder.visit_stmt(else_stmt);
                    });
                }
            }
            _ => {
                // Use default traversal for other statements
                walk_stmt(self, stmt);
            }
        }
    }

    fn visit_expr(&mut self, expr: &'ast Spanned<Expression>) {
        // Track expression for type inference
        let expr_id = self.index.add_expression(ExpressionInfo {
            file: self.file,
            ast_node: expr.value().clone(),
            ast_span: expr.span(),
            scope_id: self.current_scope(),
        });

        match expr.value() {
            Expression::Identifier(name) => {
                // Track identifier usage
                let usage = IdentifierUsage {
                    name: name.value().clone(),
                    span: name.span(),
                    scope_id: self.current_scope(),
                };
                let usage_idx = self.index.add_identifier_usage(usage);

                // Resolve and link to definition
                if let Some((def_scope, place_id)) =
                    self.index.resolve_name(name.value(), self.current_scope())
                {
                    if let Some(place_table) = self.index.place_table_mut(def_scope) {
                        place_table.mark_as_used(place_id);
                    }

                    if let Some((def_id, _)) =
                        self.index.definition_for_place(def_scope, place_id)
                    {
                        self.index.add_use(usage_idx, def_id);
                    }
                }
            }
            _ => {
                // Use default traversal for other expressions
                walk_expr(self, expr);
            }
        }
    }
}
```

#### 6. Enable Multi-Pass Analysis

```rust
// Create specialized visitors for different passes
pub struct DeclarationCollector<'db, 'ast> {
    builder: &'db mut SemanticIndexBuilder<'db, 'ast>,
}

impl<'ast> Visitor<'ast> for DeclarationCollector<'_, 'ast> {
    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
        match stmt.value() {
            // Only process declarations in first pass
            Statement::Const(const_def) => {
                self.builder.declare_constant(const_def);
            }
            // Skip function bodies in first pass
            _ => {}
        }
    }
}

pub struct BodyProcessor<'db, 'ast> {
    builder: &'db mut SemanticIndexBuilder<'db, 'ast>,
}

impl<'ast> Visitor<'ast> for BodyProcessor<'_, 'ast> {
    fn visit_stmt(&mut self, stmt: &'ast Spanned<Statement>) {
        // Process function bodies in second pass
        walk_stmt(self, stmt);
    }
}

// Use two-pass analysis
impl<'db, 'ast> SemanticIndexBuilder<'db, 'ast> {
    pub fn build(mut self) -> SemanticIndex {
        // Pass 1: Collect declarations
        let mut decl_collector = DeclarationCollector { builder: &mut self };
        for item in self.module.items() {
            decl_collector.visit_stmt(item);
        }

        // Pass 2: Process bodies
        let mut body_processor = BodyProcessor { builder: &mut self };
        for item in self.module.items() {
            body_processor.visit_stmt(item);
        }

        self.index
    }
}
```

This refactoring enables:

- Clean separation of traversal logic from semantic analysis
- Easy addition of new analysis passes
- Consistent traversal order across all visitors
- Better maintainability and extensibility
- Reusable traversal logic for other tools (linters, formatters)

## Issue 3: Enhance Scope Management with Reachability Tracking

### What

Implement ruff's sophisticated scope management system including reachability
constraints and flow-sensitive analysis for dead code detection and type
narrowing.

### Why

The current implementation tracks scopes but lacks:

- Reachability analysis for dead code detection
- Flow-sensitive type narrowing
- Proper handling of control flow statements
- Support for eager vs lazy evaluation contexts

Ruff's approach enables:

- Dead code detection (code after `return` is unreachable)
- Type narrowing in conditionals (`if x is not None`)
- Proper handling of break/continue in loops
- Context-aware symbol resolution

### How

#### 1. Current Cairo-M Scope Tracking

```rust
// Current simple scope tracking
impl<'db> SemanticIndexBuilder<'db> {
    fn push_scope(&mut self, kind: ScopeKind) -> FileScopeId {
        let parent = self.current_scope();
        let scope_id = self.index.scopes.push(Scope::new(Some(parent), kind));
        self.scope_stack.push(scope_id);
        scope_id
    }

    fn visit_if_statement(&mut self, if_stmt: &IfStatement) {
        // No reachability tracking
        self.visit_expr(&if_stmt.condition);
        self.visit_stmt(&if_stmt.then_block);
        if let Some(else_block) = &if_stmt.else_block {
            self.visit_stmt(else_block);
        }
    }
}
```

#### 2. Ruff's Predicate System

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/predicate.rs
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Predicate<'db> {
    pub(crate) node: PredicateNode<'db>,
    pub(crate) is_positive: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PredicateOrLiteral<'db> {
    Literal(bool),
    Predicate(Predicate<'db>),
}

impl PredicateOrLiteral<'_> {
    pub(crate) fn negated(self) -> Self {
        match self {
            PredicateOrLiteral::Literal(value) => PredicateOrLiteral::Literal(!value),
            PredicateOrLiteral::Predicate(Predicate { node, is_positive }) => {
                PredicateOrLiteral::Predicate(Predicate {
                    node,
                    is_positive: !is_positive,
                })
            }
        }
    }
}

// Special predicate IDs
impl ScopedPredicateId {
    /// A special ID that is used for an "always true" predicate.
    pub(crate) const ALWAYS_TRUE: ScopedPredicateId = ScopedPredicateId(0xffff_ffff);

    /// A special ID that is used for an "always false" predicate.
    pub(crate) const ALWAYS_FALSE: ScopedPredicateId = ScopedPredicateId(0xffff_fffe);
}
```

#### 3. Ruff's Reachability Constraint System

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/reachability_constraints.rs
// The system uses ternary logic: always-true, always-false, ambiguous

// Ternary OR operation for merging control flow:
//       | OR           | always-false | ambiguous    | always-true  |
//       |--------------|--------------|--------------|--------------|
//       | always false | always-false | ambiguous    | always-true  |
//       | ambiguous    | ambiguous    | ambiguous    | always-true  |
//       | always true  | always-true  | always-true  | always-true  |

// From builder.rs - tracking reachability in if statements
impl<'ast> SemanticIndexBuilder<'_, 'ast> {
    fn visit_stmt_if(&mut self, node: &'ast ast::StmtIf) {
        self.visit_expr(&node.test);
        let mut no_branch_taken = self.flow_snapshot();
        let mut last_predicate = self.record_expression_narrowing_constraint(&node.test);
        let mut last_reachability_constraint =
            self.record_reachability_constraint(last_predicate);

        // Visit then block with constraint
        self.visit_body(&node.body);

        let mut post_clauses: Vec<FlowSnapshot> = vec![];

        // Handle elif/else chains
        for (clause_test, clause_body) in elif_else_clauses {
            post_clauses.push(self.flow_snapshot());
            self.flow_restore(no_branch_taken.clone());

            // Record negated constraint for branches not taken
            self.record_negated_narrowing_constraint(last_predicate);
            self.record_negated_reachability_constraint(last_reachability_constraint);

            if let Some(elif_test) = clause_test {
                self.visit_expr(elif_test);
                no_branch_taken = self.flow_snapshot();
                last_predicate = self.record_expression_narrowing_constraint(elif_test);
                last_reachability_constraint =
                    self.record_reachability_constraint(last_predicate);
            }

            self.visit_body(clause_body);
        }

        // Merge all branches
        for post_clause_state in post_clauses {
            self.flow_merge(post_clause_state);
        }
    }
}
```

#### 4. Ruff's Loop Handling with Break States

```rust
// From builder.rs - handling loops and break statements
#[derive(Default)]
struct Loop {
    break_states: Vec<FlowSnapshot>,
}

impl<'ast> SemanticIndexBuilder<'_, 'ast> {
    fn push_loop(&mut self) -> Option<Loop> {
        self.current_scope_info_mut()
            .current_loop
            .replace(Loop::default())
    }

    fn pop_loop(&mut self, outer_loop: Option<Loop>) -> Loop {
        std::mem::replace(&mut self.current_scope_info_mut().current_loop, outer_loop)
            .expect("pop_loop() should not be called without a prior push_loop()")
    }

    fn visit_stmt_while(&mut self, node: &'ast ast::StmtWhile) {
        self.visit_expr(&node.test);

        let pre_loop = self.flow_snapshot();
        let predicate = self.record_expression_narrowing_constraint(&node.test);
        self.record_reachability_constraint(predicate);

        let outer_loop = self.push_loop();
        self.visit_body(&node.body);
        let this_loop = self.pop_loop(outer_loop);

        // Merge pre-loop state (condition could be false initially)
        self.flow_merge(pre_loop);

        // Record negated constraint for else branch
        self.record_negated_reachability_constraint(later_reachability_constraint);
        self.record_negated_narrowing_constraint(predicate);

        self.visit_body(&node.orelse);

        // Merge break states after visiting else
        for break_state in this_loop.break_states {
            self.flow_merge(break_state);
        }
    }
}
```

#### 5. Adapt Reachability System for Cairo-M

```rust
// New reachability.rs for Cairo-M
use index_vec::{IndexVec, Idx};

/// Ternary logic for reachability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reachability {
    AlwaysTrue,
    AlwaysFalse,
    Ambiguous,
}

impl Reachability {
    /// Ternary OR operation for control flow merging
    pub fn or(self, other: Self) -> Self {
        use Reachability::*;
        match (self, other) {
            (AlwaysTrue, _) | (_, AlwaysTrue) => AlwaysTrue,
            (Ambiguous, _) | (_, Ambiguous) => Ambiguous,
            (AlwaysFalse, AlwaysFalse) => AlwaysFalse,
        }
    }

    /// Ternary AND operation for sequential constraints
    pub fn and(self, other: Self) -> Self {
        use Reachability::*;
        match (self, other) {
            (AlwaysFalse, _) | (_, AlwaysFalse) => AlwaysFalse,
            (Ambiguous, _) | (_, Ambiguous) => Ambiguous,
            (AlwaysTrue, AlwaysTrue) => AlwaysTrue,
        }
    }
}

/// A predicate representing a condition in the code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    pub expr_id: ExpressionId,
    pub is_positive: bool,
}

impl Predicate {
    pub fn negated(self) -> Self {
        Self {
            expr_id: self.expr_id,
            is_positive: !self.is_positive,
        }
    }
}

/// Unique ID for predicates within a scope
index_vec::define_index_type! {
    pub struct ScopedPredicateId = u32;
    MAX_INDEX = 0xffff_fffd;
}

impl ScopedPredicateId {
    pub const ALWAYS_TRUE: Self = Self::from_raw(0xffff_ffff);
    pub const ALWAYS_FALSE: Self = Self::from_raw(0xffff_fffe);
}

/// Reachability constraints for a scope
#[derive(Debug, Default)]
pub struct ReachabilityConstraints {
    predicates: IndexVec<ScopedPredicateId, Predicate>,
    atoms: Vec<ReachabilityAtom>,
}

#[derive(Debug, Clone)]
struct ReachabilityAtom {
    predicate_id: ScopedPredicateId,
    reachability: Reachability,
}

impl ReachabilityConstraints {
    pub fn add_predicate(&mut self, predicate: Predicate) -> ScopedPredicateId {
        self.predicates.push(predicate)
    }

    pub fn add_atom(&mut self, predicate_id: ScopedPredicateId) -> ScopedReachabilityConstraintId {
        let atom = ReachabilityAtom {
            predicate_id,
            reachability: Reachability::Ambiguous,
        };
        ScopedReachabilityConstraintId::new(self.atoms.len())
    }
}
```

#### 6. Enhanced Scope with Reachability

```rust
// Updated place.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Parent scope, if any (None for module scope)
    pub parent: Option<FileScopeId>,
    /// The kind of scope this represents
    pub kind: ScopeKind,
    /// Reachability constraint for this scope
    pub reachability: ScopedReachabilityConstraintId,
    /// Whether this scope is in a type checking block
    pub in_type_checking_block: bool,
}
```

#### 7. Flow Snapshots for Control Flow Merging

```rust
// New flow.rs for Cairo-M
#[derive(Debug, Clone)]
pub struct FlowSnapshot {
    /// Current bindings at this point
    bindings: Vec<BindingState>,
    /// Active narrowing constraints
    constraints: Vec<NarrowingConstraint>,
    /// Reachability state
    reachability: Reachability,
}

impl FlowSnapshot {
    pub fn merge(self, other: Self) -> Self {
        Self {
            bindings: merge_bindings(self.bindings, other.bindings),
            constraints: merge_constraints(self.constraints, other.constraints),
            reachability: self.reachability.or(other.reachability),
        }
    }
}
```

#### 8. Integrate into SemanticIndexBuilder

```rust
// Updated semantic_index/builder.rs
impl<'db> SemanticIndexBuilder<'db> {
    fn record_reachability_constraint(&mut self, predicate: Predicate) -> ScopedReachabilityConstraintId {
        let predicate_id = self.current_reachability_constraints_mut()
            .add_predicate(predicate);
        self.current_reachability_constraints_mut()
            .add_atom(predicate_id)
    }

    fn visit_if_statement(&mut self, if_stmt: &IfStatement) {
        // Visit condition
        self.visit_expr(&if_stmt.condition);

        // Create predicate from condition
        let predicate = Predicate {
            expr_id: self.expression_id(&if_stmt.condition),
            is_positive: true,
        };

        // Snapshot before branching
        let pre_branch = self.flow_snapshot();

        // Record constraint for then branch
        let constraint = self.record_reachability_constraint(predicate.clone());
        self.visit_stmt(&if_stmt.then_block);
        let post_then = self.flow_snapshot();

        // Restore and handle else branch
        self.flow_restore(pre_branch);
        self.record_reachability_constraint(predicate.negated());

        if let Some(else_block) = &if_stmt.else_block {
            self.visit_stmt(else_block);
        }

        // Merge control flow
        self.flow_merge(post_then);
    }

    fn visit_return_statement(&mut self, _ret: &ReturnStatement) {
        // Mark subsequent code as unreachable
        self.current_reachability = Reachability::AlwaysFalse;
    }

    fn visit_loop_statement(&mut self, loop_stmt: &LoopStatement) {
        let outer_loop = self.push_loop();

        // Loops might not execute, so record ambiguous constraint
        self.record_reachability_constraint(Predicate {
            expr_id: ExpressionId::AMBIGUOUS,
            is_positive: true,
        });

        self.visit_stmt(&loop_stmt.body);
        let this_loop = self.pop_loop(outer_loop);

        // Merge break states
        for break_state in this_loop.break_states {
            self.flow_merge(break_state);
        }
    }

    fn visit_break_statement(&mut self) {
        if let Some(loop_info) = self.current_loop_mut() {
            loop_info.break_states.push(self.flow_snapshot());
        }
        self.current_reachability = Reachability::AlwaysFalse;
    }
}
```

#### 9. Usage Example

```rust
// Example: Dead code detection
fn example() {
    let x = 1;
    if condition {
        return;  // Reachability becomes AlwaysFalse
        let y = 2;  // This binding is unreachable
    }
    // Reachability here depends on condition (Ambiguous)
    let z = 3;
}

// Example: Type narrowing
fn narrow_type(x: Option<i32>) {
    if x.is_some() {
        // Here we know x is Some, constraint is recorded
        let val = x.unwrap();  // Safe due to narrowing
    }
}
```

This refactoring enables:

- Dead code detection after terminal statements
- Flow-sensitive type narrowing in conditionals
- Proper handling of loop control flow
- Foundation for advanced type inference

## Issue 4: Optimize Data Structures for Performance

### What

Replace current HashMap-based lookups with ruff's optimized data structures
using IndexVec and specialized hash tables for better memory efficiency and
cache locality.

### Why

The current implementation uses standard HashMaps which:

- Have higher memory overhead (hashing, collision resolution)
- Require heap allocations for keys
- Don't leverage the sequential nature of scope IDs
- Have poor cache locality for sequential access

Ruff's approach uses:

- `IndexVec` for O(1) access by ID with minimal overhead
- `hashbrown::HashTable` for custom hashing without key storage
- Binary search for range-based lookups
- Compact representation of relationships

### How

#### 1. Current Cairo-M Implementation

```rust
// Current HashMap-based storage
pub struct SemanticIndex {
    // Using HashMap with String keys
    definitions: HashMap<String, Definition>,
    scopes: HashMap<ScopeId, Scope>,
    expressions: HashMap<ExpressionId, ExpressionInfo>,
    // Inefficient lookups
    scope_by_span: HashMap<Span, ScopeId>,
}
```

#### 2. Ruff's IndexVec Architecture

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index.rs
use ruff_index::IndexVec;

pub(crate) struct SemanticIndex<'db> {
    /// List of all place tables in this file, indexed by scope.
    place_tables: IndexVec<FileScopeId, Arc<PlaceTable>>,

    /// List of all scopes in this file.
    scopes: IndexVec<FileScopeId, Scope>,

    /// Map expressions to their corresponding scope.
    scopes_by_expression: ExpressionsScopeMap,

    /// Use-def map for each scope in this file.
    use_def_maps: IndexVec<FileScopeId, ArcUseDefMap<'db>>,

    /// Lookup table to map between node ids and ast nodes.
    ast_ids: IndexVec<FileScopeId, AstIds>,

    /// The Salsa ingredient for each scope in this file.
    scope_ids_by_scope: IndexVec<FileScopeId, ScopeId<'db>>,
}

// Efficient O(1) access
impl<'db> SemanticIndex<'db> {
    #[track_caller]
    pub(super) fn place_table(&self, scope_id: FileScopeId) -> Arc<PlaceTable> {
        self.place_tables[scope_id].clone()
    }

    #[track_caller]
    pub(super) fn use_def_map(&self, scope_id: FileScopeId) -> ArcUseDefMap<'_> {
        self.use_def_maps[scope_id].clone()
    }

    #[track_caller]
    pub(crate) fn scope(&self, id: FileScopeId) -> &Scope {
        &self.scopes[id]
    }
}
```

#### 3. Ruff's HashTable for Place Lookups

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index.rs
type PlaceSet = hashbrown::HashTable<ScopedPlaceId>;

// From place.rs - efficient place storage without key duplication
pub struct PlaceTable {
    /// The place expressions in this scope.
    places: IndexVec<ScopedPlaceId, PlaceExprWithFlags>,

    /// The set of places - stores only IDs, not keys
    place_set: PlaceSet,
}

impl PlaceTable {
    /// Returns the [`ScopedPlaceId`] of the place named `name`.
    pub(crate) fn place_id_by_name(&self, name: &str) -> Option<ScopedPlaceId> {
        self.place_set
            .find(Self::hash_name(name), |id| {
                self.place_expr(*id).as_name().map(Name::as_str) == Some(name)
            })
            .copied()
    }

    fn hash_name(name: &str) -> u64 {
        let mut hasher = FxHasher::default();
        name.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_place_expr<'e>(place_expr: impl Into<PlaceExprRef<'e>>) -> u64 {
        let place_expr: PlaceExprRef = place_expr.into();
        let mut hasher = FxHasher::default();

        // Special case for simple names (e.g. "foo"). Only hash the name so
        // that a lookup by name can find it (see `place_by_name`).
        if place_expr.sub_segments.is_empty() {
            place_expr.root_name.as_str().hash(&mut hasher);
        } else {
            place_expr.hash(&mut hasher);
        }
        hasher.finish()
    }
}
```

#### 4. Binary Search for Expression Scope Mapping

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index.rs
/// Interval map that maps a range of expression node ids to their corresponding scopes.
///
/// Lookups require `O(log n)` time, where `n` is roughly the number of scopes
#[derive(Eq, PartialEq, Debug, Default)]
struct ExpressionsScopeMap(Box<[(std::ops::RangeInclusive<NodeIndex>, FileScopeId)]>);

impl ExpressionsScopeMap {
    fn try_get<E>(&self, node: &E) -> Option<FileScopeId>
    where
        E: HasTrackedScope,
    {
        let node_index = node.node_index().load();

        let entry = self
            .0
            .binary_search_by_key(&node_index, |(range, _)| *range.start());

        let index = match entry {
            Ok(index) => index,
            Err(index) => index.checked_sub(1)?,
        };

        let (range, scope) = &self.0[index];
        if range.contains(&node_index) {
            Some(*scope)
        } else {
            None
        }
    }
}
```

#### 5. Ruff's Use-Def Map with Dense Storage

```rust
// From use_def.rs - all data structures use IndexVec for efficiency
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UseDefMap<'db> {
    /// Array of [`Definition`] in this scope
    all_definitions: IndexVec<ScopedDefinitionId, DefinitionState<'db>>,

    /// Array of predicates in this scope
    predicates: Predicates<'db>,

    /// Array of narrowing constraints in this scope
    narrowing_constraints: NarrowingConstraints,

    /// Array of reachability constraints in this scope
    reachability_constraints: ReachabilityConstraints,

    /// [`Bindings`] reaching a [`ScopedUseId`]
    bindings_by_use: IndexVec<ScopedUseId, Bindings>,

    /// [`PlaceState`] visible at end of scope for each place
    end_of_scope_places: IndexVec<ScopedPlaceId, PlaceState>,

    /// All potentially reachable bindings and declarations, for each place
    reachable_definitions: IndexVec<ScopedPlaceId, ReachableDefinitions>,
}
```

#### 6. Adapt Data Structures for Cairo-M

```rust
// New semantic_index.rs with optimized storage
use index_vec::IndexVec;
use hashbrown::HashTable;
use rustc_hash::FxHasher;

pub struct SemanticIndex {
    /// Dense storage for all scopes
    scopes: IndexVec<FileScopeId, Scope>,

    /// Dense storage for place tables
    place_tables: IndexVec<FileScopeId, Arc<PlaceTable>>,

    /// Dense storage for use-def maps
    use_def_maps: IndexVec<FileScopeId, Arc<UseDefMap>>,

    /// Binary search structure for expression->scope mapping
    expression_scopes: ExpressionScopeMap,

    /// Only store mappings that can't use dense storage
    definitions_by_node: FxHashMap<NodeKey, DefinitionId>,
}

impl SemanticIndex {
    /// O(1) scope access
    #[inline]
    pub fn scope(&self, id: FileScopeId) -> &Scope {
        &self.scopes[id]
    }

    /// O(1) place table access
    #[inline]
    pub fn place_table(&self, id: FileScopeId) -> &PlaceTable {
        &self.place_tables[id]
    }

    /// O(log n) expression scope lookup
    pub fn expression_scope(&self, span: Span) -> Option<FileScopeId> {
        self.expression_scopes.find(span)
    }
}

/// Efficient interval map for span->scope lookups
#[derive(Debug, Default)]
pub struct ExpressionScopeMap {
    /// Sorted by start position for binary search
    intervals: Box<[(SpanRange, FileScopeId)]>,
}

impl ExpressionScopeMap {
    pub fn find(&self, span: Span) -> Option<FileScopeId> {
        let pos = span.start;

        // Binary search for the interval containing this position
        let idx = self.intervals
            .binary_search_by_key(&pos, |(range, _)| range.start)
            .unwrap_or_else(|idx| idx.saturating_sub(1));

        let (range, scope_id) = &self.intervals[idx];
        if range.contains(pos) {
            Some(*scope_id)
        } else {
            None
        }
    }
}
```

#### 7. Memory Optimization Techniques

```rust
// From ruff's PlaceTableBuilder
impl PlaceTableBuilder {
    pub(super) fn add_symbol(&mut self, name: Name) -> (ScopedPlaceId, bool) {
        let hash = PlaceTable::hash_name(&name);
        let entry = self.table.place_set.entry(
            hash,
            |id| self.table.places[*id].as_name() == Some(&name),
            |id| PlaceTable::hash_place_expr(&self.table.places[*id].expr),
        );

        match entry {
            Entry::Occupied(entry) => (*entry.get(), false),
            Entry::Vacant(entry) => {
                let symbol = PlaceExprWithFlags::name(name);
                let id = self.table.places.push(symbol);
                entry.insert(id);
                (id, true)
            }
        }
    }
}

// Shrinking after building
impl PlaceTable {
    fn shrink_to_fit(&mut self) {
        self.places.shrink_to_fit();
    }
}
```

#### 8. Performance Benefits

```rust
// Before: HashMap lookups
let definition = self.definitions.get(&name)?;  // String key, hash computation

// After: IndexVec lookups
let definition = &self.definitions[def_id];     // Direct array access

// Before: Finding expression scope
for (span, scope) in &self.scope_by_span {      // O(n) iteration
    if span.contains(pos) { return Some(scope); }
}

// After: Binary search
self.expression_scopes.find(span)               // O(log n) lookup
```

This refactoring provides:

- **Memory efficiency**: IndexVec uses contiguous memory with no per-element
  overhead
- **Cache locality**: Sequential access patterns benefit from CPU cache
- **Fast lookups**: O(1) for ID-based access, O(log n) for range queries
- **No key duplication**: HashTable stores only IDs, keys are in IndexVec
- **Predictable performance**: No hash collisions or rehashing

## Issue 5: Improve Use-Def Analysis with Flow Sensitivity

### What

Enhance the use-def analysis to support flow-sensitive tracking following ruff's
sophisticated approach, enabling accurate type inference across control flow
branches.

### Why

The current implementation tracks uses and definitions but lacks:

- Flow-sensitive analysis for better type inference
- Handling of conditional definitions
- Support for narrowing constraints
- Proper merge semantics at control flow joins

Ruff's approach enables:

- Type narrowing in conditionals (`if x is not None`)
- Tracking multiple possible bindings per use
- Correct handling of shadowing in branches
- Proper merge semantics for control flow joins

### How

#### 1. Current Cairo-M Implementation

```rust
// Current simple use-def tracking
pub struct UseDefMap {
    definitions: HashMap<String, Definition>,
    uses: HashMap<UseId, DefinitionId>,
    // No flow sensitivity
}

impl SemanticIndexBuilder {
    fn handle_assignment(&mut self, target: &str, value: &Expression) {
        let def_id = self.create_definition(target, value);
        // Simply overwrites previous definition
        self.use_def_map.definitions.insert(target.to_string(), def_id);
    }
}
```

#### 2. Ruff's Flow-Sensitive Architecture

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/use_def.rs
/// A snapshot of the definitions and constraints state at a particular point in control flow.
#[derive(Clone, Debug)]
pub(super) struct FlowSnapshot {
    place_states: IndexVec<ScopedPlaceId, PlaceState>,
    reachability: ScopedReachabilityConstraintId,
}

#[derive(Debug)]
pub(super) struct UseDefMapBuilder<'db> {
    /// Append-only array of [`DefinitionState`].
    all_definitions: IndexVec<ScopedDefinitionId, DefinitionState<'db>>,

    /// Live bindings at each so-far-recorded use.
    bindings_by_use: IndexVec<ScopedUseId, Bindings>,

    /// Currently live bindings and declarations for each place.
    place_states: IndexVec<ScopedPlaceId, PlaceState>,

    /// All potentially reachable bindings and declarations, for each place.
    reachable_definitions: IndexVec<ScopedPlaceId, ReachableDefinitions>,
}
```

#### 3. Ruff's PlaceState for Tracking Live Bindings

```rust
// From ruff: place_state.rs - tracking per control flow point
/// Live bindings for a single place at some point in control flow. Each live binding comes
/// with a set of narrowing constraints and a reachability constraint.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Bindings {
    /// A list of live bindings for this place, sorted by their `ScopedDefinitionId`
    live_bindings: SmallVec<[LiveBinding; 2]>,
}

/// One of the live bindings for a single place at some point in control flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LiveBinding {
    pub(super) binding: ScopedDefinitionId,
    pub(super) narrowing_constraint: ScopedNarrowingConstraint,
    pub(super) reachability_constraint: ScopedReachabilityConstraintId,
}

impl Bindings {
    /// Record a newly-encountered binding for this place.
    pub(super) fn record_binding(
        &mut self,
        binding: ScopedDefinitionId,
        reachability_constraint: ScopedReachabilityConstraintId,
        is_class_scope: bool,
        is_place_name: bool,
        previous_definitions: PreviousDefinitions,
    ) {
        // The new binding replaces all previous live bindings in this path
        if previous_definitions.are_shadowed() {
            self.live_bindings.clear();
        }
        self.live_bindings.push(LiveBinding {
            binding,
            narrowing_constraint: ScopedNarrowingConstraint::empty(),
            reachability_constraint,
        });
    }
}
```

#### 4. Control Flow Merging with Constraints

```rust
// From ruff: place_state.rs - merging at control flow joins
impl Bindings {
    /// Merge two sets of bindings from different control flow paths.
    fn merge(
        &mut self,
        b: Self,
        narrowing_constraints: &mut NarrowingConstraintsBuilder,
        reachability_constraints: &mut ReachabilityConstraintsBuilder,
    ) {
        // Merge the two sorted lists of bindings
        let a = std::mem::take(self);
        let a = a.live_bindings.into_iter();
        let b = b.live_bindings.into_iter();

        for zipped in a.merge_join_by(b, |a, b| a.binding.cmp(&b.binding)) {
            match zipped {
                EitherOrBoth::Both(a, b) => {
                    // If the same definition is visible through both paths, any constraint
                    // that applies on only one path is irrelevant to the resulting type from
                    // unioning the two paths, so we intersect the constraints.
                    let narrowing_constraint = narrowing_constraints
                        .intersect_constraints(a.narrowing_constraint, b.narrowing_constraint);

                    // For reachability constraints, we merge them using a ternary OR operation
                    let reachability_constraint = reachability_constraints
                        .add_or_constraint(a.reachability_constraint, b.reachability_constraint);

                    self.live_bindings.push(LiveBinding {
                        binding: a.binding,
                        narrowing_constraint,
                        reachability_constraint,
                    });
                }

                EitherOrBoth::Left(binding) | EitherOrBoth::Right(binding) => {
                    self.live_bindings.push(binding);
                }
            }
        }
    }
}
```

#### 5. Flow Snapshot Operations in Builder

```rust
// From ruff: builder.rs - flow operations
impl<'db> SemanticIndexBuilder<'db> {
    fn flow_snapshot(&self) -> FlowSnapshot {
        self.current_use_def_map().snapshot()
    }

    fn flow_restore(&mut self, state: FlowSnapshot) {
        self.current_use_def_map_mut().restore(state);
    }

    fn flow_merge(&mut self, state: FlowSnapshot) {
        self.current_use_def_map_mut().merge(state);
    }

    fn visit_stmt_if(&mut self, node: &'ast ast::StmtIf) {
        self.visit_expr(&node.test);

        // Snapshot before branching
        let mut no_branch_taken = self.flow_snapshot();

        // Record constraints for then branch
        let mut last_predicate = self.record_expression_narrowing_constraint(&node.test);
        let mut last_reachability_constraint =
            self.record_reachability_constraint(last_predicate);

        // Visit then block with constraint
        self.visit_body(&node.body);

        let mut post_clauses: Vec<FlowSnapshot> = vec![];

        // Handle elif/else chains
        for (clause_test, clause_body) in elif_else_clauses {
            // Save state after previous clause
            post_clauses.push(self.flow_snapshot());

            // Restore to state before any branch was taken
            self.flow_restore(no_branch_taken.clone());

            // Record negated constraint for branches not taken
            self.record_negated_narrowing_constraint(last_predicate);
            self.record_negated_reachability_constraint(last_reachability_constraint);

            // Process clause...
            self.visit_body(clause_body);
        }

        // Merge all branches
        for post_clause_state in post_clauses {
            self.flow_merge(post_clause_state);
        }
    }
}
```

#### 6. Adapt Flow-Sensitive Analysis for Cairo-M

```rust
// New use_def/flow.rs for Cairo-M
use index_vec::IndexVec;
use smallvec::SmallVec;

/// A live binding at a specific control flow point
#[derive(Debug, Clone)]
pub struct LiveBinding {
    /// The definition this binding refers to
    pub definition_id: DefinitionId,
    /// Type narrowing constraints that apply
    pub narrowing_constraints: NarrowingConstraintSet,
    /// Reachability constraint for this binding
    pub reachability: ScopedReachabilityConstraintId,
}

/// All live bindings for a place at a control flow point
#[derive(Debug, Clone, Default)]
pub struct PlaceBindings {
    /// Multiple bindings can be live (from different branches)
    live_bindings: SmallVec<[LiveBinding; 2]>,
}

impl PlaceBindings {
    /// Record a new binding, potentially shadowing previous ones
    pub fn record_binding(
        &mut self,
        definition_id: DefinitionId,
        reachability: ScopedReachabilityConstraintId,
        shadows_previous: bool,
    ) {
        if shadows_previous {
            self.live_bindings.clear();
        }

        self.live_bindings.push(LiveBinding {
            definition_id,
            narrowing_constraints: NarrowingConstraintSet::empty(),
            reachability,
        });
    }

    /// Apply a narrowing constraint to all live bindings
    pub fn apply_narrowing(&mut self, constraint: NarrowingConstraint) {
        for binding in &mut self.live_bindings {
            binding.narrowing_constraints.add(constraint);
        }
    }

    /// Merge bindings from another control flow path
    pub fn merge(&mut self, other: Self, constraints: &mut ConstraintBuilder) {
        let self_bindings = std::mem::take(&mut self.live_bindings);

        // Keep bindings that exist in either path
        for binding in self_bindings.into_iter().chain(other.live_bindings) {
            // Check if we already have this definition
            if let Some(existing) = self.live_bindings.iter_mut()
                .find(|b| b.definition_id == binding.definition_id)
            {
                // Merge constraints - intersection for narrowing
                existing.narrowing_constraints = constraints.intersect(
                    existing.narrowing_constraints,
                    binding.narrowing_constraints
                );
                // OR for reachability
                existing.reachability = constraints.or_reachability(
                    existing.reachability,
                    binding.reachability
                );
            } else {
                self.live_bindings.push(binding);
            }
        }
    }
}
```

#### 7. Flow State Management

```rust
// New use_def/builder.rs for Cairo-M
/// Complete flow state at a control flow point
#[derive(Debug, Clone)]
pub struct FlowSnapshot {
    /// Live bindings for each place
    place_states: IndexVec<ScopedPlaceId, PlaceBindings>,
    /// Current reachability
    reachability: ScopedReachabilityConstraintId,
}

pub struct UseDefMapBuilder {
    /// All definitions in the scope
    all_definitions: IndexVec<DefinitionId, Definition>,

    /// Current flow state
    place_states: IndexVec<ScopedPlaceId, PlaceBindings>,

    /// Bindings at each use site
    bindings_by_use: IndexVec<UseId, Vec<LiveBinding>>,

    /// Current reachability
    reachability: ScopedReachabilityConstraintId,
}

impl UseDefMapBuilder {
    pub fn snapshot(&self) -> FlowSnapshot {
        FlowSnapshot {
            place_states: self.place_states.clone(),
            reachability: self.reachability,
        }
    }

    pub fn restore(&mut self, snapshot: FlowSnapshot) {
        self.place_states = snapshot.place_states;
        self.reachability = snapshot.reachability;
    }

    pub fn merge(&mut self, snapshot: FlowSnapshot) {
        // Merge each place's bindings
        for (place_id, other_state) in snapshot.place_states.into_iter_enumerated() {
            self.place_states[place_id].merge(other_state, &mut self.constraints);
        }

        // Merge reachability
        self.reachability = self.constraints.or_reachability(
            self.reachability,
            snapshot.reachability
        );
    }

    pub fn record_use(&mut self, use_id: UseId, place_id: ScopedPlaceId) {
        // Record all currently live bindings for this use
        let live_bindings = self.place_states[place_id].live_bindings.clone();
        self.bindings_by_use[use_id] = live_bindings;
    }
}
```

#### 8. Example: Type Narrowing

```rust
// Example showing how flow sensitivity improves type inference
fn example(x: Option<i32>) {
    // x has binding B1: Option<i32>

    if x.is_some() {
        // Narrowing constraint applied to B1
        // x still has binding B1, but with constraint "is Some"
        let val = x.unwrap(); // Safe due to narrowing
    } else {
        // Narrowing constraint "is None" applied to B1
        // Type inference knows x is None here
    }

    // After merge: B1 has no constraints (intersection of opposites)
    // Type is back to Option<i32>
}

// Multiple bindings example
fn multi_binding(flag: bool) {
    let x: Option<i32>;

    if flag {
        x = Some(1);  // Binding B1
    } else {
        x = None;     // Binding B2
    }

    // After merge: x has bindings [B1, B2]
    // Type inference sees both possibilities

    if x.is_some() {
        // Narrowing eliminates B2 (None binding)
        // Only B1 remains, safe to unwrap
    }
}
```

This refactoring enables:

- Accurate type inference across control flow
- Proper handling of conditional bindings
- Type narrowing for pattern matching and guards
- Support for flow-sensitive null/None checking

## Issue 6: Add Cross-Module Import Resolution

### What

Implement proper cross-module import resolution inspired by ruff's approach to
tracking imported modules, resolving cross-module references, and handling
re-exports.

### Why

The current implementation has basic import tracking but lacks:

- Efficient resolution of imported symbols
- Tracking of which modules are imported
- Support for re-exports and wildcards
- Integration with the module resolver

Ruff's approach provides:

- Accurate tracking of imported modules and their ancestors
- Proper handling of wildcard imports (`from module import *`)
- Support for re-exports and `__all__` detection
- Flow-sensitive import visibility

### How

#### 1. Current Cairo-M Import Handling

```rust
// Current simple import tracking
impl SemanticIndexBuilder {
    fn handle_import(&mut self, import: &ImportStatement) {
        // Basic import registration
        let def = Definition::Import {
            module: import.module.clone(),
            name: import.name.clone()
        };
        self.add_definition(import.name, def);
    }
}
```

#### 2. Ruff's Import Module Tracking

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index.rs
/// Returns the set of modules that are imported anywhere in `file`.
///
/// This set only considers `import` statements, not `from...import` statements, because:
///   - In `from foo import bar`, we cannot determine whether `foo.bar` is a submodule
///   - We cannot resolve relative imports without knowing the current module name
#[salsa::tracked(returns(deref))]
pub(crate) fn imported_modules<'db>(db: &'db dyn Db, file: File) -> Arc<FxHashSet<ModuleName>> {
    semantic_index(db, file).imported_modules.clone()
}

pub(crate) struct SemanticIndex<'db> {
    // ... other fields
    /// The set of modules that are imported anywhere within this file.
    imported_modules: Arc<FxHashSet<ModuleName>>,
}
```

#### 3. Ruff's Import Statement Processing

```rust
// From builder.rs - handling import statements
impl<'ast> SemanticIndexBuilder<'_, 'ast> {
    fn visit_stmt_import(&mut self, node: &'ast ast::StmtImport) {
        self.current_use_def_map_mut()
            .record_node_reachability(NodeKey::from_node(node));

        for (alias_index, alias) in node.names.iter().enumerate() {
            // Mark the imported module, and all of its parents, as being imported
            if let Some(module_name) = ModuleName::new(&alias.name) {
                self.imported_modules.extend(module_name.ancestors());
            }

            let (symbol_name, is_reexported) = if let Some(asname) = &alias.asname {
                (asname.id.clone(), asname.id == alias.name.id)
            } else {
                (Name::new(alias.name.id.split('.').next().unwrap()), false)
            };

            let symbol = self.add_symbol(symbol_name);
            self.add_definition(
                symbol,
                ImportDefinitionNodeRef {
                    node,
                    alias_index,
                    is_reexported,
                },
            );
        }
    }
}
```

#### 4. Ruff's Import Definition Structure

```rust
// From definition.rs - import definition tracking
#[derive(Copy, Clone, Debug)]
pub(crate) struct ImportDefinitionNodeRef<'ast> {
    pub(crate) node: &'ast ast::StmtImport,
    pub(crate) alias_index: usize,
    pub(crate) is_reexported: bool,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ImportFromDefinitionNodeRef<'ast> {
    pub(crate) node: &'ast ast::StmtImportFrom,
    pub(crate) alias_index: usize,
    pub(crate) is_reexported: bool,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct StarImportDefinitionNodeRef<'ast> {
    pub(crate) node: &'ast ast::StmtImportFrom,
    pub(crate) place_id: ScopedPlaceId,
}
```

#### 5. Ruff's Wildcard Import Handling

```rust
// From builder.rs - handling from...import *
if &alias.name == "*" {
    // Wildcard imports are invalid syntax everywhere except the top-level scope
    if !self.in_module_scope() {
        continue;
    }

    let Ok(module_name) =
        ModuleName::from_import_statement(self.db, self.file, node)
    else {
        continue;
    };

    let Some(module) = resolve_module(self.db, &module_name) else {
        continue;
    };

    let Some(referenced_module) = module.file() else {
        continue;
    };

    // Import all exported names from the module
    for export in exported_names(self.db, referenced_module) {
        let symbol_id = self.add_symbol(export.clone());
        let node_ref = StarImportDefinitionNodeRef {
            node,
            place_id: symbol_id,
        };

        // Create a predicate for flow-sensitive visibility
        let star_import = StarImportPlaceholderPredicate::new(
            self.db,
            self.file,
            symbol_id,
            referenced_module,
        );

        let star_import_predicate = self.add_predicate(star_import.into());

        // The definition is only reachable if the export is reachable
        // in the source module
        self.push_additional_definition(symbol_id, node_ref);
    }
}
```

#### 6. Ruff's Export Detection

```rust
// From re_exports.rs - detecting module exports
#[salsa::tracked(returns(deref))]
pub(super) fn exported_names(db: &dyn Db, file: File) -> Box<[Name]> {
    let module = parsed_module(db, file).load(db);
    let mut finder = ExportFinder::new(db, file);
    finder.visit_body(module.suite());
    finder.resolve_exports()
}

struct ExportFinder<'db> {
    db: &'db dyn Db,
    file: File,
    visiting_stub_file: bool,
    exports: FxHashMap<&'db Name, PossibleExportKind>,
    dunder_all: DunderAll,
}

impl<'db> ExportFinder<'db> {
    fn resolve_exports(self) -> Box<[Name]> {
        match self.dunder_all {
            DunderAll::NotPresent => self
                .exports
                .into_iter()
                .filter_map(|(name, kind)| {
                    // Don't export private names (starting with _)
                    if name.starts_with('_') {
                        return None;
                    }
                    Some(name.clone())
                })
                .collect(),
            DunderAll::Present => {
                // If __all__ is present, only export names in __all__
                self.exports.into_keys().cloned().collect()
            }
        }
    }
}
```

#### 7. Adapt Import Resolution for Cairo-M

```rust
// New import_resolution.rs for Cairo-M
use rustc_hash::FxHashSet;
use std::sync::Arc;

/// Track imported modules for a file
#[derive(Debug, Default)]
pub struct ImportTracker {
    /// All modules imported via `use` statements
    imported_modules: FxHashSet<ModulePath>,
    /// All symbols imported from other modules
    imported_symbols: FxHashMap<String, ImportedSymbol>,
}

#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    pub module: ModulePath,
    pub original_name: String,
    pub local_name: String,
    pub is_reexported: bool,
    pub is_wildcard: bool,
}

impl ImportTracker {
    /// Process a use statement: `use module::path`
    pub fn add_module_import(&mut self, module: ModulePath) {
        // Add the module and all its ancestors
        let mut current = module.clone();
        while !current.is_empty() {
            self.imported_modules.insert(current.clone());
            current = current.parent();
        }
    }

    /// Process a symbol import: `use module::path::{symbol1, symbol2}`
    pub fn add_symbol_import(
        &mut self,
        module: ModulePath,
        imports: Vec<(String, Option<String>)>
    ) {
        for (original, alias) in imports {
            let local_name = alias.unwrap_or_else(|| original.clone());
            let is_reexported = local_name == original;

            self.imported_symbols.insert(local_name.clone(), ImportedSymbol {
                module: module.clone(),
                original_name: original,
                local_name,
                is_reexported,
                is_wildcard: false,
            });
        }
    }

    /// Process a wildcard import: `use module::path::*`
    pub fn add_wildcard_import(&mut self, module: ModulePath, exports: Vec<String>) {
        for export in exports {
            self.imported_symbols.insert(export.clone(), ImportedSymbol {
                module: module.clone(),
                original_name: export.clone(),
                local_name: export,
                is_reexported: true,
                is_wildcard: true,
            });
        }
    }
}
```

#### 8. Export Detection for Cairo-M

```rust
// New exports.rs for Cairo-M
/// Detect exported symbols from a module
pub fn exported_symbols(db: &dyn Db, module: ModuleId) -> Arc<Vec<String>> {
    let semantic_index = semantic_index(db, module);
    let mut exports = Vec::new();

    // In Cairo-M, exports are determined by:
    // 1. pub declarations at module level
    // 2. pub use statements (re-exports)

    for (name, place) in semantic_index.module_scope_symbols() {
        if place.is_public() && !name.starts_with('_') {
            exports.push(name.clone());
        }
    }

    // Check for explicit re-exports
    for import in &semantic_index.imports {
        if import.is_public && import.is_reexported {
            exports.push(import.local_name.clone());
        }
    }

    Arc::new(exports)
}
```

#### 9. Integration with SemanticIndexBuilder

```rust
// Updated semantic_index/builder.rs
impl<'db> SemanticIndexBuilder<'db> {
    fn visit_use_statement(&mut self, use_stmt: &UseStatement) {
        match &use_stmt.kind {
            UseKind::Module(path) => {
                // Track module import
                self.import_tracker.add_module_import(path.clone());

                // Create import definition
                let symbol = self.add_symbol(path.last_segment());
                self.add_definition(symbol, Definition::ModuleImport {
                    path: path.clone(),
                    is_reexported: use_stmt.is_pub,
                });
            }
            UseKind::Symbols { module, symbols } => {
                // Track symbol imports
                let imports: Vec<_> = symbols.iter()
                    .map(|s| (s.name.clone(), s.alias.clone()))
                    .collect();

                self.import_tracker.add_symbol_import(module.clone(), imports);

                // Create definitions for each imported symbol
                for symbol in symbols {
                    let local_name = symbol.alias.as_ref()
                        .unwrap_or(&symbol.name);
                    let symbol_id = self.add_symbol(local_name.clone());

                    self.add_definition(symbol_id, Definition::SymbolImport {
                        module: module.clone(),
                        original: symbol.name.clone(),
                        is_reexported: use_stmt.is_pub,
                    });
                }
            }
            UseKind::Wildcard(module) => {
                // Resolve module and get exports
                if let Some(exports) = self.resolve_module_exports(module) {
                    self.import_tracker.add_wildcard_import(
                        module.clone(),
                        exports.clone()
                    );

                    // Create definitions for each export
                    for export in exports {
                        let symbol_id = self.add_symbol(export.clone());
                        self.add_definition(symbol_id, Definition::WildcardImport {
                            module: module.clone(),
                            name: export,
                            is_reexported: use_stmt.is_pub,
                        });
                    }
                }
            }
        }
    }
}
```

This refactoring enables:

- Accurate tracking of module dependencies
- Proper wildcard import resolution
- Support for re-exports and visibility
- Foundation for cross-module type inference
- Efficient symbol resolution across modules

## Issue 7: Implement Comprehensive Error Recovery

### What

Enhance error recovery in semantic analysis to continue processing after
encountering errors, providing more complete diagnostics and enabling better IDE
support.

### Why

The current implementation may stop or panic on certain errors, limiting:

- Multiple error reporting
- IDE responsiveness
- Partial semantic information

Ruff's approach:

- Continues analysis after errors
- Provides partial results
- Collects all errors for comprehensive reporting
- Enables better IDE experience

### How

#### 1. Current Cairo-M Error Handling

```rust
// Current approach - may panic or stop on errors
impl SemanticIndexBuilder {
    fn handle_error(&mut self, error: SemanticError) {
        // Simply logs or panics
        panic!("Semantic error: {:?}", error);
    }

    fn visit_invalid_node(&mut self, node: &InvalidNode) {
        // Stops processing
        return;
    }
}
```

#### 2. Ruff's Error Collection System

```rust
// From ruff: crates/ty_python_semantic/src/semantic_index/builder.rs
pub struct SemanticIndexBuilder<'db, 'ast> {
    // ... other fields

    /// Errors collected by the `semantic_checker`.
    semantic_syntax_errors: RefCell<Vec<SemanticSyntaxError>>,
}

impl<'db, 'ast> SemanticIndexBuilder<'db, 'ast> {
    pub(super) fn build(self) -> SemanticIndex {
        // ... build process

        SemanticIndex {
            // ... other fields

            // Errors are collected and returned as part of the index
            semantic_syntax_errors: self.semantic_syntax_errors.into_inner(),
        }
    }
}
```

#### 3. Ruff's Error Reporting with Context

```rust
// From ruff: semantic_errors.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSyntaxError {
    pub kind: SemanticSyntaxErrorKind,
    pub range: TextRange,
    pub python_version: PythonVersion,
}

// From builder.rs - error reporting implementation
impl SemanticSyntaxContext for SemanticIndexBuilder<'_, '_> {
    fn report_semantic_error(&self, error: SemanticSyntaxError) {
        // Only report if this file should be checked
        if self.db.should_check_file(self.file) {
            // Collect error instead of panicking
            self.semantic_syntax_errors.borrow_mut().push(error);
        }
    }
}

// Example error reporting with context
self.report_semantic_error(SemanticSyntaxError {
    kind: SemanticSyntaxErrorKind::AnnotatedGlobal(name.id.as_str().into()),
    range: name.range,
    python_version: self.python_version,
});

self.report_semantic_error(SemanticSyntaxError {
    kind: SemanticSyntaxErrorKind::AnnotatedNonlocal(
        name.id.as_str().into(),
    ),
    range: name.range,
    python_version: self.python_version,
});
```

#### 4. Ruff's Semantic Error Checker Integration

```rust
// From builder.rs - integrating semantic checker
impl<'ast> Visitor<'ast> for SemanticIndexBuilder<'_, 'ast> {
    fn visit_stmt(&mut self, stmt: &'ast ast::Stmt) {
        // Check for semantic errors before processing
        self.with_semantic_checker(|semantic, context| {
            semantic.visit_stmt(stmt, context)
        });

        // Continue processing even if errors were found
        match stmt {
            ast::Stmt::FunctionDef(function_def) => {
                // Process function definition...
            }
            // ... other cases
        }
    }
}

impl<'db, 'ast> SemanticIndexBuilder<'db, 'ast> {
    fn with_semantic_checker(&mut self, f: impl FnOnce(&mut SemanticSyntaxChecker, &Self)) {
        let mut checker = std::mem::take(&mut self.semantic_checker);
        f(&mut checker, self);
        self.semantic_checker = checker;
    }
}
```

#### 5. Ruff's Error Types and Recovery

```rust
// From semantic_errors.rs - comprehensive error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticSyntaxErrorKind {
    LateFutureImport,
    LoadBeforeGlobalDeclaration { name: String, start: TextSize },
    YieldOutsideFunction(YieldOutsideFunctionKind),
    ReturnOutsideFunction,
    AwaitOutsideAsyncFunction(AwaitOutsideAsyncFunctionKind),
    ReboundComprehensionVariable,
    DuplicateTypeParameter,
    MultipleCaseAssignment(String),
    IrrefutableCasePattern(IrrefutablePatternKind),
    SingleStarredAssignment,
    WriteToDebug(String),
    InvalidExpression(String),
    DuplicateMatchKey(String),
    DuplicateMatchClassAttribute(String),
    AnnotatedGlobal(String),
    AnnotatedNonlocal(String),
    // ... more error types
}

// Error checking continues after finding errors
impl SemanticSyntaxChecker {
    fn check_stmt<Ctx: SemanticSyntaxContext>(&mut self, stmt: &ast::Stmt, ctx: &Ctx) {
        match stmt {
            Stmt::ImportFrom(import_from) => {
                if self.seen_futures_boundary && is_future_import(import_from) {
                    Self::add_error(ctx, SemanticSyntaxErrorKind::LateFutureImport, *range);
                }
                // Continue checking other aspects
            }
            Stmt::Match(match_stmt) => {
                Self::irrefutable_match_case(match_stmt, ctx);
                // Continue checking patterns
                for case in &match_stmt.cases {
                    let mut visitor = MatchPatternVisitor {
                        names: FxHashSet::default(),
                        ctx,
                    };
                    visitor.visit_pattern(&case.pattern);
                }
            }
            // ... continue with other statements
        }
    }
}
```

#### 6. Adapt Error Recovery for Cairo-M

```rust
// New error_recovery.rs for Cairo-M
use std::cell::RefCell;

/// Semantic error with context
#[derive(Debug, Clone)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
    pub context: ErrorContext,
}

#[derive(Debug, Clone)]
pub enum SemanticErrorKind {
    UndefinedVariable(String),
    TypeMismatch { expected: String, found: String },
    InvalidAssignment(String),
    UseBeforeDeclaration(String),
    DuplicateDefinition(String),
    InvalidImport(String),
    // ... more error types
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file: FileId,
    pub scope: FileScopeId,
    pub in_function: Option<String>,
}

/// Error collector for semantic analysis
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: RefCell<Vec<SemanticError>>,
}

impl ErrorCollector {
    pub fn report_error(&self, error: SemanticError) {
        self.errors.borrow_mut().push(error);
    }

    pub fn into_errors(self) -> Vec<SemanticError> {
        self.errors.into_inner()
    }
}
```

#### 7. Builder Integration with Error Recovery

```rust
// Updated semantic_index/builder.rs
pub struct SemanticIndexBuilder<'db> {
    // ... existing fields

    /// Error collector
    error_collector: ErrorCollector,
}

impl<'db> SemanticIndexBuilder<'db> {
    fn visit_statement(&mut self, stmt: &Statement) {
        // Check for semantic errors
        self.check_semantic_errors(stmt);

        // Continue processing regardless of errors
        match stmt {
            Statement::Let { pattern, value, .. } => {
                // Try to process, collecting errors
                if let Err(e) = self.process_let_binding(pattern, value) {
                    self.error_collector.report_error(e);
                    // Use default/partial information
                    self.add_undefined_place(pattern);
                }
            }
            Statement::Expression(expr) => {
                // Continue even if expression is invalid
                if let Err(e) = self.visit_expression(expr) {
                    self.error_collector.report_error(e);
                }
            }
            // ... other cases
        }
    }

    fn process_let_binding(
        &mut self,
        pattern: &Pattern,
        value: &Expression
    ) -> Result<(), SemanticError> {
        // Process with error recovery
        let value_type = self.infer_expression_type(value)
            .unwrap_or_else(|e| {
                self.error_collector.report_error(e);
                Type::Unknown // Use unknown type as fallback
            });

        // Continue processing pattern
        self.bind_pattern(pattern, value_type)?;
        Ok(())
    }

    fn check_semantic_errors(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check for invalid assignments
                if self.is_constant(target) {
                    self.error_collector.report_error(SemanticError {
                        kind: SemanticErrorKind::InvalidAssignment(
                            "Cannot assign to constant".to_string()
                        ),
                        span: target.span(),
                        context: self.current_error_context(),
                    });
                }
            }
            // ... other checks
        }
    }
}
```

#### 8. Error Recovery Strategies

```rust
// Recovery strategies for different error scenarios
impl<'db> SemanticIndexBuilder<'db> {
    /// Create placeholder for undefined symbols
    fn add_undefined_place(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name) => {
                let place = PlaceExpr::name(name.clone());
                let place_id = self.add_place(place, PlaceFlags::empty());

                // Mark as undefined but continue
                self.add_definition(place_id, Definition::Undefined {
                    name: name.clone(),
                    attempted_type: Type::Unknown,
                });
            }
            Pattern::Tuple(patterns) => {
                // Recursively handle tuple patterns
                for p in patterns {
                    self.add_undefined_place(p);
                }
            }
        }
    }

    /// Continue with partial type information
    fn infer_expression_type(&mut self, expr: &Expression) -> Result<Type, SemanticError> {
        match expr {
            Expression::Invalid => {
                // Return unknown type for invalid expressions
                Ok(Type::Unknown)
            }
            Expression::Identifier(name) => {
                // Try to resolve, return Unknown if not found
                self.resolve_identifier(name)
                    .or_else(|e| {
                        self.error_collector.report_error(e);
                        Ok(Type::Unknown)
                    })
            }
            // ... other cases
        }
    }
}
```

#### 9. Testing Error Recovery

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continues_after_undefined_variable() {
        let source = r#"
            let x = undefined_var;  // Error here
            let y = 42;             // Should still process this
        "#;

        let index = build_semantic_index(source);

        // Should have collected the error
        assert_eq!(index.errors.len(), 1);
        assert!(matches!(
            index.errors[0].kind,
            SemanticErrorKind::UndefinedVariable(_)
        ));

        // But should have processed y
        assert!(index.has_definition("y"));
    }

    #[test]
    fn test_multiple_errors_collected() {
        let source = r#"
            let x = undefined1;
            let y = undefined2;
            const C = 1;
            C = 2;  // Invalid assignment
        "#;

        let index = build_semantic_index(source);

        // Should collect all errors
        assert_eq!(index.errors.len(), 3);
    }
}
```

This refactoring enables:

- Comprehensive error collection without stopping analysis
- Better IDE support with partial semantic information
- Multiple error reporting in a single pass
- Graceful degradation for invalid code
- Rich error context for better diagnostics

## Implementation Order

1. **Phase 1 - Foundation** (Issues 1, 4):

   - Refactor place management system
   - Optimize data structures
   - Ensure all existing tests pass

2. **Phase 2 - Core Improvements** (Issues 2, 3):

   - Implement visitor pattern
   - Enhance scope management
   - Add reachability tracking

3. **Phase 3 - Advanced Features** (Issues 5, 6, 7):
   - Improve use-def analysis
   - Add cross-module resolution
   - Implement error recovery

## Testing Strategy

Each issue should include:

1. Unit tests for new components
2. Integration tests with existing code
3. Snapshot tests for semantic analysis output
4. Performance benchmarks where applicable
5. Error case testing

## Migration Strategy

1. Implement new components alongside existing ones
2. Add feature flags for gradual migration
3. Ensure backward compatibility during transition
4. Remove old code only after full validation

## Success Metrics

- All existing tests pass
- Improved performance (measure with benchmarks)
- Better error messages and recovery
- Cleaner, more maintainable code
- Enhanced IDE support capabilities
