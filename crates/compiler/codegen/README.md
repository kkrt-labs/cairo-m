# Cairo-M Codegen Crate

## Overview

The `codegen` crate translates MIR (Mid-level Intermediate Representation) into
CASM (Cairo Assembly). It is the final stage of the Cairo‑M compiler pipeline
and produces instructions executable by the Cairo VM.

This crate follows a “Normalize → Select → Emit” architecture:

- Normalize: Canonicalize shapes (swap/complement/bias rules) into a small set
  of normalized operations.
- Select: Choose opcodes from a single source of truth (opcodes table) for the
  normalized shape.
- Emit: Push instructions, labels, and track frame writes through a single
  emission seam.

## Architecture

### Core Components

- `generator.rs`: Orchestrates code generation (two-pass: generation and label
  resolution), walks functions and basic blocks, and delegates to the builder
  facade.

- `builder.rs`: Thin facade and state holder (frame usage, labels, instruction
  buffer). Delegates domain logic to small modules:
  - `builder/emit.rs`: Central emission helpers (push instructions, labels,
    touch tracking). All pushes and label creation go through here.
  - `builder/copy.rs`: Felt/u32/aggregate copy utilities (`copy_slots`,
    `store_copy_u32`). Eliminates ad‑hoc loops.
  - `builder/normalize.rs`: Pure normalization rules (swap/complement/bias). No
    side effects, easy to test.
  - `builder/opcodes.rs`: Opcode selection table for normalized ops (single
    source of truth).
  - `builder/felt.rs`: Felt operations (assign/arith/boolean) using normalize +
    opcode selection + emit.
  - `builder/u32_ops.rs`: U32 arithmetic/comparison/bitwise ops; handles two’s
    complement, bias rules, complements.
  - `builder/ctrlflow.rs`: Short‑circuit lowering templates (AND/OR/NOT) in one
    place.
  - `builder/calls.rs`: Calls, argument passing (with in‑place optimization),
    and returns.

- `layout.rs`: Stack frame layout management
  - Computes fp‑relative offsets for all values
  - Calling convention (callee perspective):
    - Arguments: `fp - M - K - 2` .. `fp - K - 3`
    - Return values: `fp - K - 2` .. `fp - 3`
    - Locals/temps: `fp + 0` ..
  - Provides `FunctionLayout::new_for_test()` and `allocate_value` for
    lightweight tests.

## Testing Strategy

Two complementary approaches:

- Unit tests (preferred for logic close to the builder):
  - Pure rules (normalize, opcode selection): small, fast, deterministic tests.
  - Emission sequences (ctrlflow, felt/u32 ops, copy, calls): construct a
    minimal `FunctionLayout::new_for_test()`, allocate a few `ValueId`s with
    known offsets, invoke the corresponding API, and assert on
    `InstructionBuilder` opcodes/operands and labels.
  - Reusable helpers: `ValueId::from_raw`, `Value::{operand, integer, boolean}`
    keep tests concise.

- Snapshot tests (integration):
  - Live in `tests/` and exercise full MIR → CASM paths.
  - Use `insta` to manage snapshots. After a refactor that changes comments or
    sequencing, update via `cargo insta review`.

Run all tests:

- Unit/build checks: `cargo check -p cairo-m-compiler-codegen`
- Full tests: `cargo test -p cairo-m-compiler-codegen`
- Review snapshots (when needed):
  `cargo insta review -p cairo-m-compiler-codegen`

## Key Features

- Felt arithmetic and boolean operations
- U32 arithmetic, comparisons, and bitwise operations
- Short‑circuit boolean lowering (AND/OR/NOT) in one place
- Function calls with argument‑in‑place optimization and return handling
- Label resolution for jumps and calls
- Stack frame layout and frame usage tracking

## Design Principles

- Single‑purpose modules: Each concern (normalize, selection, emission,
  ctrlflow, calls) has its own module and tests.
- Deterministic emission: All pushes/labels/touch go through `emit.rs` to ensure
  uniform behavior and easier auditing.
- Testable by construction: Pure rules and small templates make it
  straightforward to reason about and unit test codegen decisions.

## Contributing

When adding or changing codegen:

- Put logic in the appropriate module (e.g.,
  felt/u32/ctrlflow/calls/copy/normalize/opcodes).
- Add unit tests close to the logic you changed. Prefer small tests that assert
  opcodes and operands.
- If a change affects integration snapshots, run `cargo test` and update via
  `cargo insta review`.

Guidelines:

- Keep functions focused and small; avoid monolithic match arms.
- Prefer data‑driven normalization and selection to ad‑hoc rewrites.
- Treat comments as part of the contract — they are visible in snapshots and
  help users understand the output.

## Future Improvements

- Expand normalization coverage (more derived ops, fewer explicit opcodes).
- Broaden unit test coverage (bitwise and comparisons) where useful.
- Explore additional short‑circuit patterns (e.g., chained logical operators) if
  MIR needs it.
