use std::convert::TryInto;
use std::fs;
use std::path::Path;

use cairo_m_common::{InputValue, Program};
use cairo_m_compiler::{CompilerOptions, compile_cairo};

/// Compile the SHA-256 Cairo-M example program.
pub fn compile_sha256() -> Program {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap();
    let source_path = format!(
        "{}/examples/sha256-cairo-m/src/sha256.cm",
        workspace_root.display()
    );
    let source_text = fs::read_to_string(&source_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", source_path));
    let options = CompilerOptions::default();
    let output =
        compile_cairo(source_text, source_path, options).expect("Failed to compile sha256.cm");
    (*output.program).clone()
}

/// Pad a 1024-byte message and convert it to an InputValue buffer for Cairo-M.
///
/// Returns the padded buffer and the number of 512-bit chunks.
pub fn prepare_sha256_input_1kb(msg: &[u8]) -> (Vec<InputValue>, usize) {
    let mut padded_bytes = msg.to_vec();
    padded_bytes.push(0x80);
    while padded_bytes.len() % 64 != 56 {
        padded_bytes.push(0x00);
    }
    let bit_len = (msg.len() as u64) * 8;
    padded_bytes.extend_from_slice(&bit_len.to_be_bytes());

    let num_chunks = padded_bytes.len() / 64;

    let mut padded_words: Vec<u32> = padded_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().expect("Chunk size mismatch")))
        .collect();
    // 1024-byte message -> 17 chunks after padding -> 272 u32 words
    padded_words.resize(272, 0);

    let input_values = padded_words
        .into_iter()
        .map(|w| InputValue::Number(i64::from(w)))
        .collect::<Vec<_>>();

    (input_values, num_chunks)
}
