# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Building and Running the Compiler

```bash
# Build the compiler
cargo build

# Run the compiler on a Cairo-M file
cargo run -- -i demo.cm

# Run with verbose output (shows MIR)
cargo run -- -i demo.cm -v

# Run release build
cargo build --release
cargo run --release -- -i demo.cm
```

## Testing Commands

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p cairo-m-compiler-parser
cargo test -p cairo-m-compiler-semantic
cargo test -p cairo-m-compiler-mir
cargo test -p cairo-m-compiler-codegen

# Run specific test
cargo test test_simple_function

# Review snapshot changes (when tests fail due to output changes)
cargo insta review

# Accept all snapshot changes
cargo insta accept

# Run tests with output
cargo test -- --nocapture

# Run semantic tests by category
cargo test --test semantic_tests scoping
cargo test --test semantic_tests control_flow
cargo test --test semantic_tests functions
```

## High-Level Architecture

The Cairo-M compiler follows a traditional multi-phase compiler architecture
with incremental compilation support via Salsa:

```text
Source (.cm) → Parser → Semantic Analysis → MIR → Code Generation → CASM
```

### Key Architectural Decisions

1. **Incremental Compilation**: Uses Salsa framework (like rust-analyzer) for
   caching and fine-grained dependency tracking. Database-driven architecture
   where each phase queries the previous phase.

2. **Phase Separation**: Each compilation phase is in its own crate with clear
   interfaces. Phases communicate through Salsa queries, not direct
   dependencies.

3. **Error Recovery**: The compiler continues processing after errors to provide
   comprehensive diagnostics. Each phase can handle partial/invalid input from
   previous phases.

4. **Snapshot Testing**: Extensive use of `insta` for testing compiler outputs.
   All phases use snapshot tests for AST structures, diagnostics, MIR, and
   generated code.

### Compilation Pipeline

1. **Parser** (`cairo-m-compiler-parser`):

   - Lexer: Uses `logos` for fast tokenization
   - Parser: Uses `chumsky` parser combinators for error recovery
   - Produces AST with full source location information
   - Key queries: `parse_source_file(db, file) -> Module`

2. **Semantic Analysis** (`cairo-m-compiler-semantic`):

   - Builds semantic index with scope/symbol information
   - Performs name resolution (use-def analysis)
   - Type inference and validation
   - Generates comprehensive diagnostics
   - Key queries: `semantic_index(db, file) -> SemanticIndex`

3. **MIR Generation** (`cairo-m-compiler-mir`):

   - Transforms AST to Middle-Level IR (inspired by LLVM)
   - Functions as control flow graphs of basic blocks
   - Three-address code with virtual registers
   - Supports optimization passes
   - Key queries: `lower_to_mir(db, file) -> MirModule`

4. **Code Generation** (`cairo-m-compiler-codegen`):
   - Converts MIR to Cairo Assembly (CASM)
   - Stack-based memory layout (fp-relative addressing)
   - Two-pass label resolution for jumps
   - Uses `stwo-prover` for M31 field arithmetic

### Database Layer (Salsa)

The compiler uses Salsa's database pattern for incremental compilation:

```rust
// Each crate defines its database trait
#[salsa::db]
pub trait ParserDb: SourceDb + Upcast<dyn SourceDb> {
    // Parser queries
}

#[salsa::db]
pub trait SemanticDb: ParserDb + Upcast<dyn ParserDb> {
    // Semantic queries
}
```

Key Salsa patterns used:

- `#[salsa::input]` for source files
- `#[salsa::tracked]` for computed values (AST, semantic info)
- `#[salsa::interned]` for deduplicated values (identifiers, types)
- Cycle recovery for recursive types/inference

### Testing Infrastructure

The project uses sophisticated testing with fixture files and snapshots:

- **Parser tests**: Test cases in `parser/tests/test_cases/`, snapshots of AST
  structure
- **Semantic tests**: Organized by concern (scoping/, control_flow/, functions/,
  types/)
- **MIR tests**: IR generation snapshots for various language features
- **Codegen tests**: Assembly output snapshots

Test helpers:

- `assert_parse_snapshot!` - Parser AST snapshots
- `assert_diagnostics_snapshot!` - Semantic error snapshots
- `assert_semantic_ok!` / `assert_semantic_err!` - Inline semantic tests

### Language Features Supported

- Functions with parameters and return types
- Local variables (`let` and `local` bindings)
- Control flow (`if`/`else` statements)
- Basic types (`felt`)
- Arithmetic and comparison operations
- Function calls
- Structs (parsing complete, semantic analysis in progress)
- Arrays and tuples (parsing complete)

### Development Workflow

When adding new features:

1. **Update Parser**: Add grammar rules in `parser/src/parser.rs`
2. **Add Semantic Validation**: Create validator in `semantic/src/validation/`
3. **Extend MIR Generation**: Update `mir/src/ir_generation.rs`
4. **Implement Codegen**: Add instruction generation in
   `codegen/src/generator.rs`
5. **Write Tests**: Add test cases with snapshots at each level

### Key Implementation Details

- **Error Reporting**: Uses `ariadne` for beautiful diagnostics with source
  spans
- **AST Design**: Preserves all source information including trivia (comments,
  whitespace)
- **MIR Design**: SSA-like with virtual registers, explicit control flow
- **Memory Model**: Stack-based with fp-relative addressing in generated CASM
