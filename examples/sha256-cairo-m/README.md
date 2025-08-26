# SHA-256 Cairo-M Implementation

A complete SHA-256 cryptographic hash function implementation in Cairo-M with
comprehensive Rust integration tests.

## Project Structure

- `cairom.toml` - Cairo-M project manifest file
- `src/` - Cairo-M source files
  - `sha256.cm` - SHA-256 implementation
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
cargo test test_sha256_empty_string
```

### Show test output

```bash
cargo test -- --nocapture
```

Note: The required RUSTFLAGS are automatically configured in
`.cargo/config.toml`

## Implementation Details

### SHA-256 Algorithm

This implementation follows the NIST FIPS 180-4 specification for SHA-256:

- Processes messages in 512-bit (64-byte) chunks
- Uses 32-bit word operations compatible with Cairo-M's u32 type
- Implements all required bitwise operations (rotate, shift, XOR, AND)
- Supports messages up to 2 chunks (128 bytes) in the current configuration

### Cairo-M Features Used

- **u32 arithmetic**: Native support for 32-bit unsigned integer operations
- **Arrays**: Fixed-size arrays for message buffers and hash state
- **Control flow**: While loops for chunk processing and message scheduling
- **Bitwise operations**: XOR, AND, and custom rotate-right implementation

### Testing Approach

The test suite includes:

- **Unit tests**: Property-based testing for individual SHA-256 operations
  (rotr, sigma functions, Ch, Maj)
- **Integration tests**: Comprehensive test vectors including edge cases
- **Reference comparison**: All results verified against Rust's `sha2` crate

## Extending the Implementation

To modify the maximum message size:

1. Update `MAX_CHUNKS` in `integration_test.rs`
2. Adjust `PADDED_BUFFER_U32_SIZE` accordingly
3. Update the fixed array size in `sha256.cm` to match

## Resources

- [Cairo-M Documentation](https://github.com/kkrt-labs/cairo-m)
- [NIST FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf) -
  SHA-256 specification
