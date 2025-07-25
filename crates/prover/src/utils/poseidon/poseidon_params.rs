// Poseidon hash parameters shared between the implementation and build script
// This is the single source of truth for all Poseidon constants

/// ALPHA: power in the S-Box. The paper suggest x**5 as S-Box.
///
/// Note that stwo-cairo uses alpha=3.
/// We pick 5 so that gcd(PRIME-1, 5) = 1 holds, indeed gcd(PRIME-1, 3) = 3.
/// This const is exclusively used in the build script not for the hash computation nor the AIR (where x**5 s-box is hardcoded)
pub const ALPHA: u32 = 5;

/// P: The prime field modulus (2^31 - 1)
pub const P: u32 = 2_147_483_647;

/// Prime bit length
pub const PRIME_BIT_LEN: usize = 31;

/// M: number of bits of security
pub const M: usize = 96;

/// INPUT_SIZE: number of inputs here 2 for hash(M31::from, M31::from)
pub const INPUT_SIZE: usize = 2;

/// CAPACITY_SIZE: number of capacity elements, the paper uses 2*M bits.
///
/// We use the div_ceil to ensure that the capacity size is at least 2*M bits.
pub const CAPACITY_SIZE: usize = (2 * M).div_ceil(PRIME_BIT_LEN);

/// T: State size
pub const T: usize = INPUT_SIZE + CAPACITY_SIZE;

/// FULL_ROUNDS
/// The poseidon paper tests use 8 full rounds.
pub const FULL_ROUNDS: usize = 8;

/// PARTIAL_ROUNDS
/// The paper exposes different critieras for different attacks. The tests use 35 (M=80), 60 (M=128), 120 (M=256).
pub const PARTIAL_ROUNDS: usize = 56;
