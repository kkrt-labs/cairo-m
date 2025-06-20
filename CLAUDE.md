# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

Cairo-M is a brand new CPU AIR (Algebraic Intermediate Representation)
leveraging the M31 prime field (2^31 - 1) as its native prime field, designed to
enable efficient proving on consumer hardware, particularly mobile devices. It
uses Starkware's Stwo proving system and follows a minimal register architecture
similar to Cairo Zero but optimized for modern proving systems.

## Essential Commands

### Building and Running

```bash
# Initial setup (required for Stwo submodule)
git submodule update --init --recursive

# Build entire workspace
cargo build
cargo build --release

# Run the compiler
cargo run --bin cairo-m-compiler -- -i demo.cm              # Compile Cairo-M file

# Run compiled programs
cargo run --bin cairo-m-runner -- <json-file> --entry-point <function-name>

# Run benchmarks
RUSTFLAGS="-C target-cpu=native" cargo bench --bench vm_benchmark -- --verbose
```

### Testing

```bash
# Run all tests
cargo test

# Test specific crates
cargo test -p cairo-m-compiler-parser
cargo test -p cairo-m-compiler-semantic
cargo test -p cairo-m-compiler-mir
cargo test -p cairo-m-compiler-codegen
cargo test -p cairo-m-runner

# Run specific test categories
cargo test --test semantic_tests scoping
cargo test --test semantic_tests control_flow

# Snapshot testing
cargo insta review    # Review snapshot changes
cargo insta accept    # Accept all snapshot changes

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Using trunk.io (preferred)
trunk check     # Run all linters
trunk fmt       # Auto-format code

# Direct usage
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::nursery --cap-lints=warn --no-deps -D warnings -D unused_imports
cargo fmt
```

## Architecture

Cairo-M consists of three main crates:

### 1. Compiler (`crates/compiler/`)

Multi-phase incremental compiler using Salsa framework:

- **Parser**: Lexing (logos) and parsing (chumsky) with error recovery
- **Semantic**: Type checking, name resolution, and validation
- **MIR**: Middle-level IR with control flow graphs and virtual registers
- **Codegen**: Generates Cairo Assembly (CASM)

### 2. Runner (`crates/runner/`)

VM with minimal register architecture:

- Two registers: PC (program counter) and FP (frame pointer)
- Flat memory model storing QM31 values
- Fixed instruction encoding: [opcode, arg0, arg1, arg2]
- Generates execution traces for proof generation

### 3. Prover (`crates/prover/`)

- Generates proofs using Stwo proving system
- Handles memory constraints and verification
- Binary trace format for efficient proof generation

## Key Technical Details

### Compilation Pipeline

```text
Source (.cm) → Parser → Semantic Analysis → MIR → Code Generation → CASM
```

### Database-Driven Architecture

Uses Salsa for incremental compilation with queries:

- `parse_source_file(db, file) -> Module`
- `semantic_index(db, file) -> SemanticIndex`
- `lower_to_mir(db, file) -> MirModule`

### Testing Infrastructure

- Snapshot testing with `insta` for all compiler phases
- Test fixtures organized by feature category
- Diff tests comparing Cairo-M execution with Rust implementations
- Custom test harness macros for semantic validation

### Development Tools

- `ast-grep` (`sg`) installed for structural code search
- Rust nightly toolchain (see rust-toolchain.toml)
- trunk.io for linting and formatting
- macOS users need `lld` and Homebrew's `llvm`

### Memory and Instruction Model

- Instructions loaded at address 0
- Frame layout: [args, return_values, old_fp, return_pc]
- 32 opcodes including arithmetic, memory, control flow, and immediate
  operations
- Field arithmetic in M31 (division uses multiplicative inverse)

## Working with the Codebase

When implementing features for the compiler:

1. Update parser grammar in `parser/src/parser.rs`
2. Add semantic validation in `semantic/src/validation/`
3. Extend MIR generation in `mir/src/ir_generation.rs`
4. Implement codegen in `codegen/src/generator.rs`
5. Write tests with snapshots at each level

Use `sg --lang rust -p <pattern>` for syntax-aware code search instead of
text-based tools unless explicitly needed.

The compiler preserves comprehensive error information through all phases to
provide helpful diagnostics to users.

## Coding Guidelines

### Error Handling

- Use `thiserror` for custom error types to avoid boilerplate in `Display` and
  `Error` implementations
- Use `anyhow` for error handling in binaries with `with_context()` for
  descriptive error messages
- Prefer `unwrap_or_else()` over `unwrap()` when providing custom error handling
- Use `#[from]` attribute with thiserror for automatic error conversion
- Use `?` operator instead of manual error propagation where possible

Example:

```rust
// Good - using anyhow with context
let source_text = fs::read_to_string(&args.input)
    .with_context(|| format!("Failed to read file '{}'", args.input.display()))?;

// Good - using thiserror #[from]
ReturnValueError(#[from] MemoryError),
```

### Iterator Patterns and Performance

- Use single-pass iteration when possible to avoid multiple traversals

Example:

```rust
// Good - single pass with partition
let (semantic_errors, warnings): (Vec<_>, Vec<_>) = semantic_diagnostics
    .into_iter()
    .filter(|d| {
        matches!(
            d.severity,
            DiagnosticSeverity::Error | DiagnosticSeverity::Warning
        )
    })
    .partition(|d| d.severity == DiagnosticSeverity::Error);

// Good - using Into trait for conversions
program.instructions.iter().map(Into::into).collect();
```

### Documentation and Comments

- Use standard Rust doc comment format with `## Arguments`, `## Returns`
  sections
- Provide context in comments when implementation choices aren't obvious
- Specify algorithmic complexity or implementation details when relevant
- Avoid in-code comments unless they are necessary to understand complex code

Example:

```rust
/// ## Arguments
/// * `program` - The compiled program to run
/// * `entrypoint` - Name of the entry point function to execute
/// * `options` - Runner options
///
/// ## Returns
```
