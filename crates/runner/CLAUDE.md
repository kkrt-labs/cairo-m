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
- **Fixed instruction encoding**: [opcode, arg0, arg1, arg2]

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
