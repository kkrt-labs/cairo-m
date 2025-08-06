# Cairo-M WASM Frontend

## Work in Progress

A WASM frontend for the Cairo-M compiler using
[WOMIR](https://github.com/powdr-labs/womir).

## Current Status

This crate can load WASM files, parse them into WOMIR, and display the parsed
content. It provides both a command-line tool and a library interface.

## Usage

### Command Line

```bash
cargo run -- <path-to-wasm-file>

# Example
cargo run -- tests/test_cases/add.wasm
```

### Library

```rust
use cairo_m_wasm::loader::{load_module, format_wasm_module};

let module = load_module("path/to/file.wasm")?;
println!("{}", format_wasm_module(&module));

let program = module.program()?;
println!("Functions: {}", program.functions.len());
```

## Example Output

```text
Function: add (5 directives)
    0: __func_0 [6]:
    1: add [6]:
    2:     I32Add $3 $2 -> $5
    3:     Copy $5 -> $4
    4:     Return $0 $1
```

## Future Plans

The goal is to integrate with the existing DAG code from the WOMIR project
rather than implementing custom WOMIR-to-Cairo-M translation. This will enable:

- Advanced analysis passes using WOMIR's DAG representation
- Integration with the main Cairo-M compiler pipeline
- Leveraging existing optimization and transformation capabilities

## Testing

```bash
cargo insta test
cargo insta review  # Review snapshot changes
```

Test cases include real WASM files that demonstrate loading and parsing
capabilities.
