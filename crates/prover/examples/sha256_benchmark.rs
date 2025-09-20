use std::time::Instant;

use cairo_m_prover::adapter::SHA256HashInput;
use cairo_m_prover::sha256::prover_sha256::prove_sha256;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

// You can adjust this value to test different sizes
// Start with 2^10 and increase as needed up to 2^18
const LOG_NUM_HASHES: u32 = 18; // 2^18 = 262,144 hashes as requested
const NUM_HASHES: usize = 1 << LOG_NUM_HASHES;

fn generate_sha256_inputs(num_hashes: usize) -> Vec<SHA256HashInput> {
    (0..num_hashes)
        .map(|i| {
            // Create a simple 64-byte message for each hash
            // Fill with pattern based on index to ensure variety
            let mut message = [M31::zero(); 32];
            for j in 0..32 {
                message[j] = M31::from((i + j) as u32 & 0xFFFF);
            }
            message
        })
        .collect()
}

fn main() {
    println!("=== SHA256 Benchmark ===");
    println!(
        "Generating 2^{} = {} SHA256 inputs...",
        LOG_NUM_HASHES, NUM_HASHES
    );

    let inputs = generate_sha256_inputs(NUM_HASHES);

    println!("Starting proof generation for {} hashes...", NUM_HASHES);
    println!("This may take several minutes...");
    let start_time = Instant::now();

    let _proof = prove_sha256::<Blake2sMerkleChannel>(&inputs, None).expect("Proving failed");

    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();

    // Calculate hashing frequency
    let hashes_per_second = NUM_HASHES as f64 / elapsed_secs;

    println!("\n=== Results ===");
    println!("Number of hashes: {}", NUM_HASHES);
    println!("Total time: {:.2} seconds", elapsed_secs);
    println!("Hashing frequency: {:.0} hashes/second", hashes_per_second);
    println!(
        "Time per hash: {:.3} ms",
        (elapsed_secs * 1000.0) / NUM_HASHES as f64
    );

    // Additional metrics
    println!("\nThroughput metrics:");
    println!("  {:.2} hashes/ms", hashes_per_second / 1000.0);
    println!("  {:.2} Khashes/s", hashes_per_second / 1000.0);
    if hashes_per_second > 1_000_000.0 {
        println!("  {:.2} Mhashes/s", hashes_per_second / 1_000_000.0);
    }
}
