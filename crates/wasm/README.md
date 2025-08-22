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
**Complete**: Support for arithmetic operations (Add, Sub, Mul, Div), constants,
and local variables ✅ **Complete**: Bitwise operations (`i32.and`, `i32.or`,
`i32.xor`) ✨ **NEW** ✅ **Complete**: Function parameter handling and return
values with optimized ValueIds (0, 1, 2...) ✅ **Complete**: Control flow
operations (`if`, `else`, `br`, `br_if`, `br_if_zero`) ✅ **Complete**:
Comparison operations (`i32.eq`, `i32.ne`, `i32.lt_u`, `i32.gt_u`, etc.) ✅
**Complete**: Local variable operations (`local.get`, `local.set`, `local.tee`)
✅ **Complete**: Basic function calls with parameter passing ✅ **Complete**:
Loop structures with proper scoping and variable management ✅ **Complete**:
Nested loops with complex control flow structures ✅ **Complete**: SSA form with
phi nodes for proper optimization compatibility ✨ **NEW** ✅ **Complete**:
Comprehensive test suite with snapshot testing 🚧 **In Progress**: Advanced
control flow (`br_table`, complex nested constructs) 🚧 **In Progress**: Memory
operations (`load`, `store`)

## Usage

### Command Line

```bash
# Convert WASM to compiled program and print to stdout
cargo run -- <path-to-wasm-file>

# Convert WASM to compiled program and save to file
cargo run -- <path-to-wasm-file> --output <output-file>

# Show only MIR without compiling to final program
cargo run -- <path-to-wasm-file> --mir-only

# Enable verbose output (shows loading and conversion progress)
cargo run -- <path-to-wasm-file> --verbose

# Examples
cargo run -- tests/test_cases/add.wasm
cargo run -- tests/test_cases/bitwise.wasm --verbose
cargo run -- tests/test_cases/arithmetic.wasm --mir-only
cargo run -- tests/test_cases/func_call.wasm --output program.json
```

#### Command Line Options

- `<WASM_FILE>`: Input WASM file to compile (required)
- `-o, --output <FILE>`: Output file to write the compiled program to
- `-v, --verbose`: Enable verbose output (shows MIR and progress information)
- `--mir-only`: Show only MIR without compiling to final program
- `-h, --help`: Show help information
- `-V, --version`: Show version information

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

- **Arithmetic**: `i32.add`, `i32.sub`, `i32.mul`, `i32.div_u`
- **Bitwise Operations**: `i32.and`, `i32.or`, `i32.xor` ✨ **NEW**
- **Constants**: `i32.const`
- **Local Variables**: `local.get`, `local.set`, `local.tee`
- **Function Parameters**: Parameter handling with optimized ValueId allocation
  (0, 1, 2...)
- **Function Calls**: Call operations with proper parameter passing
- **Comparison Operations**: `i32.eq`, `i32.ne`, `i32.lt_u`, `i32.gt_u`,
  `i32.le_u`, `i32.ge_u`
- **Control Flow**: `if`, `else`, `br`, `br_if`, `br_if_zero`
- **Loops**: Full nested loop support with proper header/body separation and
  variable scoping ✅ **COMPLETE**
- **Value Passing**: SSA form with phi nodes for proper dataflow analysis

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

**SSA Form with Phi Nodes (Latest)** ✨ **NEW**: Major architectural upgrade for
better optimization compatibility:

- **Phi Node Implementation**: Replaced slot-based system with proper SSA form
  using phi nodes
- **Proper Control Flow**: Labels and loop headers now use phi nodes for value
  merging
- **Optimization Ready**: Compatible with standard compiler optimization passes
- **Deferred Phi Population**: Two-pass algorithm creates empty phi nodes, then
  populates operands
- **Better Type Safety**: Improved value flow tracking and type consistency

**Bitwise Operations Support** ✨ **NEW**: Extended instruction set coverage:

- **Complete Bitwise Set**: Support for `i32.and`, `i32.or`, `i32.xor`
  operations
- **Optimized Binary Operation Handling**: Streamlined conversion from WASM to
  MIR binary operations
- **Test Coverage**: Comprehensive test cases for bitwise operations

**Loop Implementation & Optimization**: Major improvements to control flow and
value management:

- **Complete Loop Support**: Full implementation of WASM loops with proper
  header/body separation
- **Complete Nested Loop Support**: Full support for complex nested loop
  structures with proper scoping
- **Phi-based Loop Variables**: Loop-carried values now use phi nodes in loop
  headers
- **Proper Scope Management**: Loop sub-DAGs get their own value scopes to avoid
  ValueOrigin collisions

The conversion process follows these steps:

1. Load WASM bytecode using `wasmparser`
2. Convert to WOMIR's BlockLess DAG representation
3. Apply a two-pass algorithm to generate SSA form MIR:
   - Pass 1: Preallocate function parameters (ValueIds 0,1,2...), basic blocks
     for labels, and create empty phi nodes for control flow merge points
   - Pass 2: Generate instructions and control flow, recording deferred phi
     operands
   - Pass 3: Finalize phi nodes by populating them with their collected operands

### Control Flow Implementation

The control flow system uses **SSA form with phi nodes** for joining values at
labels:

- **Phi Nodes**: Each label parameter gets a phi node that merges values from
  different predecessors
- **Deferred Population**: Phi nodes are created empty during block allocation,
  then populated with operands
- **Proper SSA Form**: Values maintain single-assignment property with explicit
  control flow dependencies
- **Loop Headers**: Loop-carried values use phi nodes in loop headers for proper
  dataflow representation

This approach provides:

- **Optimization Compatibility**: Standard SSA form works with existing compiler
  optimization passes
- **Better Analysis**: Explicit dataflow dependencies enable better program
  analysis
- **Type Safety**: Proper value flow tracking ensures type consistency
- **Standards Compliance**: Follows established compiler design patterns

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
- `bitwise.wasm` - Bitwise operations (AND, OR, XOR) ✨ **NEW**
- `func_call.wasm` - Function call handling
- `if_statement.wasm` - Conditional branching with robust control flow
- `simple_if.wasm` - Basic if-else control flow
- `fib.wasm` - Recursive Fibonacci (complex control flow with loops)
- `variables.wasm` - Local variable operations
- `simple_loop.wasm` - Basic loop structure with header/body separation
- `nested_loop.wasm` - Complex nested loop structures with proper scoping ✅
  **COMPLETE**

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
- **Additional Bitwise Operations**: Shift operations (`shl`, `shr_u`, `shr_s`)
  and remainder operations
- **Optimization Passes**:
  - Leverage SSA form for advanced optimizations
  - Dead code elimination and constant folding
  - Loop optimization and unrolling
- **Integration**: Full integration with Cairo-M's compilation pipeline
- **Performance**: Further optimize phi node allocation and SSA construction
- **Advanced Type System**: Enhanced type checking and inference
