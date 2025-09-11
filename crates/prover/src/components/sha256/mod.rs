use num_traits::Zero;
use rayon::iter::IndexedParallelIterator;
use serde::{Deserialize, Serialize};
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::core::fields::m31::M31;

use std::ops::{Add, AddAssign, Mul, Sub};
use stwo_constraint_framework::FrameworkComponent;
use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::qm31::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::fields::FieldExpOps;

use crate::components::Relations;

pub mod air;
pub mod witness;

pub const MESSAGE_SIZE: usize = 2 * 16; // 16 elements of 32 bits
const N_ROUNDS: usize = 64;

// Main trace size
const N_TRACE_COLUMNS: usize = 2 * 16 // Message loading
    + (64 - 16) * 2 * (8 + 4 + 2 + 2 + 2 + 2) // Message schedule
    + 64 * (6 * 6 + 2 * 2 + 2 * 8 + 2 * 2 + 2 * 2 + 6 + 6 + 3 * 2 + 8 * 2); // Rounds

// Interaction trace size
const N_SMALL_SIGMA0_0_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_SMALL_SIGMA0_1_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_SMALL_SIGMA1_0_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_SMALL_SIGMA1_1_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_BIG_SIGMA0_0_LOOKUPS: usize = N_ROUNDS;
const N_BIG_SIGMA0_1_LOOKUPS: usize = N_ROUNDS;
const N_BIG_SIGMA1_0_LOOKUPS: usize = N_ROUNDS;
const N_BIG_SIGMA1_1_LOOKUPS: usize = N_ROUNDS;
const N_XOR_SMALL_SIGMA0_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_XOR_SMALL_SIGMA1_LOOKUPS: usize = N_ROUNDS - MESSAGE_SIZE / 2;
const N_XOR_BIG_SIGMA0_0_LOOKUPS: usize = N_ROUNDS;
const N_XOR_BIG_SIGMA0_1_LOOKUPS: usize = N_ROUNDS;
const N_XOR_BIG_SIGMA1_LOOKUPS: usize = N_ROUNDS;
const N_CH_LOOKUPS: usize = 6 * N_ROUNDS;
const N_MAJ_LOOKUPS: usize = 6 * N_ROUNDS;
const N_RANGE_CHECK_16_LOOKUPS: usize =
    2 * 16 + 2 * 2 * N_ROUNDS + 2 * 2 * (N_ROUNDS - MESSAGE_SIZE / 2); // TODO: range check sum results
const N_INTERACTION_COLUMNS: usize = SECURE_EXTENSION_DEGREE
    * (N_SMALL_SIGMA0_0_LOOKUPS
        + N_SMALL_SIGMA0_1_LOOKUPS
        + N_SMALL_SIGMA1_0_LOOKUPS
        + N_SMALL_SIGMA1_1_LOOKUPS
        + N_BIG_SIGMA0_0_LOOKUPS
        + N_BIG_SIGMA0_1_LOOKUPS
        + N_BIG_SIGMA1_0_LOOKUPS
        + N_BIG_SIGMA1_1_LOOKUPS
        + N_XOR_SMALL_SIGMA0_LOOKUPS
        + N_XOR_SMALL_SIGMA1_LOOKUPS
        + N_XOR_BIG_SIGMA0_0_LOOKUPS
        + N_XOR_BIG_SIGMA0_1_LOOKUPS
        + N_XOR_BIG_SIGMA1_LOOKUPS
        + N_CH_LOOKUPS
        + N_MAJ_LOOKUPS
        + N_RANGE_CHECK_16_LOOKUPS)
        .div_ceil(2);

#[derive(Clone, Copy, Debug)]
pub enum SigmaType {
    SmallSigma0,
    SmallSigma1,
    BigSigma0,
    BigSigma1,
}

/// Utility for representing a u32 as two field elements, for constraint evaluation.
#[derive(Clone, Debug)]
pub struct Fu32<F>
where
    F: FieldExpOps
        + Clone
        + Zero
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    pub lo: F,
    pub hi: F,
}
impl<F> Fu32<F>
where
    F: FieldExpOps
        + Zero
        + Clone
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    pub fn into_felts(self) -> [F; 2] {
        [self.lo, self.hi]
    }
}

impl<F> Zero for Fu32<F>
where
    F: FieldExpOps
        + Zero
        + Clone
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    fn zero() -> Self {
        Self {
            lo: F::zero(),
            hi: F::zero(),
        }
    }

    fn is_zero(&self) -> bool {
        self.lo.is_zero() && self.hi.is_zero()
    }
}

impl<F> Add for Fu32<F>
where
    F: FieldExpOps
        + Zero
        + Clone
        + AddAssign<F>
        + Add<F, Output = F>
        + Sub<F, Output = F>
        + Mul<M31, Output = F>,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            lo: self.lo + other.lo,
            hi: self.hi + other.hi,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

pub struct InteractionClaimData {
    pub lookup_data: LookupData,
    pub non_padded_length: usize,
}

#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub small_sigma0_0: [Vec<[PackedM31; 7]>; N_SMALL_SIGMA0_0_LOOKUPS],
    pub small_sigma0_1: [Vec<[PackedM31; 7]>; N_SMALL_SIGMA0_1_LOOKUPS],
    pub small_sigma1_0: [Vec<[PackedM31; 6]>; N_SMALL_SIGMA1_0_LOOKUPS],
    pub small_sigma1_1: [Vec<[PackedM31; 8]>; N_SMALL_SIGMA1_1_LOOKUPS],
    pub big_sigma0_0: [Vec<[PackedM31; 7]>; N_BIG_SIGMA0_0_LOOKUPS],
    pub big_sigma0_1: [Vec<[PackedM31; 7]>; N_BIG_SIGMA0_1_LOOKUPS],
    pub big_sigma1_0: [Vec<[PackedM31; 7]>; N_BIG_SIGMA1_0_LOOKUPS],
    pub big_sigma1_1: [Vec<[PackedM31; 7]>; N_BIG_SIGMA1_1_LOOKUPS],
    pub xor_small_sigma0: [Vec<[PackedM31; 6]>; N_XOR_SMALL_SIGMA0_LOOKUPS],
    pub xor_small_sigma1: [Vec<[PackedM31; 6]>; N_XOR_SMALL_SIGMA1_LOOKUPS],
    pub xor_big_sigma0_0: [Vec<[PackedM31; 3]>; N_XOR_BIG_SIGMA0_0_LOOKUPS],
    pub xor_big_sigma0_1: [Vec<[PackedM31; 3]>; N_XOR_BIG_SIGMA0_1_LOOKUPS],
    pub xor_big_sigma1: [Vec<[PackedM31; 6]>; N_XOR_BIG_SIGMA1_LOOKUPS],
    pub ch_l0: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub ch_l1: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub ch_l2: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub ch_h0: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub ch_h1: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub ch_h2: [Vec<[PackedM31; 4]>; N_CH_LOOKUPS / 6],
    pub maj_l0: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub maj_l1: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub maj_l2: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub maj_h0: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub maj_h1: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub maj_h2: [Vec<[PackedM31; 4]>; N_MAJ_LOOKUPS / 6],
    pub range_check_16: [Vec<[PackedM31; 1]>; N_RANGE_CHECK_16_LOOKUPS],
}

#[derive(Clone)]
pub struct Eval {
    pub claim: Claim,
    pub relations: Relations,
}
pub type Component = FrameworkComponent<Eval>;
