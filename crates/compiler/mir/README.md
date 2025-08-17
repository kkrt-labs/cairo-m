# Cairo M Compiler - MIR (Mid-level Intermediate Representation)

This crate implements the Mid-level Intermediate Representation (MIR) for the
Cairo-M compiler. The MIR uses an **aggregate-first design** where tuples and
structs are first-class SSA values rather than memory locations.

> **Note:** The MIR has transitioned to value-based aggregate operations. See
> [Migration Guide](../../../docs/mir_migration_guide.md) for implementation
> details and [Aggregate-First Design](../../../docs/mir_aggregate_first.md) for
> the architecture.

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

**Value-Based Aggregate Operations** (NEW):

- **Tuple Operations**: `make_tuple`, `extract_tuple`, `insert_tuple`
- **Struct Operations**: `make_struct`, `extract_field`, `insert_field`

**Memory Operations** (for arrays and explicit addresses):

- **Allocation**: `frame_alloc`, `stack_alloc`
- **Access**: `load`, `store`, `get_element_ptr`
- **Address Operations**: `address_of`, `cast`

**Core Operations**:

- **Data Movement**: `assign`
- **Arithmetic**: `binary_op` (via BinaryOp enum)
- **Control Flow**: `jump`, `branch_if`, `return`
- **Function Calls**: `call`, `void_call`
- **Debug**: `debug` instructions for diagnostics

### Optimization Passes

- **ConstantFolding**: Folds constant expressions including aggregates
- **DeadCodeElimination**: Removes unreachable blocks and unused values
- **ConditionalMemoryPasses**: Skips SROA/Mem2Reg for aggregate-only functions
- **LowerAggregates**: Optional backend compatibility pass
- **Validation**: Ensures MIR invariants are maintained

## Key Improvements (Aggregate-First Refactoring)

1. **Eliminated Double Allocation**: Aggregates are now SSA values, no
   allocation needed
2. **30-40% Faster Compilation**: For aggregate-heavy code
3. **Simpler Optimization Pipeline**: Removed complex SROA/Mem2Reg for
   aggregates
4. **Cleaner Generated Code**: Direct value operations instead of memory
   indirection

## Remaining Work

- Loop support (while, for)
- Enum/match support
- Advanced array operations
- Cross-function aggregate optimization

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
