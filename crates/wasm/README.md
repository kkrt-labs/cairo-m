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

| Feature                   | Supported | Notes                                        |
| ------------------------- | --------- | -------------------------------------------- |
| **WASM Types**            |           |                                              |
| `i32`                     | ✅        | Partial support, maps to `MirType::U32`      |
| `i64`                     | ❌        | Not yet implemented                          |
| `f32`                     | ❌        | Not yet implemented                          |
| `f64`                     | ❌        | Not yet implemented                          |
| Vector types              | ❌        | Not yet implemented                          |
| **Arithmetic Operations** |           |                                              |
| `i32.add`                 | ✅        | Maps to `BinaryOp::U32Add`                   |
| `i32.sub`                 | ✅        | Maps to `BinaryOp::U32Sub`                   |
| `i32.mul`                 | ✅        | Maps to `BinaryOp::U32Mul`                   |
| `i32.div_u`               | ✅        | Maps to `BinaryOp::U32Div`                   |
| `i32.div_s`               | ❌        | Not yet implemented                          |
| `i32.rem_u`               | ❌        | Not yet implemented                          |
| `i32.rem_s`               | ❌        | Not yet implemented                          |
| **Bitwise Operations**    |           |                                              |
| `i32.and`                 | ✅        | Maps to `BinaryOp::U32BitwiseAnd`            |
| `i32.or`                  | ✅        | Maps to `BinaryOp::U32BitwiseOr`             |
| `i32.xor`                 | ✅        | Maps to `BinaryOp::U32BitwiseXor`            |
| `i32.shl`                 | ❌        | TODO: bit shifts, rotations, u8 operations   |
| `i32.shr_u`               | ❌        | TODO: bit shifts, rotations, u8 operations   |
| `i32.shr_s`               | ❌        | TODO: bit shifts, rotations, u8 operations   |
| `i32.rotl`                | ❌        | TODO: bit shifts, rotations, u8 operations   |
| `i32.rotr`                | ❌        | TODO: bit shifts, rotations, u8 operations   |
| **Comparison Operations** |           |                                              |
| `i32.eq`                  | ✅        | Maps to `BinaryOp::U32Eq`                    |
| `i32.ne`                  | ✅        | Maps to `BinaryOp::U32Neq`                   |
| `i32.lt_u`                | ✅        | Maps to `BinaryOp::U32Greater`               |
| `i32.gt_u`                | ✅        | Maps to `BinaryOp::U32Greater`               |
| `i32.le_u`                | ✅        | Maps to `BinaryOp::U32GreaterEqual`          |
| `i32.ge_u`                | ✅        | Maps to `BinaryOp::U32GreaterEqual`          |
| `i32.lt_s`                | ✅        | Maps to 3 opcodes                            |
| `i32.gt_s`                | ✅        | Maps to 3 opcodes                            |
| `i32.le_s`                | ✅        | Maps to 3 opcodes                            |
| `i32.ge_s`                | ✅        | Maps to 3 opcodes                            |
| **Constants**             |           |                                              |
| `i32.const`               | ✅        | Full support for i32 constants               |
| **Local Variables**       |           |                                              |
| `local.get`               | ✅        | Handled by WOMIR preprocessing               |
| `local.set`               | ✅        | Handled by WOMIR preprocessing               |
| `local.tee`               | ✅        | Handled by WOMIR preprocessing               |
| **Global Variables**      |           |                                              |
| `global.get`              | ❌        | Not yet implemented                          |
| `global.set`              | ❌        | Not yet implemented                          |
| **Function Operations**   |           |                                              |
| Function parameters       | ✅        | Full support with proper type mapping        |
| Function return values    | ✅        | Full support with proper type mapping        |
| Function calls            | ✅        | `call` instruction with signature support    |
| **Control Flow**          |           |                                              |
| `if` / `else`             | ✅        | Full conditional branching support           |
| `br`                      | ✅        | Unconditional branch with value passing      |
| `br_if`                   | ✅        | Conditional branch with value passing        |
| `br_if_zero`              | ✅        | Inverted conditional branch                  |
| `br_table`                | ❌        | `todo!()` - not yet implemented              |
| **Loop Structures**       |           |                                              |
| Basic loops               | ✅        | Full loop header/body/exit support           |
| Nested loops              | ✅        | Complete nested loop support with scoping    |
| Loop-carried values       | ✅        | Phi nodes for loop variables                 |
| Continue operations       | ✅        | Proper loop continuation handling            |
| **Memory Operations**     |           |                                              |
| `i32.load`                | ❌        | Not yet implemented                          |
| `i32.store`               | ❌        | Not yet implemented                          |
| `i32.load8_u`             | ❌        | Not yet implemented                          |
| `i32.store8`              | ❌        | Not yet implemented                          |
| **Advanced Features**     |           |                                              |
| SSA form                  | ✅        | Complete SSA with phi nodes                  |
| Phi nodes                 | ✅        | Proper control flow value merging            |
| Value scoping             | ✅        | Loop and function scope management           |
| Type safety               | ✅        | Full type checking and validation            |
| **WOMIR Integration**     |           |                                              |
| BlockLess DAG loading     | ✅        | Complete WOMIR integration                   |
| DAG to MIR conversion     | ✅        | Two-pass algorithm with SSA                  |
| Error handling            | ✅        | Comprehensive error reporting                |
| **Testing**               |           |                                              |
| Snapshot testing          | ✅        | Extensive test coverage                      |
| Test cases                | ✅        | 20+ test programs including complex examples |

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

# Run a specific function with arguments
cargo run -- <path-to-wasm-file> -f <function-name> -a <arg1> -a <arg2>
```

#### Command Line Options

- `<WASM_FILE>`: Input WASM file to compile (required)
- `-o, --output <FILE>`: Output file to write the compiled program to
- `-v, --verbose`: Enable verbose output (shows MIR and progress information)
- `--mir-only`: Show only MIR without compiling to final program
- `-f, --function <NAME>`: Function name to run after compilation (entrypoint)
- `-a, --arg <VALUE>`: Arguments to pass to the entrypoint (repeat -a for
  multiple args)
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

### Input WebAssembly

```WebAssembly
(module
  (type (;0;) (func (param i32) (result i32)))
  (func $simple_if (type 0) (param $x i32) (result i32)
    ;; Simple if statement: if x > 5, return x + 10, else return x
    local.get $x
    i32.const 5
    i32.gt_u

    if (result i32)
      ;; If x > 5, return x + 10
      local.get $x
      i32.const 10
      i32.add
    else
      ;; If x <= 5, return x as is
      local.get $x
    end
  )

  (export "simple_if" (func $simple_if))
)
```

### MIR Conversion (before phi node resolution and optimization passes)

```text
module {
  // Function 0
  fn simple_if {
    parameters: [0]
    entry: 0

    0:
      %2 = 5 (u32)
      %3 = %0 U32Greater %2
      if %3 then jump 1 else jump 3

    1:
      %4 = 10 (u32)
      %5 = %0 U32Add %4
      jump 2

    2:
      %1 = φ u32 { [%3]: %0, [%1]: %5 }
      jump 4

    3:
      jump 2

    4:
      return %1

  }

}

```

## What's next?

- Missing opcodes for i32
- Vector types
- End-to-end compilation from rust of fibonacci and SHA256
