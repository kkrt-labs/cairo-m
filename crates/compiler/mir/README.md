# Cairo M Compiler - MIR (Mid-level Intermediate Representation)

This crate implements the Mid-level Intermediate Representation (MIR) for the
Cairo-M compiler. The MIR serves as a bridge between the high-level semantic AST
and the low-level Cairo Assembly (CASM) code generation.

## Overview

The MIR is a Control Flow Graph (CFG) based representation inspired by LLVM IR.
It transforms high-level language constructs into a simplified form that:

- Makes control flow explicit through basic blocks and terminators
- Uses three-address code (TAC) for simple, atomic operations
- Provides a foundation for optimization passes
- Maintains type information for safe lowering to CASM

## Architecture

### Core Components

- **MirModule**: Top-level container for all functions in a compilation unit
- **MirFunction**: Represents a single function as a CFG of basic blocks
- **BasicBlock**: Straight-line sequence of instructions with one entry/exit
- **Instruction**: Performs computation without changing control flow
- **Terminator**: Ends blocks and transfers control (jump, branch, return)
- **Value**: Represents data flowing through the program (literals or operands)
- **MirType**: Simplified type system independent of Salsa lifetimes

### Design Principles

1. **Explicit Control Flow**: All control flow is represented through
   terminators
2. **Three-Address Code**: Each instruction has at most one operation
3. **Type Safety**: Types are preserved from semantic analysis
4. **Error Recovery**: Partial MIR generation even with semantic errors
5. **Source Mapping**: Maintains connections to original AST for diagnostics

## Current Features

### Supported Language Constructs

- ✅ Functions with parameters and return values
- ✅ Variable declarations (`let`, `local`)
- ✅ Assignment statements
- ✅ Binary operations (arithmetic, comparison, logical)
- ✅ If/else statements with proper control flow merging
- ✅ Function calls (both void and with return values)
- ✅ Struct literals and field access (read/write)
- ✅ Tuple literals and indexed access
- ✅ Return statements

### Instruction Set

- **Memory Operations**: `stackalloc`, `load`, `store`, `getelementptr`
- **Data Movement**: `assign`
- **Arithmetic**: `binary_op` (via BinaryOp enum)
- **Control Flow**: `jump`, `if-then-else`, `return`
- **Function Calls**: `call`, `void_call`
- **Address Operations**: `address_of`, `cast`
- **Debug**: `debug` instructions for diagnostics

### Optimization Passes

- **DeadCodeElimination**: Removes unreachable basic blocks
- **Validation**: Ensures MIR invariants are maintained

## Known Issues

1. **Double Allocation Bug**: Aggregate literals (structs/tuples) are allocated
   twice when assigned to variables. The expression allocates once, then the let
   statement allocates again unnecessarily. See `report.md` for details.

2. **Missing Features**:
   - No support for loops yet
   - Arrays are rudimentary (using placeholder `felt*` type)
   - No enum/match support
   - Limited optimization passes

## Usage Example

```rust
use cairo_m_compiler_semantic::{File, SemanticDatabaseImpl};
use cairo_m_compiler_mir::generate_mir;

let db = SemanticDatabaseImpl::default();
let file = File::new(&db, source_code, "example.cm");

if let Some(mir_module) = generate_mir(&db, file) {
    // MIR is successfully generated
    println!("{}", mir_module.pretty_print(0));
}
```

## Testing

The crate includes a comprehensive test suite with:

- Snapshot testing using `insta`
- Custom assertion system via `//!ASSERT` comments
- Test cases covering all supported features
- Automatic validation of generated MIR

Run tests with:

```bash
cargo test -p cairo_m_compiler_mir
```

## Next Steps

See `session.md` for a detailed roadmap of planned improvements and features.
