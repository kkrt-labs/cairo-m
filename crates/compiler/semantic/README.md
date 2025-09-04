# Cairo‑M Semantic – Consumer Guide

This crate provides the semantic layer for Cairo‑M. It exposes a stable,
consumer‑focused API to:

- Build a semantic index for a file or module (Salsa‑cached)
- Navigate scopes and definitions (DefinitionIndex‑first)
- Resolve names (position‑aware, shadowing‑correct, with imports)
- Run type queries (expression/definition types, signatures)
- Validate code and collect diagnostics

The internals deliberately avoid redundant state. Name lookup uses a per‑scope
`name_index` and use‑def mappings recorded by the builder.

Who should read this:

- Language Server, MIR, codegen, lint/validation, tools consuming semantic data.

## Quick Start

```rust
use cairo_m_compiler_semantic as sem;
use sem::{SemanticDb, module_semantic_index};

// Get module index (Salsa‑cached)
let index = module_semantic_index(&db, crate_id, "main".to_string()).unwrap();

// Resolve a name in a scope chain (DefinitionIndex‑first)
let root = index.root_scope().unwrap();
let def_idx = index
    .latest_definition_index_by_name_in_chain(root, "foo")
    .expect("definition not found");
let def = index.definition(def_idx).unwrap();

// Get a type of an expression
let expr_ty = sem::type_resolution::expression_semantic_type(&db, crate_id, file, expr_id, None);

// Validate a whole crate
let diags = sem::db::project_validate_semantics(&db, crate_id);
```

## Building and Accessing the Index

- Per module: `module_semantic_index(&db, crate_id, module_name)`
- Per crate: `project_semantic_index(&db, crate_id)` (map of module name →
  index)
- Per file (from a parsed module):
  `semantic_index_from_module(&db, parsed_module, file, crate_id)`

Index navigation:

- `root_scope() -> Option<FileScopeId>`
- `scopes() -> Iterator<(FileScopeId, &Scope)>`
- `child_scopes(parent: FileScopeId) -> Iterator<FileScopeId>`
- `scope_for_span(span) -> Option<FileScopeId>`

Expressions:

- `all_expressions() -> Iterator<(ExpressionId, &ExpressionInfo)>`
- `expression(expr_id) -> Option<&ExpressionInfo>`
- `expression_id_by_span(span) -> Option<ExpressionId>`
- `span_expression_mappings() -> &FxHashMap<Span, ExpressionId>`

Definitions:

- `all_definitions() -> Iterator<(DefinitionIndex, &Definition)>`
- `definitions_in_scope(scope) -> Iterator<(DefinitionIndex, &Definition)>`
- `definition(def_idx) -> Option<&Definition>`
- `DefinitionId::new(&db, file, def_idx)` creates a globally unique ID
- `is_definition_used(def_idx) -> bool` (read or write observed)

## Name Resolution

Use the right tool for the context:

1. Exact identifier site (preferred in MIR/type inference):

- `definition_for_identifier_expr(expr_id) -> Option<(DefinitionIndex, &Definition)>`
- Uses the builder’s single‑source‑of‑truth use‑def mapping (no re‑resolution).

2. Position‑aware lookup (validators / IDE):

- `resolve_name_at_position(name, starting_scope, position_span) -> Option<(DefinitionIndex, &Definition)>`
  - Shadowing‑correct; excludes future `let`s and self‑reference within the
    initializer.
- `resolve_name_with_imports_at_position(db, crate, file, name, starting_scope, position_span)`
  - Same as above, then follows visible imports. Imported module resolution
    targets top‑level items and allows forward refs for Function/Struct/Use.

Fast helpers by scope name:

- `latest_definition_index_by_name(scope, name)` and
- `latest_definition_index_by_name_in_chain(scope, name)`

## Use‑Def Mapping

- `identifier_usages() -> &[IdentifierUsage]`
- `is_usage_resolved(usage_idx) -> bool`
- `get_use_definition(usage_idx) -> Option<&Definition>`

For consumer crates, prefer `definition_for_identifier_expr(expr_id)` whenever
you are at an identifier expression — it avoids any re‑resolution and matches
exactly what the builder recorded.

## Type Queries

- Expression:
  - `expression_semantic_type(&db, crate_id, file, expr_id, expected_hint) -> TypeId`
- Definition:
  - `definition_semantic_type(&db, crate_id, def_id) -> TypeId`
- AST Types:
  - `resolve_ast_type(&db, crate_id, file, type_ast, scope_id) -> TypeId`
- Signatures and struct data:
  - `function_semantic_signature(&db, crate_id, def_id) -> FunctionSignatureId`
  - `struct_semantic_data(&db, crate_id, def_id) -> StructTypeId`

## Validation (Diagnostics)

- Whole‑crate validation:
  `project_validate_semantics(&db, crate_id) -> DiagnosticCollection`
- Per‑validator usage (example):
  ```rust
  use cairo_m_compiler_semantic::validation::{Validator, scope_check::ScopeValidator};
  let sink = cairo_m_compiler_diagnostics::VecSink::new();
  ScopeValidator.validate(&db, crate_id, file, &index, &sink);
  let diagnostics = sink.into_diagnostics();
  ```

## Language Server Patterns

- Cursor → scope: `scope_for_span(SimpleSpan::from(pos..pos))`
- Position‑aware resolve: `resolve_name_at_position` first; if not found,
  `resolve_name_with_imports_at_position`.
- Goto‑definition: build a `DefinitionId` from `(file, def_idx)` and return
  spans from `Definition`.
- Hover: use `definition_semantic_type`/`function_semantic_signature` for types
  and signatures.

## MIR / Codegen Patterns

- Never re‑resolve identifiers. From an `ExpressionId` for an identifier:
  - `definition_for_identifier_expr(expr_id)` → `DefinitionIndex`
  - `DefinitionId::new(&db, file, def_idx)` → use in SSA or downstream layers
- Use `definition_semantic_type` to determine variable types during lowering.

## Do’s and Don’ts

- Do use DefinitionIndex‑first helpers (`definition_for_identifier_expr`,
  `latest_definition_index_by_name*`).
- Do use position‑aware APIs for IDE/validators.
- Don’t re‑resolve identifier names during type inference/MIR lowering.
- Don’t depend on internal tables; there is no public PlaceTable or flags.

## Performance Notes

- All major queries are Salsa‑tracked and cached.
- Lookups are O(1) or better with `IndexVec` and per‑scope `name_index`.
- Recording use‑def once during building avoids repeated name resolution in
  consumers.

## Public Re‑exports

From `lib.rs`:

- DB/queries: `module_semantic_index`, `project_semantic_index`, `SemanticDb`,
  `SemanticDatabaseImpl`
- Index/IDs: `SemanticIndex`, `DefinitionId`, `ExpressionId`, `FileScopeId`,
  `Scope`, `ScopeKind`
- Types: `TypeId`, `TypeData`, `StructTypeId`, `FunctionSignatureId`
- Definition model: `Definition`, `DefinitionKind`

That’s it — lean, position‑aware, and DefinitionIndex‑first.

- Prefer IndexVec over HashMap where possible
- Keep hot paths allocation-free
