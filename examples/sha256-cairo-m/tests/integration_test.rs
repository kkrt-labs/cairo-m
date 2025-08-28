use cairo_m_common::{CairoMValue, InputValue, Program};
use cairo_m_compiler::{compile_cairo, CompilerOptions};
use cairo_m_runner::{run_cairo_program, RunnerOptions};
use once_cell::sync::Lazy;
use proptest::prelude::*;
use sha2::{Digest, Sha256};
use std::convert::TryInto;

// ==================================================================================
// COMPILED PROGRAM (SHARED ACROSS ALL TESTS)
// ==================================================================================

static COMPILED_PROGRAM: Lazy<Program> = Lazy::new(|| {
    let source =
        std::fs::read_to_string("src/sha256.cm").expect("Failed to read sha256.cm source file");
    let compiled_output = compile_cairo(source, "src/".to_string(), CompilerOptions::default())
        .expect("Failed to compile Cairo-M program");
    compiled_output.program.as_ref().clone()
});

// ==================================================================================
// MACROS FOR TESTING
// ==================================================================================

/// Asserts that a Cairo program returning a [u32; 8] array matches an expected slice.
macro_rules! assert_cairo_array_result {
    ($program:expr, $entrypoint:expr, $args:expr, $expected:expr) => {{
        let result = run_cairo_program($program, $entrypoint, &$args, RunnerOptions::default())
            .expect("Failed to run Cairo program");
        assert_eq!(result.return_values.len(), 1, "Expected one return value");
        let cairo_result_vec = match &result.return_values[0] {
            CairoMValue::Array(arr) => arr
                .iter()
                .map(|v| match v {
                    CairoMValue::U32(val) => *val,
                    _ => panic!("Expected U32 value in result array, found {:?}", v),
                })
                .collect::<Vec<u32>>(),
            _ => panic!("Expected Array return value"),
        };
        assert_eq!(
            cairo_result_vec.as_slice(),
            &$expected[..],
            "Cairo array result mismatch"
        );
    }};
}

/// Asserts that a Cairo program returning a single u32 matches an expected value.
macro_rules! assert_cairo_u32_result {
    ($program:expr, $entrypoint:expr, $args:expr, $expected:expr) => {{
        let result = run_cairo_program($program, $entrypoint, &$args, RunnerOptions::default())
            .expect("Failed to run Cairo program");
        assert_eq!(result.return_values.len(), 1, "Expected one return value");
        match &result.return_values[0] {
            CairoMValue::U32(val) => assert_eq!(*val, $expected, "u32 result mismatch"),
            other => panic!("Expected U32 return value, got {:?}", other),
        }
    }};
}

// ==================================================================================
// END-TO-END SHA-256 TESTS
// ==================================================================================
//
// Test coverage includes:
// - Standard test vectors (empty, "abc", known strings)
// - Edge cases (all zeros, all ones, special chars)
// - Boundary lengths (55/56/64 bytes for padding behavior)
// - Multi-chunk messages (up to MAX_CHUNKS)
// - Property-based tests for random inputs
//
// ==================================================================================

/// Maximum number of 512-bit chunks supported by the Cairo implementation
const MAX_CHUNKS: usize = 2;

/// Fixed buffer size in u32 words: (2 chunks * 64 bytes/chunk) / 4 bytes/word = 32 words
const PADDED_BUFFER_U32_SIZE: usize = (MAX_CHUNKS * 64) / 4;

/// Helper function to run a SHA-256 test with the given message
fn test_sha256(msg: &[u8]) {
    let (padded_buffer, num_chunks) = prepare_sha256_input(msg);
    let args = vec![
        InputValue::List(padded_buffer),
        InputValue::Number(num_chunks as i64),
    ];
    let expected = rust_sha256(msg);
    assert_cairo_array_result!(&COMPILED_PROGRAM, "sha256_hash", args, expected);
}

/// Computes the SHA-256 hash using a trusted Rust implementation.
fn rust_sha256(msg: &[u8]) -> [u32; 8] {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    let result_bytes: [u8; 32] = hasher.finalize().into();

    let mut result_words = [0u32; 8];
    result_bytes
        .chunks_exact(4)
        .enumerate()
        .for_each(|(i, chunk)| {
            result_words[i] = u32::from_be_bytes(chunk.try_into().expect("Chunk size mismatch"));
        });

    result_words
}

/// Prepares a message for the Cairo-M SHA256 function by padding it and
/// converting it to a fixed-size buffer of u32 words.
fn prepare_sha256_input(msg: &[u8]) -> (Vec<InputValue>, usize) {
    // Perform standard SHA-256 padding
    let mut padded_bytes = msg.to_vec();
    padded_bytes.push(0x80);

    // Pad to 56 bytes (448 bits) within the last chunk
    while padded_bytes.len() % 64 != 56 {
        padded_bytes.push(0x00);
    }

    // Append message length as 64-bit big-endian
    let bit_len = (msg.len() as u64) * 8;
    padded_bytes.extend_from_slice(&bit_len.to_be_bytes());

    let num_chunks = padded_bytes.len() / 64;
    assert!(
        num_chunks <= MAX_CHUNKS,
        "Message requires {} chunks but only {} are supported",
        num_chunks,
        MAX_CHUNKS
    );

    // Convert bytes to u32 words (big-endian)
    let mut padded_words: Vec<u32> = padded_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().expect("Chunk size mismatch")))
        .collect();

    // Pad to fixed buffer size
    padded_words.resize(PADDED_BUFFER_U32_SIZE, 0);

    // Convert to InputValue format
    let input_values = padded_words
        .into_iter()
        .map(|word| InputValue::Number(i64::from(word)))
        .collect();

    (input_values, num_chunks)
}

// Note: Each SHA-256 test case is kept as a separate test function rather than
// combining them into a single test. This provides:
// - Better test reporting: failures are isolated to specific test cases
// - Parallel execution: tests can run concurrently for faster execution
// - Selective running: individual tests can be run with `cargo test <test_name>`
// - Test isolation: prevents cascading failures between test cases

// === Standard test vectors ===

#[test]
fn test_sha256_empty_string() {
    test_sha256(b"");
}

#[test]
fn test_sha256_abc() {
    test_sha256(b"abc");
}

#[test]
fn test_sha256_long_message_two_chunks() {
    // cspell:disable-next-line
    test_sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
}

#[test]
fn test_sha256_single_character() {
    test_sha256(b"a");
}

#[test]
fn test_sha256_alphabet_lowercase() {
    test_sha256(b"abcdefghijklmnopqrstuvwxyz");
}

#[test]
fn test_sha256_alphanumeric() {
    test_sha256(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");
}

#[test]
fn test_sha256_message_digest() {
    test_sha256(b"message digest");
}

#[test]
fn test_sha256_quick_brown_fox() {
    test_sha256(b"The quick brown fox jumps over the lazy dog");
}

// === Edge cases and patterns ===

#[test]
fn test_sha256_repeated_pattern() {
    // Tests fixed repeated pattern - complements proptest's random repeated patterns
    test_sha256(b"aaaaaaaaaa"); // 10 'a's
}

#[test]
fn test_sha256_binary_pattern() {
    test_sha256(b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f");
}

#[test]
fn test_sha256_all_zeros() {
    test_sha256(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00");
}

#[test]
fn test_sha256_all_ones() {
    test_sha256(b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff");
}

// === Boundary length tests (padding behavior) ===

#[test]
fn test_sha256_55_bytes() {
    // 55 bytes - just under the boundary where padding extends to next block
    test_sha256(b"1234567890123456789012345678901234567890123456789012345");
}

#[test]
fn test_sha256_56_bytes() {
    // 56 bytes - exactly at the boundary where padding extends to next block
    test_sha256(b"12345678901234567890123456789012345678901234567890123456");
}

#[test]
fn test_sha256_64_bytes() {
    // 64 bytes - exactly one block before padding
    test_sha256(b"1234567890123456789012345678901234567890123456789012345678901234");
}

#[test]
fn test_sha256_special_characters() {
    test_sha256(b"!@#$%^&*()_+-=[]{}|;':\",./<>?");
}

#[test]
fn test_sha256_unicode_utf8() {
    test_sha256("Hello, World! 🌍".as_bytes());
}

// Property-based testing for SHA-256
proptest! {
    #[test]
    fn test_sha256_random_inputs(input in prop::collection::vec(any::<u8>(), 0..MAX_CHUNKS*64)) {
        // SHA-256 padding adds at least 1 byte (0x80) and 8 bytes (64-bit length).
        // If the message length % 64 > 55, padding extends to the next block.
        // For MAX_CHUNKS=2:
        // - Messages up to 55 bytes -> 1 chunk after padding
        // - Messages 56-119 bytes -> 2 chunks after padding
        // - Messages 120+ bytes -> 3+ chunks (exceeds our limit)
        if input.len() <= (MAX_CHUNKS - 1) * 64 + 55 {
            test_sha256(&input);
        }
    }

    #[test]
    fn test_sha256_random_boundary_inputs(len in 50..70usize) {
        // Test around the 55/56 byte boundary where padding behavior changes
        let input: Vec<u8> = (0..len).map(|i| (i & 0xFF) as u8).collect();
        if input.len() <= (MAX_CHUNKS - 1) * 64 + 55 {
            test_sha256(&input);
        }
    }

    #[test]
    fn test_sha256_repeated_byte_patterns(byte in any::<u8>(), count in 1..100usize) {
        // Test repeated byte patterns
        let input = vec![byte; count];
        if input.len() <= (MAX_CHUNKS - 1) * 64 + 55 {
            test_sha256(&input);
        }
    }

    #[test]
    #[should_panic(expected = "Message requires")]
    fn test_sha256_exceeds_chunk_limit(len in 120..=500usize) {
        // Test messages that exceed MAX_CHUNKS limit
        // 120+ bytes require 3+ chunks after padding (exceeds our MAX_CHUNKS=2)
        let input: Vec<u8> = (0..len).map(|i| (i & 0xFF) as u8).collect();
        test_sha256(&input);
    }
}

// ==================================================================================
// UNIT TESTS FOR HELPER FUNCTIONS
// ==================================================================================

#[cfg(test)]
mod helpers {
    use super::*;

    /// SHA-256 rotate right operation
    #[inline]
    fn rust_rotr(x: u32, n: u32) -> u32 {
        x.rotate_right(n)
    }

    /// SHA-256 Σ₀ (big sigma 0) function
    #[inline]
    fn rust_big_sigma0(x: u32) -> u32 {
        x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
    }

    /// SHA-256 Σ₁ (big sigma 1) function
    #[inline]
    fn rust_big_sigma1(x: u32) -> u32 {
        x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
    }

    /// SHA-256 σ₀ (small sigma 0) function
    #[inline]
    fn rust_small_sigma0(x: u32) -> u32 {
        x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
    }

    /// SHA-256 σ₁ (small sigma 1) function
    #[inline]
    fn rust_small_sigma1(x: u32) -> u32 {
        x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
    }

    /// SHA-256 Ch (choice) function
    #[inline]
    fn rust_ch(x: u32, y: u32, z: u32) -> u32 {
        (x & y) ^ (!x & z)
    }

    /// SHA-256 Maj (majority) function
    #[inline]
    fn rust_maj(x: u32, y: u32, z: u32) -> u32 {
        (x & y) ^ (x & z) ^ (y & z)
    }

    /// Test a single-argument SHA-256 helper function
    fn test_unary_function(x: u32, function_name: &str, expected: u32) {
        let args = vec![InputValue::Number(x as i64)];
        assert_cairo_u32_result!(&COMPILED_PROGRAM, function_name, args, expected);
    }

    /// Test a three-argument SHA-256 helper function
    fn test_ternary_function(x: u32, y: u32, z: u32, function_name: &str, expected: u32) {
        let args = vec![
            InputValue::Number(x as i64),
            InputValue::Number(y as i64),
            InputValue::Number(z as i64),
        ];
        assert_cairo_u32_result!(&COMPILED_PROGRAM, function_name, args, expected);
    }

    proptest! {
        #[test]
        fn test_rotr(x: u32, n in 0u32..32) {
            let args = vec![InputValue::Number(x as i64), InputValue::Number(n as i64)];
            let expected = rust_rotr(x, n);
            assert_cairo_u32_result!(&COMPILED_PROGRAM, "rotr", args, expected);
        }

        #[test]
        fn test_big_sigma0(x: u32) {
            test_unary_function(x, "big_sigma0", rust_big_sigma0(x));
        }

        #[test]
        fn test_big_sigma1(x: u32) {
            test_unary_function(x, "big_sigma1", rust_big_sigma1(x));
        }

        #[test]
        fn test_small_sigma0(x: u32) {
            test_unary_function(x, "small_sigma0", rust_small_sigma0(x));
        }

        #[test]
        fn test_small_sigma1(x: u32) {
            test_unary_function(x, "small_sigma1", rust_small_sigma1(x));
        }

        #[test]
        fn test_ch(x: u32, y: u32, z: u32) {
            test_ternary_function(x, y, z, "ch", rust_ch(x, y, z));
        }

        #[test]
        fn test_maj(x: u32, y: u32, z: u32) {
            test_ternary_function(x, y, z, "maj", rust_maj(x, y, z));
        }
    }
}
