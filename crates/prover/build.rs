use std::path::Path;
use std::{env, fs};

use zkhash::ark_ff::PrimeField;
use zkhash::poseidon2::poseidon2_instance_m31::{MAT_DIAG16_M_1, RC16};

const T: usize = 16;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 14;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("poseidon2_constants.rs");

    let mut content = String::new();

    // Generate EXTERNAL_ROUND_CONSTS
    content.push_str("// Auto-generated constants for Poseidon2 hash\n");

    content.push_str(&format!(
        "pub const EXTERNAL_ROUND_CONSTS: [[M31; {}]; {}] = [\n",
        T, FULL_ROUNDS
    ));

    // First half of full rounds
    for round in 0..FULL_ROUNDS / 2 {
        content.push_str("    [");
        for i in 0..T {
            let value = RC16[round][i].into_bigint().0[0] as u32;
            content.push_str(&format!("M31::from_u32_unchecked({})", value));
            if i < T - 1 {
                content.push_str(", ");
            }
        }
        content.push_str("],\n");
    }

    // Second half of full rounds
    for round in 0..FULL_ROUNDS / 2 {
        let rc_index = PARTIAL_ROUNDS + FULL_ROUNDS / 2 + round;
        content.push_str("    [");
        for i in 0..T {
            let value = RC16[rc_index][i].into_bigint().0[0] as u32;
            content.push_str(&format!("M31::from_u32_unchecked({})", value));
            if i < T - 1 {
                content.push_str(", ");
            }
        }
        content.push_str("],\n");
    }

    content.push_str("];\n\n");

    // Generate INTERNAL_ROUND_CONSTS
    content.push_str(&format!(
        "pub const INTERNAL_ROUND_CONSTS: [M31; {}] = [\n",
        PARTIAL_ROUNDS
    ));
    for round in 0..PARTIAL_ROUNDS {
        let rc_index = FULL_ROUNDS / 2 + round;
        let value = RC16[rc_index][0].into_bigint().0[0] as u32;
        content.push_str(&format!("    M31::from_u32_unchecked({}),\n", value));
    }
    content.push_str("];\n\n");

    // Generate INTERNAL_MATRIX
    content.push_str(&format!("pub const INTERNAL_MATRIX: [M31; {}] = [\n", T));
    for i in 0..T {
        let value = MAT_DIAG16_M_1[i].into_bigint().0[0] as u32;
        content.push_str(&format!("    M31::from_u32_unchecked({}),\n", value));
    }
    content.push_str("];\n");

    fs::write(&dest_path, content).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
