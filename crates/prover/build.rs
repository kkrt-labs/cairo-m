use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// Include the shared parameters
include!("src/utils/poseidon/poseidon_params.rs");

fn main() {
    // Generate constants as raw u32 values (no M31 dependency)
    let round_constants =
        generate_round_constants_u32(T, FULL_ROUNDS, PARTIAL_ROUNDS, P, ALPHA, PRIME_BIT_LEN);
    let mds_matrix = generate_mds_matrix_u32(T);

    // Write to generated constants file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("poseidon_constants_generated.rs");
    let mut f = File::create(&dest_path).unwrap();

    writeln!(f, "// Generated Poseidon constants as raw u32 values").unwrap();
    writeln!(
        f,
        "// These are converted to M31 at runtime with minimal overhead"
    )
    .unwrap();
    writeln!(f).unwrap();

    // Write round constants
    writeln!(
        f,
        "pub const ROUND_CONSTANTS_U32: [u32; {}] = [",
        round_constants.len()
    )
    .unwrap();
    for (i, &constant) in round_constants.iter().enumerate() {
        if i % 8 == 0 {
            write!(f, "    ").unwrap();
        }
        write!(f, "{}", constant).unwrap();
        if i < round_constants.len() - 1 {
            write!(f, ", ").unwrap();
        }
        if (i + 1) % 8 == 0 || i == round_constants.len() - 1 {
            writeln!(f).unwrap();
        }
    }
    writeln!(f, "];").unwrap();
    writeln!(f).unwrap();

    // Write MDS matrix
    writeln!(f, "pub const MDS_MATRIX_U32: [[u32; {}]; {}] = [", T, T).unwrap();
    for (i, row) in mds_matrix.iter().enumerate() {
        writeln!(f, "    [").unwrap();
        for (j, &value) in row.iter().enumerate() {
            if j % 8 == 0 {
                write!(f, "        ").unwrap();
            }
            write!(f, "{}", value).unwrap();
            if j < row.len() - 1 {
                write!(f, ", ").unwrap();
            }
            if (j + 1) % 8 == 0 || j == row.len() - 1 {
                writeln!(f).unwrap();
            }
        }
        write!(f, "    ]").unwrap();
        if i < mds_matrix.len() - 1 {
            writeln!(f, ",").unwrap();
        } else {
            writeln!(f).unwrap();
        }
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/utils/poseidon/poseidon_params.rs");
}

/// Generate round constants using Grain LFSR in self-shrinking mode (u32 version)
fn generate_round_constants_u32(
    t: usize,
    full_rounds: usize,
    partial_rounds: usize,
    p: u32,
    alpha: u32,
    prime_bit_len: usize,
) -> Vec<u32> {
    // Initialize Grain LFSR state
    let mut state = init_grain_state(alpha, p, prime_bit_len, t, full_rounds, partial_rounds);

    // Number of round constants needed
    let rc_number = t * (full_rounds + partial_rounds);
    let mut rc_field = Vec::with_capacity(rc_number);

    // Discard first 160 output bits as per spec
    for _ in 0..160 {
        update_grain_state(&mut state);
    }

    // Generate constants
    while rc_field.len() < rc_number {
        let (new_state, bits) = generate_field_element_bits(state, prime_bit_len);
        state = new_state;

        // Convert bits to integer
        let rc_int = bits_to_u32(&bits);

        // Only use if within field
        if rc_int < p {
            rc_field.push(rc_int);
        }
    }

    rc_field
}

/// Generate MDS matrix using Cauchy construction (u32 version)
fn generate_mds_matrix_u32(t: usize) -> Vec<Vec<u32>> {
    // x_i = i for i in 0..t
    // y_j = t + j for j in 0..t
    let mut matrix = vec![vec![0u32; t]; t];

    for (i, row) in matrix.iter_mut().enumerate() {
        for (j, elem) in row.iter_mut().enumerate() {
            // mds[i][j] = 1 / (x_i + y_j) = 1 / (i + t + j)
            let sum = (i + t + j) as u32;
            // Calculate modular inverse using extended Euclidean algorithm
            *elem = mod_inverse(sum, P);
        }
    }

    matrix
}

/// Calculate modular inverse using extended Euclidean algorithm
fn mod_inverse(a: u32, m: u32) -> u32 {
    fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
        if a == 0 {
            (b, 0, 1)
        } else {
            let (g, y1, x1) = extended_gcd(b % a, a);
            let y = x1 - (b / a) * y1;
            (g, y, y1)
        }
    }

    let (_, x, _) = extended_gcd(a as i64, m as i64);
    ((x % m as i64 + m as i64) % m as i64) as u32
}

/// Initialize Grain LFSR state according to Poseidon spec
fn init_grain_state(
    alpha: u32,
    p: u32,
    prime_bit_len: usize,
    t: usize,
    full_rounds: usize,
    partial_rounds: usize,
) -> Vec<bool> {
    let mut state = Vec::with_capacity(80);

    // Field specification (2 bits)
    state.extend(&to_bits(p % 2, 2));

    // S-box specification (4 bits)
    let exp_flag = match alpha {
        3 => 0,
        5 => 1,
        u32::MAX => 2, // alpha = -1
        _ => 3,
    };
    state.extend(&to_bits(exp_flag, 4));

    // Field size (12 bits)
    state.extend(&to_bits(prime_bit_len as u32, 12));

    // State size (12 bits)
    state.extend(&to_bits(t as u32, 12));

    // Full rounds (10 bits)
    state.extend(&to_bits(full_rounds as u32, 10));

    // Partial rounds (10 bits)
    state.extend(&to_bits(partial_rounds as u32, 10));

    // Padding (30 bits of 1s)
    state.extend(vec![true; 30]);

    assert_eq!(state.len(), 80);
    state
}

/// Update Grain LFSR state
fn update_grain_state(state: &mut Vec<bool>) {
    // bi+80 = bi+62 ⊕ bi+51 ⊕ bi+38 ⊕ bi+23 ⊕ bi+13 ⊕ bi
    let new_bit = state[62] ^ state[51] ^ state[38] ^ state[23] ^ state[13] ^ state[0];
    state.remove(0);
    state.push(new_bit);
}

/// Generate field element bits using self-shrinking generator
fn generate_field_element_bits(
    mut state: Vec<bool>,
    prime_bit_len: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut bits = Vec::with_capacity(prime_bit_len);

    while bits.len() < prime_bit_len {
        // Generate first bit - the new bit is what was just calculated
        let new_bit_1 = state[62] ^ state[51] ^ state[38] ^ state[23] ^ state[13] ^ state[0];
        state.remove(0);
        state.push(new_bit_1);

        // Generate second bit
        let new_bit_2 = state[62] ^ state[51] ^ state[38] ^ state[23] ^ state[13] ^ state[0];
        state.remove(0);
        state.push(new_bit_2);

        // Self-shrinking: output bit2 only if bit1 is 1
        if new_bit_1 {
            bits.push(new_bit_2);
        }
    }

    (state, bits)
}

/// Convert integer to bits (big-endian, matching Python's bin() representation)
fn to_bits(n: u32, len: usize) -> Vec<bool> {
    (0..len).rev().map(|i| (n >> i) & 1 == 1).collect()
}

/// Convert bits to u32 (big-endian, matching Python's binary string conversion)
fn bits_to_u32(bits: &[bool]) -> u32 {
    bits.iter()
        .fold(0u32, |acc, &bit| (acc << 1) | (bit as u32))
}
