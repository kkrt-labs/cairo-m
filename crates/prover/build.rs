use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// Include the shared parameters
include!("src/utils/poseidon/poseidon_params.rs");

/// All documentation on Poseidon : https://www.poseidon-hash.info
/// Sage implementation for round constants and MDS matrix generation: https://extgit.isec.tugraz.at/krypto/hadeshash/-/blob/master/code/generate_parameters_grain.sage
/// Build script entry point that generates Poseidon constants at compile time.
///
/// This function generates the round constants and MDS matrix required for the Poseidon hash
/// function and writes them to a generated Rust file. The constants are computed using the
/// parameters defined in poseidon_params.rs and stored as raw u32 values to avoid runtime
/// dependencies on field types.
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

/// Generate round constants using Grain LFSR in self-shrinking mode.
///
/// This function implements the Grain Linear Feedback Shift Register (LFSR) in self-shrinking
/// mode as specified in the Poseidon paper to generate cryptographically secure round constants.
/// The generated constants are used to break symmetry in the Poseidon permutation.
///
/// ## Arguments
/// * `t` - State size (number of field elements in the state)
/// * `full_rounds` - Number of full rounds in the permutation
/// * `partial_rounds` - Number of partial rounds in the permutation
/// * `p` - Prime field modulus (2^31 - 1 for M31)
/// * `alpha` - S-box exponent (5 for x^5 S-box)
/// * `prime_bit_len` - Bit length of the prime field (31 for M31)
///
/// ## Returns
/// Vector of u32 round constants, one for each round and state element
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
        let _ = update_grain_state(&mut state);
    }

    // Generate constants
    while rc_field.len() < rc_number {
        let bits = generate_field_element_bits(&mut state, prime_bit_len);

        // Convert bits to integer
        let rc_int = bits_to_u32(&bits);

        // Only use if within field
        if rc_int < p {
            rc_field.push(rc_int);
        }
    }

    rc_field
}

/// Generate Maximum Distance Separable (MDS) matrix using Cauchy construction.
///
/// This function generates an MDS matrix that provides optimal diffusion in the Poseidon
/// linear layer. The Cauchy construction ensures that the matrix has the MDS property,
/// meaning any t×t submatrix is non-singular, providing maximum branch number.
///
/// The matrix elements are computed as: mds[i][j] = 1 / (x_i + y_j) where x_i = i and y_j = t + j.
///
/// ## Arguments
/// * `t` - State size (dimensions of the t×t matrix)
///
/// ## Returns
/// Two-dimensional vector representing the t×t MDS matrix with u32 elements
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

/// Calculate modular inverse using the extended Euclidean algorithm.
///
/// This function computes the modular multiplicative inverse of `a` modulo `m`, i.e.,
/// finds x such that (a * x) ≡ 1 (mod m). The extended Euclidean algorithm is used
/// to find the Bézout coefficients.
///
/// ## Arguments
/// * `a` - The value to find the inverse of
/// * `m` - The modulus
///
/// ## Returns
/// The modular inverse of `a` modulo `m`
///
/// ## Panics
/// This function assumes that `gcd(a, m) = 1` (i.e., `a` and `m` are coprime).
/// If they are not coprime, the behavior is undefined.
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

/// Initialize Grain LFSR state according to the Poseidon specification.
///
/// This function sets up the initial 80-bit state for the Grain LFSR used to generate
/// round constants. The state encodes the Poseidon parameters to ensure that different
/// parameter sets produce independent sequences of constants.
///
/// The 80-bit state is structured as:
/// - Field specification (2 bits)
/// - S-box specification (4 bits)
/// - Field size (12 bits)
/// - State size (12 bits)
/// - Full rounds (10 bits)
/// - Partial rounds (10 bits)
/// - Padding with 1s (30 bits)
///
/// ## Arguments
/// * `alpha` - S-box exponent
/// * `p` - Prime field modulus
/// * `prime_bit_len` - Bit length of the prime field
/// * `t` - State size
/// * `full_rounds` - Number of full rounds
/// * `partial_rounds` - Number of partial rounds
///
/// ## Returns
/// 80-element boolean vector representing the initial LFSR state
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

/// Update Grain LFSR state by one step.
///
/// This function implements the Grain LFSR feedback function as specified in the Poseidon paper.
/// The feedback polynomial is: b_{i+80} = b_{i+62} ⊕ b_{i+51} ⊕ b_{i+38} ⊕ b_{i+23} ⊕ b_{i+13} ⊕ b_i
///
/// Updates the Grain state and returns the newly generated bit
///
/// ## Arguments
/// * `state` - Mutable reference to the 80-bit LFSR state, updated in place
///
/// ## Returns
/// The newly generated bit after updating the state
fn update_grain_state(state: &mut Vec<bool>) -> bool {
    // bi+80 = bi+62 ⊕ bi+51 ⊕ bi+38 ⊕ bi+23 ⊕ bi+13 ⊕ bi
    let new_bit = state[62] ^ state[51] ^ state[38] ^ state[23] ^ state[13] ^ state[0];
    state.remove(0);
    state.push(new_bit);
    new_bit
}

/// This function implements the self-shrinking generator pattern on the Grain LFSR output
/// to produce bits for field elements. For each desired output bit, it generates two LFSR
/// bits and only outputs the second bit if the first bit is 1 (self-shrinking).
///
/// ## Arguments
/// * `state` - Mutable reference to the LFSR state (updated in place)
/// * `prime_bit_len` - Number of bits needed for the field element
///
/// ## Returns
/// Generated bits vector with length prime_bit_len
fn generate_field_element_bits(state: &mut Vec<bool>, prime_bit_len: usize) -> Vec<bool> {
    let mut bits = Vec::with_capacity(prime_bit_len);

    while bits.len() < prime_bit_len {
        // Generate first bit
        let new_bit_1 = update_grain_state(state);

        // Generate second bit
        let new_bit_2 = update_grain_state(state);

        // Self-shrinking: output bit2 only if bit1 is 1
        if new_bit_1 {
            bits.push(new_bit_2);
        }
    }

    bits
}

/// Convert integer to bit vector in big-endian format.
///
/// This function converts a u32 integer to a vector of boolean values representing
/// its binary representation, with the most significant bit first (big-endian).
/// The output is padded or truncated to the specified length.
///
/// ## Arguments
/// * `n` - The integer to convert
/// * `len` - The desired length of the output bit vector
///
/// ## Returns
/// Boolean vector of length `len` representing `n` in big-endian binary
fn to_bits(n: u32, len: usize) -> Vec<bool> {
    (0..len).rev().map(|i| (n >> i) & 1 == 1).collect()
}

/// Convert bit vector to u32 integer in big-endian format.
///
/// This function converts a slice of boolean values representing a binary number
/// (most significant bit first) into a u32 integer. This is the inverse operation
/// of `to_bits`.
///
/// ## Arguments
/// * `bits` - Slice of boolean values representing binary digits (MSB first)
///
/// ## Returns
/// The u32 integer represented by the bit vector
fn bits_to_u32(bits: &[bool]) -> u32 {
    bits.iter()
        .fold(0u32, |acc, &bit| (acc << 1) | (bit as u32))
}
