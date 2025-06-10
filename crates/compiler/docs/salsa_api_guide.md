# Salsa API Guide: From Ruff's Top-Tier Usage

This guide analyzes Ruff's sophisticated Salsa usage patterns to provide a
comprehensive reference for building incremental compilation systems with Salsa.

## Table of Contents

1. [Database Architecture](#database-architecture)
2. [Salsa Macros](#salsa-macros)
3. [Storage and Lifetime Management](#storage-and-lifetime-management)
4. [Advanced Features](#advanced-features)
5. [Best Practices](#best-practices)
6. [Implementation Templates](#implementation-templates)

## Database Architecture

### 1. Hierarchical Database Traits

Ruff uses a layered approach to database traits, creating a hierarchy of
capabilities:

```rust
use salsa;

/// Base database trait - foundational layer
#[salsa::db]
pub trait SourceDb: salsa::Database {
    fn system(&self) -> &dyn System;
    fn files(&self) -> &Files;
    fn vendored(&self) -> &VendoredFileSystem;
    fn python_version(&self) -> PythonVersion;
}

/// Semantic analysis layer - builds on SourceDb
#[salsa::db]
pub trait SemanticDb: SourceDb + Upcast<dyn SourceDb> {
    fn is_file_open(&self, file: File) -> bool;
    fn rule_selection(&self) -> &RuleSelection;
    fn lint_registry(&self) -> &LintRegistry;
}

/// IDE layer - builds on SemanticDb
#[salsa::db]
pub trait IdeDb: SemanticDb + Upcast<dyn SemanticDb> + Upcast<dyn SourceDb> {}

/// Project layer - top-level capabilities
#[salsa::db]
pub trait ProjectDb: SemanticDb + Upcast<dyn SemanticDb> {
    fn project(&self) -> Project;
}
```

### 2. Upcast Trait Pattern

Ruff implements an `Upcast` trait for safe database upcasting:

```rust
/// Trait for upcasting a reference to a base trait object.
pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
    fn upcast_mut(&mut self) -> &mut T;
}

// Implementation example
impl Upcast<dyn SourceDb> for ProjectDatabase {
    fn upcast(&self) -> &(dyn SourceDb + 'static) {
        self
    }
    fn upcast_mut(&mut self) -> &mut (dyn SourceDb + 'static) {
        self
    }
}
```

### 3. Database Implementation Structure

```rust
#[salsa::db]
#[derive(Clone)]
pub struct ProjectDatabase {
    // Application-specific state
    project: Option<Project>,
    files: Files,
    system: Arc<dyn System + Send + Sync + RefUnwindSafe>,

    // IMPORTANT: Storage must be last field for proper drop order
    // This ensures other Arc fields can be mutably borrowed after zalsa_mut()
    storage: salsa::Storage<ProjectDatabase>,
}

impl ProjectDatabase {
    pub fn new<S>(system: S) -> anyhow::Result<Self>
    where
        S: System + 'static + Send + Sync + RefUnwindSafe,
    {
        let mut db = Self {
            project: None,
            storage: salsa::Storage::new(
                // Optional event handler for debugging/tracing
                if tracing::enabled!(tracing::Level::TRACE) {
                    Some(Box::new(move |event: salsa::Event| {
                        if matches!(event.kind, salsa::EventKind::WillCheckCancellation) {
                            return;
                        }
                        tracing::trace!("Salsa event: {event:?}");
                    }))
                } else {
                    None
                }
            ),
            files: Files::default(),
            system: Arc::new(system),
        };

        // Initialize singletons and perform setup
        Ok(db)
    }
}

// Implement all database traits
#[salsa::db]
impl salsa::Database for ProjectDatabase {}

#[salsa::db]
impl SourceDb for ProjectDatabase {
    fn system(&self) -> &dyn System {
        &*self.system
    }
    // ... other methods
}

#[salsa::db]
impl SemanticDb for ProjectDatabase {
    fn is_file_open(&self, file: File) -> bool {
        self.project().is_file_open(self, file)
    }
    // ... other methods
}
```

## Salsa Macros

### 1. `#[salsa::input]` - External Data

Use for data that comes from outside the system:

```rust
/// File in the system - core input type
#[salsa::input]
#[derive(PartialOrd, Ord)]
pub struct File {
    /// Immutable path
    #[returns(ref)]
    pub path: FilePath,

    /// Mutable fields with defaults
    #[default]
    pub revision: FileRevision,

    #[default]
    pub permissions: Option<u32>,

    #[default]
    status: FileStatus,
}

/// Source program text
#[salsa::input(debug)]
pub struct SourceProgram {
    #[returns(ref)]
    pub text: String,
}
```

**Key patterns:**

- Use `#[returns(ref)]` for large/borrowed data
- Use `#[default]` for fields that can change over time
- Add `debug` parameter for better debugging experience
- Keep inputs minimal and focused

### 2. `#[salsa::tracked]` - Computed Values

Use for derived computations:

```rust
/// Parse source code into AST
#[salsa::tracked(returns(ref), no_eq)]
pub fn parsed_module(db: &dyn Db, file: File) -> ParsedModule {
    let source = source_text(db, file);
    let ty = file.source_type(db);

    let parsed = parse_unchecked(&source, ParseOptions::from(ty))
        .try_into_module()
        .expect("PySourceType always parses into a module");

    ParsedModule::new(parsed)
}

/// Semantic analysis for a scope
#[salsa::tracked(returns(ref), cycle_fn=scope_cycle_recover, cycle_initial=scope_cycle_initial)]
pub(crate) fn infer_scope_types<'db>(
    db: &'db dyn Db,
    scope: ScopeId<'db>
) -> TypeInference<'db> {
    // Implementation
}

// Cycle recovery functions
fn scope_cycle_recover<'db>(
    _db: &'db dyn Db,
    _value: &TypeInference<'db>,
    _count: u32,
    _scope: ScopeId<'db>,
) -> salsa::CycleRecoveryAction<TypeInference<'db>> {
    salsa::CycleRecoveryAction::Iterate
}

fn scope_cycle_initial<'db>(_db: &'db dyn Db, scope: ScopeId<'db>) -> TypeInference<'db> {
    TypeInference::cycle_fallback(scope, Type::Never)
}
```

**Key parameters:**

- `returns(ref)` - Return by reference for large data
- `returns(as_ref)` - Return `Option` by reference
- `returns(deref)` - Return `Arc<T>` as `T`
- `no_eq` - Skip equality comparison (useful for ASTs)
- `cycle_fn` / `cycle_initial` - Handle cycles with fixed-point iteration

### 3. `#[salsa::interned]` - Deduplicated Values

Use for values that benefit from structural sharing:

```rust
/// Class literal - interned for identity semantics
#[salsa::interned(debug)]
#[derive(PartialOrd, Ord)]
pub struct ClassLiteral<'db> {
    /// Name of the class at definition
    #[returns(ref)]
    pub(crate) name: ast::name::Name,

    pub(crate) body_scope: ScopeId<'db>,
    pub(crate) known: Option<KnownClass>,
    pub(crate) dataclass_params: Option<DataclassParams>,
}

/// String literal type - interned for memory efficiency
#[salsa::interned(debug)]
#[derive(PartialOrd, Ord)]
pub struct StringLiteralType<'db> {
    #[returns(deref)]
    value: Box<str>,
}

/// Module name wrapper for Salsa
#[salsa::interned(debug)]
struct ModuleNameIngredient<'db> {
    #[returns(ref)]
    pub(super) name: ModuleName,
}
```

**Key patterns:**

- Implement `PartialOrd, Ord` for deterministic ordering
- Use for types that need identity semantics
- Great for AST node types and commonly used values
- Use `#[returns(deref)]` for boxed values

## Storage and Lifetime Management

### 1. Storage Field Ordering

```rust
#[salsa::db]
#[derive(Clone)]
pub struct Database {
    // Application state first
    files: Files,
    system: Arc<dyn System + Send + Sync + RefUnwindSafe>,

    // Storage MUST be last field!
    // This ensures proper drop order for mutable borrowing tricks
    storage: salsa::Storage<Database>,
}
```

### 2. Mutable Access Pattern

```rust
impl Database {
    /// Get mutable system reference
    /// WARNING: Triggers new revision, canceling other handles
    pub fn system_mut(&mut self) -> &mut dyn System {
        // Cancel all other database references
        let _ = self.zalsa_mut();

        // Now safe to get mutable reference to Arc contents
        Arc::get_mut(&mut self.system)
            .expect("ref count should be 1 after zalsa_mut")
    }
}
```

### 3. Lifetime Management

Database lifetimes are tied to ingredient lifetimes:

```rust
// Good: Lifetime tied to database
pub fn semantic_index<'db>(db: &'db dyn Db, file: File) -> SemanticIndex<'db> {
    // Implementation
}

// Good: Scoped queries with explicit lifetimes
#[salsa::tracked]
pub(crate) fn infer_scope_types<'db>(
    db: &'db dyn Db,
    scope: ScopeId<'db>
) -> TypeInference<'db> {
    // Implementation
}
```

### 4. Event Handling

```rust
impl Database {
    pub fn new() -> Self {
        let events = Arc<Mutex<Vec<salsa::Event>>>::default();
        Self {
            storage: salsa::Storage::new(Some(Box::new({
                let events = events.clone();
                move |event| {
                    // Filter out noise
                    if matches!(event.kind, salsa::EventKind::WillCheckCancellation) {
                        return;
                    }

                    tracing::trace!("Salsa event: {event:?}");
                    let mut events = events.lock().unwrap();
                    events.push(event);
                }
            }))),
            // other fields...
        }
    }
}
```

## Advanced Features

### 1. Cycle Recovery with Fixed-Point Iteration

Ruff extensively uses cycle recovery for type inference:

```rust
#[salsa::tracked(
    returns(ref),
    cycle_fn=infer_cycle_recover,
    cycle_initial=infer_cycle_initial
)]
pub(crate) fn infer_types<'db>(
    db: &'db dyn Db,
    scope: ScopeId<'db>
) -> TypeInference<'db> {
    // Type inference implementation
    // May reference other types creating cycles
}

fn infer_cycle_recover<'db>(
    _db: &'db dyn Db,
    _value: &TypeInference<'db>,
    _count: u32,
    _scope: ScopeId<'db>,
) -> salsa::CycleRecoveryAction<TypeInference<'db>> {
    // Continue iteration until fixed point
    salsa::CycleRecoveryAction::Iterate
}

fn infer_cycle_initial<'db>(
    _db: &'db dyn Db,
    _scope: ScopeId<'db>
) -> TypeInference<'db> {
    // Start with pessimistic assumption
    TypeInference::cycle_fallback(scope, Type::Never)
}
```

**Cycle Recovery Patterns:**

- **Iterate**: Continue until fixed-point (most common)
- **Fallback**: Use a specific value and stop
- Use pessimistic initial values (`Never`, `Unknown`, empty collections)

### 2. Durability Management

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileRootKind {
    /// Changes frequently
    Project,
    /// Rarely changes
    LibrarySearchPath,
}

impl FileRootKind {
    const fn durability(self) -> salsa::Durability {
        match self {
            FileRootKind::Project => salsa::Durability::LOW,
            FileRootKind::LibrarySearchPath => salsa::Durability::HIGH,
        }
    }
}

impl FileRoot {
    pub fn durability(self, db: &dyn Db) -> salsa::Durability {
        self.kind_at_time_of_creation(db).durability()
    }
}
```

### 3. Cross-File Dependencies

Minimize cross-file dependencies:

```rust
// BAD: Direct AST dependency across files
pub fn analyze_function(db: &dyn Db, func: &ast::FunctionDef) -> Analysis {
    // This creates cross-file AST dependencies
}

// GOOD: Use semantic ingredients instead
#[salsa::tracked]
pub fn analyze_function_by_id(db: &dyn Db, func_id: FunctionId) -> Analysis {
    // Semantic ID isolates from AST changes
}
```

### 4. Fine-Grained Invalidation

```rust
/// Fine-grained place table access
#[salsa::tracked(returns(deref))]
pub(crate) fn place_table<'db>(
    db: &'db dyn Db,
    scope: ScopeId<'db>
) -> Arc<PlaceTable> {
    let file = scope.file(db);
    let index = semantic_index(db, file);
    index.place_table(scope.file_scope_id(db))
}

/// Fine-grained use-def access
#[salsa::tracked(returns(deref))]
pub(crate) fn use_def_map<'db>(
    db: &'db dyn Db,
    scope: ScopeId<'db>
) -> Arc<UseDefMap<'db>> {
    let file = scope.file(db);
    let index = semantic_index(db, file);
    index.use_def_map(scope.file_scope_id(db))
}
```

## Best Practices

### 1. Database Design

```rust
// ✅ DO: Layer database traits hierarchically
#[salsa::db]
pub trait BaseDb: salsa::Database { /* base capabilities */ }

#[salsa::db]
pub trait AnalysisDb: BaseDb + Upcast<dyn BaseDb> { /* analysis */ }

// ✅ DO: Use Upcast for safe trait upcasting
impl Upcast<dyn BaseDb> for MyDb {
    fn upcast(&self) -> &(dyn BaseDb + 'static) { self }
    fn upcast_mut(&mut self) -> &mut (dyn BaseDb + 'static) { self }
}

// ❌ DON'T: Put too many capabilities in one trait
#[salsa::db]
pub trait MonolithicDb: salsa::Database {
    // Too many concerns in one trait
}
```

### 2. Input Design

```rust
// ✅ DO: Keep inputs minimal and focused
#[salsa::input]
pub struct SourceFile {
    #[returns(ref)]
    pub path: PathBuf,

    #[default]
    pub content_revision: Revision,
}

// ❌ DON'T: Put computed data in inputs
#[salsa::input]
pub struct BadSourceFile {
    pub path: PathBuf,
    pub parsed_ast: ParsedAst, // This should be tracked!
}
```

### 3. Tracked Function Guidelines

```rust
// ✅ DO: Use appropriate return annotations
#[salsa::tracked(returns(ref))]     // For large data structures
#[salsa::tracked(returns(deref))]   // For Arc<T> -> T
#[salsa::tracked(no_eq)]            // For non-comparable data (ASTs)

// ✅ DO: Handle cycles when they can occur
#[salsa::tracked(cycle_fn=my_cycle_recover, cycle_initial=my_cycle_initial)]
pub fn may_have_cycles(db: &dyn Db, input: Input) -> Output {
    // Implementation that might create cycles
}

// ✅ DO: Use descriptive names for cycle handlers
fn my_cycle_recover(/*...*/) -> salsa::CycleRecoveryAction<Output> {
    salsa::CycleRecoveryAction::Iterate
}
```

### 4. Performance Considerations

```rust
// ✅ DO: Use fine-grained queries
#[salsa::tracked(returns(deref))]
pub fn symbol_table(db: &dyn Db, scope: ScopeId) -> Arc<SymbolTable> {
    // Only invalidates when this specific scope changes
}

// ❌ DON'T: Create coarse-grained queries
#[salsa::tracked]
pub fn analyze_entire_project(db: &dyn Db) -> ProjectAnalysis {
    // Invalidates for any project change
}

// ✅ DO: Use interning for frequently created values
#[salsa::interned]
pub struct TypeId<'db> {
    #[returns(ref)]
    pub name: String,
    pub kind: TypeKind,
}

// ✅ DO: Minimize cross-file dependencies
pub fn analyze_local_scope(db: &dyn Db, scope: LocalScopeId) -> Analysis {
    // Isolated to single file
}
```

## Implementation Templates

### 1. Basic Database Setup

```rust
use salsa;
use std::sync::Arc;

// Define your database trait
#[salsa::db]
pub trait MyDb: salsa::Database {
    fn input_manager(&self) -> &InputManager;
}

// Implement concrete database
#[salsa::db]
#[derive(Clone)]
pub struct Database {
    input_manager: InputManager,
    storage: salsa::Storage<Database>, // MUST be last!
}

impl Database {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            storage: salsa::Storage::new(None),
        }
    }
}

#[salsa::db]
impl salsa::Database for Database {}

#[salsa::db]
impl MyDb for Database {
    fn input_manager(&self) -> &InputManager {
        &self.input_manager
    }
}
```

### 2. Input + Tracked Query Pattern

```rust
// Input representing source code
#[salsa::input]
pub struct SourceFile {
    #[returns(ref)]
    pub path: PathBuf,

    #[returns(ref)]
    pub content: String,
}

// Tracked parsing query
#[salsa::tracked(returns(ref), no_eq)]
pub fn parse_file(db: &dyn MyDb, file: SourceFile) -> ParseResult {
    let content = file.content(db);
    parse_source_code(content)
}

// Usage
let db = Database::new();
let file = SourceFile::new(&db, path, content);
let parsed = parse_file(&db, file);
```

### 3. Interned Type Pattern

```rust
// Interned identifier for efficient comparison
#[salsa::interned(debug)]
#[derive(PartialOrd, Ord)]
pub struct Symbol<'db> {
    #[returns(ref)]
    pub name: String,

    pub scope: ScopeId<'db>,
}

// Usage in tracked functions
#[salsa::tracked]
pub fn resolve_symbol(db: &dyn MyDb, symbol: Symbol) -> SymbolInfo {
    // Implementation
}
```

### 4. Cycle Recovery Template

```rust
#[salsa::tracked(cycle_fn=analysis_cycle_recover, cycle_initial=analysis_cycle_initial)]
pub fn analyze_with_dependencies(db: &dyn MyDb, item: ItemId) -> Analysis {
    let mut analysis = Analysis::new();

    // This might create cycles through dependencies
    for dep in item.dependencies(db) {
        let dep_analysis = analyze_with_dependencies(db, dep);
        analysis.merge(dep_analysis);
    }

    analysis.finalize(item)
}

fn analysis_cycle_recover(
    _db: &dyn MyDb,
    _value: &Analysis,
    _count: u32,
    _item: ItemId,
) -> salsa::CycleRecoveryAction<Analysis> {
    salsa::CycleRecoveryAction::Iterate
}

fn analysis_cycle_initial(_db: &dyn MyDb, _item: ItemId) -> Analysis {
    Analysis::empty() // Start with empty analysis
}
```

This guide provides a solid foundation for using Salsa effectively, based on the
patterns that Ruff has refined through extensive real-world usage. The key is to
start simple and gradually adopt more advanced patterns as your system grows in
complexity.
