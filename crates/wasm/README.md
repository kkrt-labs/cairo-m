# Cairo-M WASM Frontend

A complete WASM frontend for the Cairo-M compiler that converts WASM bytecode to
Cairo-M's MIR (Mid-level Intermediate Representation) using
[WOMIR](https://github.com/powdr-labs/womir).

## Features

- **WASM Loading**: Parse WASM files into WOMIR's BlockLess DAG representation
- **MIR Conversion**: Convert WASM bytecode to Cairo-M MIR for integration with
  the compiler pipeline
- **Comprehensive Testing**: Extensive test coverage with snapshot testing for
  both loading and conversion
- **CLI Tool**: Command-line interface for quick WASM analysis and conversion

## Current Status

✅ **Complete**: Load and parse WASM files into WOMIR BlockLess DAG format ✅
**Complete**: Convert basic WASM operations to Cairo-M MIR instructions ✅
**Complete**: Support for arithmetic operations (Add, Sub, Mul), constants, and
local variables ✅ **Complete**: Function parameter handling and return values
✅ **Testing**: Comprehensive test suite with snapshot testing 🚧 **In
Progress**: Control flow, comparisons, and advanced operations

## Usage

### Command Line

```bash
# Convert WASM to MIR and display the result
cargo run -- <path-to-wasm-file>

# Examples
cargo run -- tests/test_cases/add.wasm
cargo run -- tests/test_cases/arithmetic.wasm
cargo run -- tests/test_cases/func_call.wasm
```

### Library - WASM Loading

```rust
use cairo_m_wasm::loader::BlocklessDagModule;

// Load WASM file into WOMIR representation
let module = BlocklessDagModule::from_file("path/to/file.wasm")?;

// Display the parsed WASM structure
println!("{}", module);

// Access function count and details
let function_count = module.with_program(|program| program.functions.len());
println!("Functions: {}", function_count);
```

### Library - WASM to MIR Conversion

```rust
use cairo_m_wasm::{
    loader::BlocklessDagModule,
    flattening::WasmModuleToMir,
};
use cairo_m_compiler_mir::PrettyPrint;

// Load WASM and convert to MIR
let module = BlocklessDagModule::from_file("path/to/file.wasm")?;
let mir_module = WasmModuleToMir::new(module).to_mir()?;

// Pretty print the MIR
println!("{}", mir_module.pretty_print(0));
```

## Example Output

### WASM Loading

```text
add:
  Node { operation: Inputs, inputs: [], output_types: [I32, I32] }
  Node { operation: WASMOp(I32Add), inputs: [ValueOrigin { node: 0, output_idx: 1 }, ValueOrigin { node: 0, output_idx: 0 }], output_types: [I32] }
  Node { operation: Br(BreakTarget { depth: 0, kind: FunctionOrLoop }), inputs: [ValueOrigin { node: 1, output_idx: 0 }], output_types: [] }
```

### MIR Conversion

```text
module {
  // Function 0
  fn add {
    parameters: [0, 1]
    entry: 0

    0:
      %2 = %1 Add %0
      return %2
  }
}
```

## Supported WASM Operations

- **Arithmetic**: `i32.add`, `i32.sub`, `i32.mul`
- **Constants**: `i32.const`
- **Local Variables**: `local.get`, `local.set`, `local.tee` (partial)
- **Function Parameters**: Parameter handling and returns
- **Basic Function Calls**: Simple call operations (experimental)

### Not Yet Supported

- **Comparison Operations**: `i32.eq`, `i32.ne`, `i32.lt`, etc.
- **Control Flow**: `if`, `else`, `br`, `br_if`, `select`
- **Memory Operations**: `load`, `store`
- **Advanced Operations**: Loops, nested blocks, floating-point operations

## Architecture

The crate is organized into two main modules:

1. **`loader`**: Handles WASM file loading and parsing into WOMIR's BlockLess
   DAG representation
2. **`flattening`**: Converts the WOMIR DAG to Cairo-M MIR using a two-pass
   algorithm

The conversion process follows these steps:

1. Load WASM bytecode using `wasmparser`
2. Convert to WOMIR's BlockLess DAG representation
3. Apply a two-pass algorithm to generate MIR:
   - Pass 1: Create basic blocks for all labels
   - Pass 2: Generate instructions and control flow

## Testing

```bash
# Run all tests
cargo test

# Run with snapshot testing
cargo insta test

# Review snapshot changes
cargo insta review
```

Test cases include:

- `add.wasm` - Simple addition function
- `arithmetic.wasm` - Complex arithmetic expressions
- `func_call.wasm` - Function call handling
- `if_statement.wasm` - Conditional branching
- `fib.wasm` - Recursive Fibonacci (more complex control flow)
- `var_manipulation.wasm` - Local variable operations

## Dependencies

- **`womir`**: WASM parsing and BlockLess DAG representation
- **`cairo-m-compiler-mir`**: Cairo-M MIR types and utilities
- **`wasmparser`**: Low-level WASM parsing
- **`ouroboros`**: Self-referencing struct support for lifetime management

## Future Enhancements

- Support for more WASM operations (memory operations, floating-point, etc.)
- Advanced control flow constructs (loops, nested blocks)
- Memory model integration
- Optimization passes in the MIR representation
- Integration with Cairo-M's full compilation pipeline
