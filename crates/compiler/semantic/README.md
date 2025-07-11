# Cairo-M Semantic Analysis

This crate implements semantic analysis for the Cairo-M compiler, transforming
parsed AST into a rich semantic model with comprehensive validation and type
checking.

## 🎯 Purpose

The semantic crate is the brain of the Cairo-M compiler. It takes the syntactic
AST from the parser and builds a complete understanding of the program's
meaning, including:

- **Name Resolution**: Links every identifier usage to its definition
- **Type Inference**: Determines the type of every expression
- **Scope Analysis**: Tracks variable visibility and lifetime
- **Semantic Validation**: Detects errors like undeclared variables, type
  mismatches, and dead code
- **Incremental Compilation**: Uses Salsa framework for efficient caching and
  recompilation

## 🏗️ Architecture

The crate follows a layered architecture inspired by rust-analyzer and Ruff:

```text
┌─────────────────────────────────────┐
│        Type System Layer            │ ← Type inference & checking
├─────────────────────────────────────┤
│        Validation Layer             │ ← Semantic rules & diagnostics
├─────────────────────────────────────┤
│        Semantic Index               │ ← Core semantic model
├─────────────────────────────────────┤
│    Definitions & Use-Def Chains     │ ← Symbol resolution
├─────────────────────────────────────┤
│      Places & Scope Hierarchy       │ ← Scope tracking
├─────────────────────────────────────┤
│          Parser AST                 │ ← Input from parser crate
└─────────────────────────────────────┘
```

### Core Components

#### **Semantic Index** (`semantic_index.rs`)

The heart of semantic analysis. Contains:

- Complete scope hierarchy with parent-child relationships
- Symbol tables (PlaceTable) for each scope
- Use-def chains linking identifier uses to definitions
- Expression metadata for type inference
- Efficient lookups via IndexVec and HashMap

#### **Type System** (`types.rs`, `type_resolution.rs`)

Salsa-based type representation:

- `TypeId`: Interned type identifiers for fast comparison
- `TypeData`: Actual type information (Felt, Struct, Tuple, Pointer, Function)
- `StructTypeId` & `FunctionSignatureId`: Interned complex types
- Type inference for all expressions
- Type compatibility checking

#### **Scope & Symbol Management** (`place.rs`, `definition.rs`)

- `Scope`: Hierarchical containers (Module, Function, Namespace, Block)
- `Place`: Named entities that can hold values
- `Definition`: Links symbols to their AST nodes and metadata
- Two-pass analysis enables forward references

#### **Validation Framework** (`validation/`)

Extensible plugin-like system:

- `ScopeValidator`: Undeclared/unused variables, duplicates
- `TypeValidator`: Type checking for operations and assignments
- `ControlFlowValidator`: Reachability and return analysis
- Beautiful diagnostics with source locations

### Key Design Decisions

1. **Salsa Integration**: All major queries are `#[salsa::tracked]` for
   incremental compilation
2. **Interned Types**: Complex types are interned for O(1) comparison
3. **Direct AST Storage**: ExpressionInfo stores AST nodes directly for fast
   access
4. **Two-Pass Analysis**: Declaration collection then body processing for
   forward refs
5. **Index-Based Storage**: Uses IndexVec for cache-friendly sequential access

## 📋 Current Capabilities

### ✅ Implemented

- **Complete Scope Analysis**: Hierarchical scope tracking with full
  parent-child relationships
- **Name Resolution**: Comprehensive use-def chains linking every identifier to
  its definition
- **Type System**:
  - Primitive types (`felt`)
  - Struct types with field access
  - Function types with signatures
  - Tuple types
  - Pointer types
  - Type inference for all expressions
- **Validation**:
  - Undeclared variable detection
  - Unused variable warnings
  - Duplicate definition errors
  - Type mismatch detection
  - Basic control flow analysis
- **Language Features**:
  - Functions with parameters and return types
  - Local variables (`let` and `local`)
  - Struct definitions and literals
  - Control flow (`if`/`else`)
  - Binary operations
  - Member access

### 🚧 Not Yet Implemented

- Arrays (parsing exists, semantic support pending)
- Loops (`for`, `while`)
- Advanced type inference (constraint solving)
- Module/import resolution
- Generic types
- Pattern matching

## 🔨 API Usage

### Basic Usage

```rust
use cairo_m_compiler_semantic::{SemanticDb, semantic_index, validate_semantics};

// Create a database
let db = SemanticDatabaseImpl::default();

// Create a source file
let file = SourceFile::new(&db, source_code, "main.cm");

// Get semantic index (cached by Salsa)
let index = semantic_index(&db, file)?;

// Run validation
let diagnostics = validate_semantics(&db, &parsed_module, file);
```

### Querying the Semantic Model

```rust
// Resolve a name to its definition
let (def_index, definition) = index.resolve_name_to_definition("variable_name", scope_id)?;

// Get type of an expression
let expr_type = expression_semantic_type(&db, file, expression_id);

// Check type compatibility
let compatible = are_types_compatible(&db, actual_type, expected_type);

// Walk scope hierarchy
let root_scope = index.root_scope()?;
for child_scope_id in index.child_scopes(root_scope) {
    let scope = index.scope(child_scope_id)?;
    println!("Found {} scope", scope.kind);
}
```

## 🧪 Testing

The crate uses a comprehensive testing approach:

- **Inline Tests**: Unit tests with helper macros like `assert_semantic_ok!` and
  `assert_semantic_err!`
- **Snapshot Tests**: Complex scenarios using `.cm` fixture files with `insta`
- **Organized by Concern**: Tests grouped by feature (scoping/, types/,
  control_flow/, etc.)

See `tests/README.md` for detailed testing documentation.

## 🔧 Contributing

### Adding Validation Rules

1. Create a new validator implementing the `Validator` trait
2. Add to the default registry in `validation/validator.rs`
3. Write comprehensive tests

### Extending the Type System

1. Add new variants to `TypeData` if needed
2. Update `resolve_ast_type` for AST→Type conversion
3. Update `expression_semantic_type` for inference
4. Add compatibility rules to `are_types_compatible`

### Performance Considerations

- All type queries should be `#[salsa::tracked]`
- Use interned types for complex type structures
- Prefer IndexVec over HashMap where possible
- Keep hot paths allocation-free
