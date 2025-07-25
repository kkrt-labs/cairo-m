# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

Cairo-M is a minimal CPU AIR (Algebraic Intermediate Representation) leveraging
the M31 prime field (2^31 - 1) for efficient proving on consumer hardware,
particularly mobile devices. The runner crate executes compiled Cairo-M programs
and generates execution traces for proof generation.

## Key Commands

### Build and Test

```bash
# Build the runner
cargo build -p cairo-m-runner

# Run tests
cargo test -p cairo-m-runner

# Execute a compiled program
cargo run --bin cairo-m-runner -- <json-file> --entry-point <function-name>
```

### Code Quality

```bash
# Run all linters and formatters
trunk check

# Auto-format code
trunk fmt
```

## Architecture Overview

### VM Design

The Cairo-M VM uses a minimal register architecture:

- **PC**: Program Counter
- **FP**: Frame Pointer
- **Memory**: Flat address space storing QM31 values (4 M31 field elements)
- **Variable-size instruction encoding**: 1-5 M31 elements per instruction

### Memory Layout

- Instructions loaded at address 0
- Frame pointer initialized after bytecode
- Call convention: [args, return_values, old_fp, return_pc]

### Instruction Set

32 opcodes (0-31) organized into categories:

- **Arithmetic**: add, sub, mul, div (field operations)
- **Memory**: store operations (arithmetic results, immediates)
- **Control flow**: jump, jnz, call, ret
- **Special**: imm (load immediate values)

### Code Organization

```text
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── vm/
│   ├── mod.rs       # VM implementation and execution loop
│   ├── state.rs     # PC/FP register management
│   └── instructions/# Instruction implementations
└── memory/
    └── mod.rs       # Memory management and tracing
```

### Key Implementation Details

1. **Execution Model**: Step-based execution where each step fetches, decodes,
   and executes one instruction
2. **Tracing**: RefCell-based interior mutability for collecting execution
   traces
3. **Error Handling**: Custom error types for memory and instruction errors
4. **Field Arithmetic**: Division uses multiplicative inverse in M31 field
5. **Memory Growth**: Automatic expansion up to 2^30 elements

### Integration Points

The runner integrates with:

- `cairo-m-compiler`: Loads CompiledProgram JSON format
- `stwo-prover`: Generates traces compatible with Stwo proving system
- Binary trace format for efficient proof generation

### Coding Guidelines

#### Error Handling

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

#### Iterator Patterns and Performance

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

#### Documentation and Comments

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
