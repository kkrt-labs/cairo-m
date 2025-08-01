# Cairo-M Codegen Crate

## Overview

The `codegen` crate is responsible for translating MIR (Mid-level Intermediate
Representation) to CASM (Cairo Assembly) instructions. It's the final stage of
the Cairo-M compiler pipeline that produces executable assembly code for the
Cairo VM.

## Architecture

### Core Components

- **`generator.rs`**: Main code generation orchestration
  - Implements two-pass compilation: instruction generation followed by label
    resolution
  - Manages function and basic block generation
  - Handles control flow between blocks

- **`builder.rs`**: Instruction building utilities
  - Provides high-level methods for generating CASM instructions
  - Translates MIR instructions to Cairo VM opcodes
  - Implements binary operations, assignments, function calls, and control flow

- **`layout.rs`**: Stack frame layout management
  - Calculates fp-relative offsets for all values
  - Implements Cairo calling convention:
    - Arguments: `fp - M - K - 2` to `fp - K - 3`
    - Return values: `fp - K - 2` to `fp - 3`
    - Locals: `fp + 0` onwards

### Key Features

✅ **Implemented**:

- Basic arithmetic operations (add, sub, mul, div)
- Control flow (if-else, jumps, conditional branches)
- Function calls with arguments and return values
- Local variables and parameter handling
- Label resolution for jumps and function calls
- Stack frame management

❌ **Not Yet Implemented**:

- Load/Store for indirect memory access
- AddressOf operations
- GetElementPtr for struct/array access
- Cast operations
- Debug instructions

## CASM Output Format

Generated CASM follows this format (debug mode):

```text
label_name:
  PC: OPCODE OFF0 OFF1 OFF2 OPERAND    // comment
```

Examples:

- `6 -3 42` - Store immediate 42 at [fp -3]
- `0 -6 -5 2` - Add [fp -6] and [fp -5], store at [fp +2]
- `12 7 10` - Call function at address 10
- `31 0 4` - Jump relative 4 if [fp +0] != 0

## Contributing

### Adding New Instructions

1. Add the MIR instruction case in `builder.rs::generate_instruction()`
2. Implement the translation logic using `CasmBuilder` methods
3. Add test cases in `tests/test_cases/`
4. Run tests and update snapshots:
   `cargo test --package cairo-m-compiler-codegen`

### Testing

The crate uses snapshot testing with `insta`. Test workflow:

1. Write a `.cm` test file in `tests/test_cases/`
2. Add a test function in `tests/codegen_tests.rs`
3. Run tests: `cargo test`
4. Review snapshots: `cargo insta review`

### Code Style

- Use descriptive variable names
- Add comments for complex logic
- Follow Rust naming conventions
- Keep functions focused and small
- Write comprehensive tests for new features

## Design Decisions

1. **Two-Pass Compilation**: First generate instructions with symbolic labels,
   then resolve to concrete addresses
2. **FP-Relative Addressing**: All memory access is frame pointer relative
3. **Calling Convention**: Follows Cairo VM conventions with communication area
   for arguments
4. **Error Handling**: Structured errors with specific variants for different
   failure modes

## Future Improvements

See `session.md` for the current development roadmap and optimization
opportunities.
