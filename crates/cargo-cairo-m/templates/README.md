# {{name}}

A Cairo-M project with integrated Rust testing.

## Project Structure

- `cairom.toml` - Cairo-M project manifest file
- `src/` - Cairo-M source files
  - `fibonacci.cm` - Example fibonacci implementation
- `tests/` - Rust integration tests
- `Cargo.toml` - Rust project configuration

## Prerequisites

### macOS Users

You need to have LLVM installed:

```bash
brew install llvm
```

## Common Commands

### Run all tests

```bash
cargo test
```

### Run a specific test

```bash
cargo test test_fibonacci
```

### Show test output

```bash
cargo test -- --nocapture
```

Note: The required RUSTFLAGS are automatically configured in
`.cargo/config.toml`

## Adding New Cairo-M Files

1. Create a new `.cm` file in the `src/` directory
2. If needed, update the `entry_point` in `cairom.toml` to point to your main
   file
3. Write a corresponding test in `tests/`
4. Use `compile_cairo` with the source directory path (e.g., "src/")
5. Use `run_cairo_program` to execute your compiled Cairo-M code
6. Compare results with Rust reference implementations

## Resources

- [Cairo-M Documentation](https://github.com/kkrt-labs/cairo-m)
- [Cairo-M Language Reference](https://github.com/kkrt-labs/cairo-m/docs)
