# Heap Allocation and Typed Pointers Plan

This document outlines the minimal, end-to-end changes to support heap
allocation via a `new` expression and typed pointers that flow from the frontend
to the runner.

## Big Picture

- Add `new T[n]` expression that returns a `T*`.
- Done: Added `TokenType::New` and `Expression::New { elem_type, count }`;
  lowered through all phases.
- Introduce typed pointers in semantic types; represent pointers in MIR as
  `Pointer<T>` (single-slot, typed).
- Done: Implemented `TypeData::Pointer{element_type}` in semantic; mapped to
  `MirType::Pointer { element }`.
- Lower `new T[n]` to a MIR `HeapAllocCells` with
  `cells = n * size_in_slots(T)`.
- Done: Implemented in MIR lowering (`expr.rs`) using
  `DataLayout::value_size_of`.
- Allow `ptr[i]` reads/writes via explicit `Load`/`Store` on `Place`s in MIR;
  codegen scales by element size.
- Done: MIR lowering emits `Load`/`Store` for pointer interactions with proper
  element types.

## Parser

- Files:
  - `crates/compiler/parser/src/lexer.rs`
  - `crates/compiler/parser/src/parser.rs`
- Changes:
  - Keyword: Add token `TokenType::New` (+ `Display`) so the lexer recognizes
    `new`.
    - Status: Implemented in `lexer.rs`.
  - AST: Add
    `Expression::New { elem_type: Spanned<TypeExpr>, count: Spanned<Expression> }`.
    - Status: Implemented in `parser.rs`.
  - Grammar: In the expression parser `Primary` tier, parse
    `new TypeExpr "[" Expr "]"` into `Expression::New { ... }`.
    - Status: Implemented in `parser.rs` (uses simplified named type as
      planned).
  - Notes: Pointer types already parse via
    `TypeExpr::Pointer(Box<Spanned<TypeExpr>>)`; no changes required here. To
    keep things simple and break cycles, `new` may use a simplified type parser
    (named types) initially; semantic resolution will still enforce struct
    names, etc.
    - Status: As planned; full `TypeExpr` support for `new` can be added later.

## Semantic

- Files:
  - `crates/compiler/semantic/src/types.rs`
  - `crates/compiler/semantic/src/type_resolution.rs`
  - `crates/compiler/semantic/src/validation/*` (optional adjustments)
- Changes:
  - Types: add typed pointers
    - Add `TypeData::Pointer { element_type: TypeId<'db> }`.
    - Update `TypeId::format_type` / `TypeData::display_name` to render `T*`.
    - Status: Implemented in `types.rs` (formatting and display included).
  - Type resolution
    - In `resolve_ast_type`: map `AstTypeExpr::Pointer(inner)` to
      `TypeData::Pointer` instead of `Error`.
    - In `expression_semantic_type`:
      - Add case for `Expression::New`: resolve `elem_type`; return
        `TypeData::Pointer { element_type: ... }`.
      - Extend IndexAccess to accept pointers: if the container type is
        `Pointer { element_type }`, the resulting type is `element_type`.
    - Status: Implemented in `type_resolution.rs` (pointer type, `new`, and
      pointer indexing supported).
  - Type compatibility
    - Extend `are_types_compatible` to consider `Pointer(A)` compatible with
      `Pointer(B)` iff `A` compatible with `B`.
    - Status: Implemented in `type_resolution.rs`.
  - Validation
    - No special validation required beyond existing rules; optional restriction
      to disallow `new` in const contexts can be added later.
    - Status: Added `new`-specific count type check; element type is visited via
      `visit_type_expr`, so undeclared types are caught by existing scope
      validator.

## MIR

- Files:
  - `crates/compiler/mir/src/mir_types.rs`
  - `crates/compiler/mir/src/lowering/expr.rs`
  - `crates/compiler/mir/src/lowering/stmt.rs`
  - (Instruction already exists) `crates/compiler/mir/src/instruction.rs`
- Changes:
  - Pointer mapping
    - In `MirType::from_semantic_type`, map `TypeData::Pointer { .. }` to
      `MirType::Pointer { element }` (single-slot, typed).
    - Status: Implemented in `mir_types.rs`.
  - Lowering: `new`
    - Handle `Expression::New { elem_type, count }`:
      - Convert element type to MIR type and compute
        `elem_slots = DataLayout::value_size_of(elem_ty)`.
      - Lower `count` to a `Value`. If `elem_slots > 1`, emit a `* elem_slots`
        multiply to get `cells`.
      - Emit `Instruction::heap_alloc_cells(dest, cells)` where `dest: felt` is
        the pointer.
    - Status: Implemented in `lowering/expr.rs`.
  - Lowering: pointer indexing
    - Reads: For `ptr[i]`, build a `Place` with index projection and emit
      `Load(dest, place, elem_ty)`.
    - Writes: For `ptr[i] = v`, reuse the same `Place` extended as needed
      (fields/tuples) and emit `Store(place, v, elem_ty)`. Do not rebind the
      pointer variable.
    - Status: Implemented in `lowering/expr.rs` (reads) and `lowering/stmt.rs`
      (writes); uses `Place` projections.

## Codegen

- Files: No changes required.
  - `crates/compiler/codegen/src/generator.rs` already lowers `HeapAllocCells`
    using a bump allocator over a global `HEAP_CURSOR` and supports
    pointer-based `ArrayIndex`/`ArrayInsert` with element-size scaling.
  - Status: Match arms updated for exhaustiveness; `HeapAllocCells` lowering is
    handled at the block level.

## Runner

- Files: No changes required.
  - `crates/runner/src/memory/mod.rs` implements split locals/heap and
    high-address mapping; it aligns with the bump allocation semantics used by
    codegen.
  - Status: Verified; no changes needed.

## Edge Cases and Simplifications

- `new T[n]` supported for `felt`, `u32`, and structs composed of these.
  - Status: Implemented and covered by tests.
- Pointer arithmetic is out-of-scope; only `ptr[i]` loads/stores are supported.
  - Status: Intentionally unsupported.
- Nested arrays remain unsupported.
  - Status: Intentionally unsupported.
- Pointers are single-slot values in MIR (`Pointer<T>`), preserving element type
  for layout.
- Status: Implemented.

## Minimal Test Plan

- Parser
  - `let p: felt* = new felt[10];`
  - `let q: u32* = new u32[n];`
  - `let r: Point* = new Point[3];`
  - Status: Implemented in parser tests; all pass.
- Semantic
  - `new T[n]` has type `T*`.
  - `p[i]` has type `felt`; `q[i]` has type `u32`; `r[i].x` has the type of
    `Point.x`.
  - Status: Covered by white‑box tests and parameterized snapshot tests; all
    pass.
- MIR/Codegen snapshots
  - `new u32[n]` emits `HeapAllocCells` with `cells = 2*n`.
  - `p[i]` load emits `Load` in MIR and double-deref with scaled index in CASM.
  - `p[i] = v` emits `Store` in MIR and correct slot writes in CASM.
  - Status: MIR and codegen snapshots added for pointer cases.
