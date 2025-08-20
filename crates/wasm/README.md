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
with optimized ValueIds (0, 1, 2...) ✅ **Complete**: Control flow operations
(`if`, `else`, `br`, `br_if`, `br_if_zero`) ✅ **Complete**: Comparison
operations (`i32.eq`, `i32.ne`, `i32.lt`, `i32.gt`, etc.) ✅ **Complete**: Local
variable operations (`local.get`, `local.set`, `local.tee`) ✅ **Complete**:
Basic function calls with parameter passing ✅ **Complete**: Loop structures
with proper scoping and variable management ✅ **Complete**: Nested loops with
complex control flow structures ✅ **Complete**: Comprehensive test suite with
snapshot testing 🚧 **In Progress**: Advanced control flow (`br_table`, complex
nested constructs) 🚧 **In Progress**: Memory operations (`load`, `store`)

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
- **Local Variables**: `local.get`, `local.set`, `local.tee`
- **Function Parameters**: Parameter handling with optimized ValueId allocation
  (0, 1, 2...)
- **Function Calls**: Call operations with proper parameter passing
- **Comparison Operations**: `i32.eq`, `i32.ne`, `i32.lt`, `i32.gt`, `i32.le`,
  `i32.ge`
- **Control Flow**: `if`, `else`, `br`, `br_if`, `br_if_zero`
- **Loops**: Full nested loop support with proper header/body separation and
  variable scoping
- **Value Passing**: Robust label parameter handling via pre-allocated slots

### Not Yet Supported

- **Advanced Control Flow**: `br_table`, complex nested constructs
- **Memory Operations**: `load`, `store`
- **Advanced Operations**: Floating-point operations, SIMD instructions

## Architecture

The crate is organized into two main modules:

1. **`loader`**: Handles WASM file loading and parsing into WOMIR's BlockLess
   DAG representation
2. **`flattening`**: Converts the WOMIR DAG to Cairo-M MIR using a two-pass
   algorithm

### Recent Improvements

**Loop Implementation & Optimization (Latest)**: Major improvements to control
flow and value management:

- **Complete Loop Support**: Full implementation of WASM loops with proper
  header/body separation
- **Optimized Parameter Allocation**: Function parameters now get ValueIds 0, 1,
  2... for cleaner MIR output
- **Complete Nested Loop Support**: Full support for complex nested loop
  structures with proper scoping and variable management
- **Proper Scope Management**: Loop sub-DAGs get their own value scopes to avoid
  ValueOrigin collisions
- **Pre-allocated Header Slots**: Loop variables are allocated during the first
  pass for consistent handling
- **Eliminated Redundant Copies**: Streamlined value flow by removing
  unnecessary intermediate mappings

**Control Flow Implementation**: The control flow system uses a robust memory
slot-based approach:

- **Memory Slot System**: Each label parameter gets dedicated memory slots for
  reliable value passing
- **Two-Pass Algorithm**: Preallocate blocks and slots, then generate
  instructions with proper value flow
- **Clean Value Management**: Proper scoping ensures no conflicts between
  different control flow contexts

The conversion process follows these steps:

1. Load WASM bytecode using `wasmparser`
2. Convert to WOMIR's BlockLess DAG representation
3. Apply a two-pass algorithm to generate MIR:
   - Pass 1: Preallocate function parameters (ValueIds 0,1,2...), basic blocks
     for labels, and loop header slots
   - Pass 2: Generate instructions and control flow with proper scoping and
     value management

### Control Flow Implementation

The control flow system uses a **memory slot-based approach** for joining values
at labels:

- **Memory Slots**: Each label parameter gets a dedicated stack slot (spill
  slot)
- **Predecessor Storage**: Each predecessor stores its values to the label's
  slots before branching
- **Label Entry Loading**: At label entry, values are loaded from slots into the
  label's output variables

This approach provides:

- **Simplicity**: Straightforward store-then-load pattern
- **Robustness**: No complex phi-node logic or value coalescing
- **Reliability**: Explicit memory operations ensure correct value flow

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
- `if_statement.wasm` - Conditional branching with robust control flow
- `simple_if.wasm` - Basic if-else control flow
- `fib.wasm` - Recursive Fibonacci (complex control flow with loops)
- `variables.wasm` - Local variable operations
- `simple_loop.wasm` - Basic loop structure with header/body separation
- `nested_loop.wasm` - Complex nested loop structures with proper scoping
- `select.wasm` - Select operation handling

## Dependencies

- **`womir`**: WASM parsing and BlockLess DAG representation
- **`cairo-m-compiler-mir`**: Cairo-M MIR types and utilities
- **`wasmparser`**: Low-level WASM parsing
- **`ouroboros`**: Self-referencing struct support for lifetime management

## Future Enhancements

- **Advanced Control Flow**: `br_table` operations and complex nested constructs
- **Memory Operations**: Load/store operations with proper memory model
  integration
- **Extended Types**: Support for i64, f32, f64, and vector types
- **Memory Model**: Integration with Cairo-M's memory management system
- **Optimization Passes**:
  - Eliminate redundant slot allocations for single-predecessor labels
  - Dead code elimination in MIR representation
  - Loop optimization and unrolling
- **Integration**: Full integration with Cairo-M's compilation pipeline
- **Performance**: Further optimize memory slot allocation and reduce memory
  traffic
- **Advanced Nested Constructs**: Support for even more complex nested
  structures if needed
