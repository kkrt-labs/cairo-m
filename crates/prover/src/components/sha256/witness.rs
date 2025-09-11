#![allow(non_snake_case)]

use num_traits::Zero;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::Relation;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::SHA256HashInput;
use crate::components::sha256::{
    Claim, Fu32, InteractionClaim, InteractionClaimData, LookupData, LookupDataMutChunk, SigmaType,
    MESSAGE_SIZE, N_INTERACTION_COLUMNS, N_TRACE_COLUMNS,
};
use crate::components::Relations;
use crate::utils::enabler::Enabler;

const MASK_SMALL_SIGMA0_L0: u32 = 0x4aaa; // O1 : 1, 3, 5, 7, 9, 11, 14
const MASK_SMALL_SIGMA0_L1: u32 = 0x155; // O0 : 0, 2, 4, 6, 8
const MASK_SMALL_SIGMA0_L2: u32 = 0xb400; // O0 : 10, 12, 13, 15
const MASK_SMALL_SIGMA0_H0: u32 = 0x550000; // O1 : 16, 18, 20, 22
const MASK_SMALL_SIGMA0_H1: u32 = 0xb5000000; // O1 : 24, 26, 28, 29, 31
const MASK_SMALL_SIGMA0_H2: u32 = 0x4aaa0000; // O0 : 17, 19, 21, 23, 25, 27, 30
const MASK_SMALL_SIGMA0_OUT0_LO: u32 = 0x2aa0; // O0 : 5, 7, 9, 11, 13
const MASK_SMALL_SIGMA0_OUT1_LO: u32 = 0x5550; // O1 : 4, 6, 8, 10, 12, 14
const MASK_SMALL_SIGMA0_OUT0_HI: u32 = 0x55500000; // O0 : 20, 22, 24, 26, 28, 30
const MASK_SMALL_SIGMA0_OUT1_HI: u32 = 0x2aa00000; // O1 : 21, 23, 25, 27, 29
const MASK_SMALL_SIGMA0_OUT2_LO: u32 = 0x800f; // O2: 0, 1, 2, 3, 15
const MASK_SMALL_SIGMA0_OUT2_HI: u32 = 0x800f0000; // O2: 16, 17, 18, 19, 31

const MASK_SMALL_SIGMA1_L0: u32 = 0x4285; // O0 : 0, 2, 7, 9, 14
const MASK_SMALL_SIGMA1_L1: u32 = 0x17a; // O1 : 1, 3, 4, 5, 6, 8
const MASK_SMALL_SIGMA1_L2: u32 = 0xbc00; // O1 : 10, 11, 12, 13, 15
const MASK_SMALL_SIGMA1_H0: u32 = 0x4aa40000; // O0 : 18, 21, 23, 25, 27, 30
const MASK_SMALL_SIGMA1_H1: u32 = 0x15a0000; // O1 : 17, 19, 20, 22, 24
const MASK_SMALL_SIGMA1_H2: u32 = 0xb4000000; // O1 : 26, 28, 29, 31
const MASK_SMALL_SIGMA1_OUT0_LO: u32 = 0x150a; // O0 : 1, 3, 8, 10, 12
const MASK_SMALL_SIGMA1_OUT1_LO: u32 = 0x2a95; // O1 : 0, 2, 4, 7, 9, 11, 13
const MASK_SMALL_SIGMA1_OUT0_HI: u32 = 0x40a0000; // O0 : 17, 19, 26
const MASK_SMALL_SIGMA1_OUT1_HI: u32 = 0x6ad50000; // O1 : 16, 18, 20, 22, 23, 25, 27, 29, 30
const MASK_SMALL_SIGMA1_OUT2_LO: u32 = 0xc060; // O2 : 5, 6, 14, 15
const MASK_SMALL_SIGMA1_OUT2_HI: u32 = 0x91200000; // O2 : 21, 24, 28, 31

const MASK_BIG_SIGMA0_L0: u32 = 0x7292; // O1 : 1, 4, 7, 9, 12, 13, 14
const MASK_BIG_SIGMA0_L1: u32 = 0x6d; // O0 : 0, 2, 3, 5, 6
const MASK_BIG_SIGMA0_L2: u32 = 0x8d00; // O0 : 8, 10, 11, 15
const MASK_BIG_SIGMA0_H0: u32 = 0xd60000; // O1 : 17, 18, 20, 22, 23
const MASK_BIG_SIGMA0_H1: u32 = 0x9c000000; // O1 : 26, 27, 28, 31
const MASK_BIG_SIGMA0_H2: u32 = 0x63290000; // O0 : 16, 19, 21, 24, 25, 29, 30
const MASK_BIG_SIGMA0_OUT0_LO: u32 = 0x4318; // O0 : 3, 4, 8, 9, 14
const MASK_BIG_SIGMA0_OUT1_LO: u32 = 0x84a4; // O1 : 2, 5, 7, 10, 15
const MASK_BIG_SIGMA0_OUT0_HI: u32 = 0x48420000; // O0 : 17, 22, 27, 30
const MASK_BIG_SIGMA0_OUT1_HI: u32 = 0x21100000; // O1 : 20, 24, 29
const MASK_BIG_SIGMA0_OUT2_LO: u32 = 0x3843; // O2 : 0, 1, 6, 11, 12, 13
const MASK_BIG_SIGMA0_OUT2_HI: u32 = 0x96ad0000; // O2 : 16, 18, 19, 21, 23, 25, 26, 28, 31

const MASK_BIG_SIGMA1_L0: u32 = 0xf83; // O0 : 0, 1, 7, 8, 9, 10, 11
const MASK_BIG_SIGMA1_L1: u32 = 0x7c; // O1 : 2, 3, 4, 5, 6
const MASK_BIG_SIGMA1_L2: u32 = 0xf000; // O1 : 12, 13, 14, 15
const MASK_BIG_SIGMA1_H0: u32 = 0x7c0000; // O0 : 18, 19, 20, 21, 22
const MASK_BIG_SIGMA1_H1: u32 = 0xf0000000; // O0 : 28, 29, 30, 31
const MASK_BIG_SIGMA1_H2: u32 = 0xf830000; // O1 : 16, 17, 23, 24, 25, 26, 27
const MASK_BIG_SIGMA1_OUT0_LO: u32 = 0x1e03; // O0 : 0, 1, 9, 10, 11, 12
const MASK_BIG_SIGMA1_OUT1_LO: u32 = 0x80f0; // O1 : 4, 5, 6, 7, 15
const MASK_BIG_SIGMA1_OUT0_HI: u32 = 0x80f00000; // O0 : 20, 21, 22, 23, 31
const MASK_BIG_SIGMA1_OUT1_HI: u32 = 0x1e030000; // O1 : 16, 17, 25, 26, 27, 28
const MASK_BIG_SIGMA1_OUT2_LO: u32 = 0x610c; // O2 : 2, 3, 8, 13, 14
const MASK_BIG_SIGMA1_OUT2_HI: u32 = 0x610c0000; // O2 : 18, 19, 24, 29, 30

#[derive(Copy, Clone, Serialize, Deserialize, Debug, Default)]
struct Indexes {
    col_index: usize,
    small_sigma0_0_index: usize,
    small_sigma0_1_index: usize,
    small_sigma1_0_index: usize,
    small_sigma1_1_index: usize,
    big_sigma0_0_index: usize,
    big_sigma0_1_index: usize,
    big_sigma1_0_index: usize,
    big_sigma1_1_index: usize,
    xor_small_sigma0_index: usize,
    xor_small_sigma1_index: usize,
    xor_big_sigma0_0_index: usize,
    xor_big_sigma0_1_index: usize,
    xor_big_sigma1_index: usize,
    ch_index: usize,
    maj_index: usize,
    range_check_16_index: usize,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N_TRACE_COLUMNS];
        let interaction_log_sizes = vec![self.log_size; N_INTERACTION_COLUMNS];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    #[allow(clippy::needless_range_loop)]
    pub fn write_trace<MC: MerkleChannel>(
        inputs: &Vec<SHA256HashInput>,
    ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let non_padded_length = inputs.len();
        let log_size = std::cmp::max(non_padded_length.next_power_of_two(), N_LANES).ilog2();

        // Pack round data from the prover input
        let packed_inputs: Vec<[PackedM31; MESSAGE_SIZE]> = inputs
            .iter()
            .chain(std::iter::repeat(&[M31::zero(); MESSAGE_SIZE]))
            .take(1 << log_size)
            .array_chunks::<N_LANES>()
            .map(|chunk| {
                std::array::from_fn(|x| PackedM31::from_array(std::array::from_fn(|y| chunk[y][x])))
            })
            .collect();

        let enabler_col = Enabler::new(non_padded_length);

        // Generate lookup data and fill the trace
        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                LookupData::uninitialized(log_size - LOG_N_LANES),
            )
        };

        (
            trace.par_iter_mut(),
            packed_inputs.into_par_iter(),
            lookup_data.par_iter_mut(),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(row_index, (mut row, message, mut lookup_data))| {
                let mut indexes = Indexes::default();
                let enabler = enabler_col.packed_at(row_index);
                *row[indexes.col_index] = enabler;
                indexes.col_index += 1;

                let K: [Fu32<PackedM31>; 64] = std::array::from_fn(|_| Fu32::zero());
                let mut H: [Fu32<PackedM31>; 8] = std::array::from_fn(|_| Fu32::zero());

                // ╔════════════════════════════════════╗
                // ║             Scheduling             ║
                // ╚════════════════════════════════════╝
                let mut W: [Fu32<PackedM31>; 64] = std::array::from_fn(|_| Fu32::zero());

                // Load message
                (0..16).for_each(|i| {
                    // Load lo and hi bits
                    W[i].lo = message[2 * i];
                    W[i].hi = message[2 * i + 1];
                    *row[indexes.col_index] = W[i].lo;
                    indexes.col_index += 1;
                    *row[indexes.col_index] = W[i].hi;
                    indexes.col_index += 1;
                    *lookup_data.range_check_16[indexes.range_check_16_index] = [W[i].lo];
                    indexes.range_check_16_index += 1;
                    *lookup_data.range_check_16[indexes.range_check_16_index] = [W[i].hi];
                    indexes.range_check_16_index += 1;
                });

                // Compute message schedule
                for i in 16..64 {
                    // TODO: W[i-15] and W[i-2] are not in temp sum so they could be decomposed in 4 limbs instead of 6

                    // Compute s0
                    let w_i_minus_15 = decompose_input(W[i - 15].clone(), SigmaType::SmallSigma0);
                    w_i_minus_15.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    let s0 = sigma(
                        SigmaType::SmallSigma0,
                        w_i_minus_15,
                        &mut indexes,
                        &mut row,
                        &mut lookup_data,
                    );

                    // Compute s1
                    let w_i_minus_2 = decompose_input(W[i - 2].clone(), SigmaType::SmallSigma1);
                    w_i_minus_2.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    let s1 = sigma(
                        SigmaType::SmallSigma1,
                        w_i_minus_2,
                        &mut indexes,
                        &mut row,
                        &mut lookup_data,
                    );

                    let temp = add3_u32_unchecked(
                        W[i - 16].clone(),
                        W[i - 7].clone(),
                        s0,
                        &mut indexes.col_index,
                        &mut row,
                    );
                    W[i] = add2_u32_unchecked(temp, s1, &mut indexes.col_index, &mut row);
                }

                // ╔════════════════════════════════════╗
                // ║             Rounds                 ║
                // ╚════════════════════════════════════╝
                for i in 0..64 {
                    let a: [PackedM31; 6] = decompose_input(H[0].clone(), SigmaType::BigSigma0);
                    let b: [PackedM31; 6] = decompose_input(H[1].clone(), SigmaType::BigSigma0);
                    let c: [PackedM31; 6] = decompose_input(H[2].clone(), SigmaType::BigSigma0);
                    let d: Fu32<PackedM31> = H[3].clone();
                    let e: [PackedM31; 6] = decompose_input(H[4].clone(), SigmaType::BigSigma1);
                    let f: [PackedM31; 6] = decompose_input(H[5].clone(), SigmaType::BigSigma1);
                    let g: [PackedM31; 6] = decompose_input(H[6].clone(), SigmaType::BigSigma1);
                    let h: Fu32<PackedM31> = H[7].clone();

                    a.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    b.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    c.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    *row[indexes.col_index] = d.lo;
                    indexes.col_index += 1;
                    *row[indexes.col_index] = d.hi;
                    indexes.col_index += 1;
                    e.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    f.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    g.iter().for_each(|x| {
                        *row[indexes.col_index] = *x;
                        indexes.col_index += 1;
                    });
                    *row[indexes.col_index] = h.lo;
                    indexes.col_index += 1;
                    *row[indexes.col_index] = h.hi;
                    indexes.col_index += 1;

                    let S0 = sigma(
                        SigmaType::BigSigma0,
                        a,
                        &mut indexes,
                        &mut row,
                        &mut lookup_data,
                    );
                    let S1 = sigma(
                        SigmaType::BigSigma1,
                        e,
                        &mut indexes,
                        &mut row,
                        &mut lookup_data,
                    );
                    let ch = ch(e, f, g, &mut indexes, &mut row, &mut lookup_data);
                    let maj = maj(a, b, c, &mut indexes, &mut row, &mut lookup_data);
                    let temp0 = add3_u32_unchecked(h, ch, S1, &mut indexes.col_index, &mut row);
                    let temp1 = add3_u32_unchecked(
                        temp0,
                        K[i].clone(),
                        W[i].clone(),
                        &mut indexes.col_index,
                        &mut row,
                    );
                    let temp2 = add2_u32_unchecked(S0, maj, &mut indexes.col_index, &mut row);

                    H[0] = add3_u32_unchecked(
                        temp1.clone(),
                        temp2,
                        H[0].clone(),
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[1] = add2_u32_unchecked(
                        H[1].clone(),
                        Fu32 {
                            lo: a[0] + a[1] + a[2],
                            hi: a[3] + a[4] + a[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[2] = add2_u32_unchecked(
                        H[2].clone(),
                        Fu32 {
                            lo: b[0] + b[1] + b[2],
                            hi: b[3] + b[4] + b[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[3] = add2_u32_unchecked(
                        H[3].clone(),
                        Fu32 {
                            lo: c[0] + c[1] + c[2],
                            hi: c[3] + c[4] + c[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[4] = add3_u32_unchecked(
                        d,
                        temp1,
                        H[4].clone(),
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[5] = add2_u32_unchecked(
                        H[5].clone(),
                        Fu32 {
                            lo: e[0] + e[1] + e[2],
                            hi: e[3] + e[4] + e[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[6] = add2_u32_unchecked(
                        H[6].clone(),
                        Fu32 {
                            lo: f[0] + f[1] + f[2],
                            hi: f[3] + f[4] + f[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                    H[7] = add2_u32_unchecked(
                        H[7].clone(),
                        Fu32 {
                            lo: g[0] + g[1] + g[2],
                            hi: g[3] + g[4] + g[5],
                        },
                        &mut indexes.col_index,
                        &mut row,
                    );
                }
            });
        (
            Self { log_size },
            trace,
            InteractionClaimData {
                lookup_data,
                non_padded_length,
            },
        )
    }
}

fn sigma(
    sigma: SigmaType,
    [l0, l1, l2, h0, h1, h2]: [PackedM31; 6],
    indexes: &mut Indexes,
    row: &mut Vec<&mut PackedM31>,
    lookup_data: &mut LookupDataMutChunk<'_>,
) -> Fu32<PackedM31> {
    // Compute out0 and out1
    let [out0_lo, out0_hi, out1_lo, out1_hi, out2_0_lo, out2_1_lo, out2_0_hi, out2_1_hi] =
        get_output(sigma, [l0, l1, l2, h0, h1, h2]);

    *row[indexes.col_index] = out0_lo;
    indexes.col_index += 1;
    *row[indexes.col_index] = out0_hi;
    indexes.col_index += 1;
    *row[indexes.col_index] = out1_lo;
    indexes.col_index += 1;
    *row[indexes.col_index] = out1_hi;
    indexes.col_index += 1;

    *row[indexes.col_index] = out2_0_lo;
    indexes.col_index += 1;
    *row[indexes.col_index] = out2_1_lo;
    indexes.col_index += 1;
    *row[indexes.col_index] = out2_0_hi;
    indexes.col_index += 1;
    *row[indexes.col_index] = out2_1_hi;
    indexes.col_index += 1;

    let xor_out2_lo = apply_xor(out2_0_lo, out2_1_lo);
    let xor_out2_hi = apply_xor(out2_0_hi, out2_1_hi);
    *row[indexes.col_index] = xor_out2_lo;
    indexes.col_index += 1;
    *row[indexes.col_index] = xor_out2_hi;
    indexes.col_index += 1;

    // Compute output of small sigma0 for first set of bits
    // BigSigma0 has special treatment because the set of bits that are affected by
    // both input0 and input1 is too large (we would need to XOR two words of 15 bits)
    match sigma {
        SigmaType::SmallSigma0 => {
            *lookup_data.small_sigma0_0[indexes.small_sigma0_0_index] =
                [l1, l2, h2, out0_lo, out0_hi, out2_0_lo, out2_0_hi];
            indexes.small_sigma0_0_index += 1;
            *lookup_data.small_sigma0_1[indexes.small_sigma0_1_index] =
                [l0, h0, h1, out1_lo, out1_hi, out2_1_lo, out2_1_hi];
            indexes.small_sigma0_1_index += 1;
            *lookup_data.xor_small_sigma0[indexes.xor_small_sigma0_index] = [
                out2_0_lo,
                out2_0_hi,
                out2_1_lo,
                out2_1_hi,
                xor_out2_lo,
                xor_out2_hi,
            ];
            indexes.xor_small_sigma0_index += 1;
        }
        SigmaType::SmallSigma1 => {
            *lookup_data.small_sigma1_0[indexes.small_sigma1_0_index] =
                [l0, h0, out0_lo, out0_hi, out2_0_lo, out2_0_hi];
            indexes.small_sigma1_0_index += 1;
            *lookup_data.small_sigma1_1[indexes.small_sigma1_1_index] =
                [l1, l2, h1, h2, out1_lo, out1_hi, out2_1_lo, out2_1_hi];
            indexes.small_sigma1_1_index += 1;
            *lookup_data.xor_small_sigma1[indexes.xor_small_sigma1_index] = [
                out2_0_lo,
                out2_0_hi,
                out2_1_lo,
                out2_1_hi,
                xor_out2_lo,
                xor_out2_hi,
            ];
            indexes.xor_small_sigma1_index += 1;
        }
        SigmaType::BigSigma0 => {
            *lookup_data.big_sigma0_0[indexes.big_sigma0_0_index] =
                [l1, l2, h2, out0_lo, out0_hi, out2_0_lo, out2_0_hi];
            indexes.big_sigma0_0_index += 1;
            *lookup_data.big_sigma0_1[indexes.big_sigma0_1_index] =
                [l0, h0, h1, out1_lo, out1_hi, out2_1_lo, out2_1_hi];
            indexes.big_sigma0_1_index += 1;
            *lookup_data.xor_big_sigma0_0[indexes.xor_big_sigma0_0_index] =
                [out2_0_lo, out2_1_lo, xor_out2_lo];
            indexes.xor_big_sigma0_0_index += 1;
            *lookup_data.xor_big_sigma0_1[indexes.xor_big_sigma0_1_index] =
                [out2_0_hi, out2_1_hi, xor_out2_hi];
            indexes.xor_big_sigma0_1_index += 1;
        }
        SigmaType::BigSigma1 => {
            *lookup_data.big_sigma1_0[indexes.big_sigma1_0_index] =
                [l0, h0, h1, out0_lo, out0_hi, out2_0_lo, out2_0_hi];
            indexes.big_sigma1_0_index += 1;
            *lookup_data.big_sigma1_1[indexes.big_sigma1_1_index] =
                [l1, l2, h2, out1_lo, out1_hi, out2_1_lo, out2_1_hi];
            indexes.big_sigma1_1_index += 1;
            *lookup_data.xor_big_sigma1[indexes.xor_big_sigma1_index] = [
                out2_0_lo,
                out2_0_hi,
                out2_1_lo,
                out2_1_hi,
                xor_out2_lo,
                xor_out2_hi,
            ];
            indexes.xor_big_sigma1_index += 1;
        }
    };

    // Add all limbs together to rebuild the 32-bit result
    let out0 = Fu32 {
        lo: out0_lo,
        hi: out0_hi,
    };
    let out1 = Fu32 {
        lo: out1_lo,
        hi: out1_hi,
    };
    let out2 = Fu32 {
        lo: xor_out2_lo,
        hi: xor_out2_hi,
    };
    let res = add3_u32_unchecked(out0, out1, out2, &mut indexes.col_index, row);

    *lookup_data.range_check_16[indexes.range_check_16_index] = [res.lo];
    indexes.range_check_16_index += 1;
    *lookup_data.range_check_16[indexes.range_check_16_index] = [res.hi];
    indexes.range_check_16_index += 1;

    res
}

fn ch(
    e: [PackedM31; 6],
    f: [PackedM31; 6],
    g: [PackedM31; 6],
    indexes: &mut Indexes,
    row: &mut [&mut PackedM31],
    lookup_data: &mut LookupDataMutChunk<'_>,
) -> Fu32<PackedM31> {
    let ch: [PackedM31; 6] = std::array::from_fn(|i| apply_ch(e[i], f[i], g[i]));
    ch.iter().enumerate().for_each(|(i, x)| {
        *row[indexes.col_index] = *x;
        indexes.col_index += 1;
        *lookup_data.ch[indexes.ch_index] = [PackedM31::from(M31::from(i)), e[i], f[i], g[i], *x];
        indexes.ch_index += 1;
    });

    Fu32 {
        lo: ch[0] + ch[1] + ch[2],
        hi: ch[3] + ch[4] + ch[5],
    }
}

fn maj(
    a: [PackedM31; 6],
    b: [PackedM31; 6],
    c: [PackedM31; 6],
    indexes: &mut Indexes,
    row: &mut [&mut PackedM31],
    lookup_data: &mut LookupDataMutChunk<'_>,
) -> Fu32<PackedM31> {
    let maj: [PackedM31; 6] = std::array::from_fn(|i| apply_maj(a[i], b[i], c[i]));
    maj.iter().enumerate().for_each(|(i, x)| {
        *row[indexes.col_index] = *x;
        indexes.col_index += 1;
        *lookup_data.maj[indexes.maj_index] = [PackedM31::from(M31::from(i)), a[i], b[i], c[i], *x];
        indexes.maj_index += 1;
    });

    Fu32 {
        lo: maj[0] + maj[1] + maj[2],
        hi: maj[3] + maj[4] + maj[5],
    }
}

fn decompose_input(a: Fu32<PackedM31>, sigma: SigmaType) -> [PackedM31; 6] {
    let a_u32: [u32; N_LANES] =
        a.lo.to_array()
            .iter()
            .zip(a.hi.to_array().iter())
            .map(|(lo, hi)| lo.0 | (hi.0 << 16))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

    // Apply decompose_u32 to each element of a_u32
    let decomposed: [[u32; 6]; N_LANES] = std::array::from_fn(|i| decompose_u32(a_u32[i], sigma));

    // Convert to [PackedM31; 6]
    std::array::from_fn(|limb_idx| {
        PackedM31::from_array(std::array::from_fn(|lane_idx| {
            M31(decomposed[lane_idx][limb_idx])
        }))
    })
}

const fn decompose_u32(a: u32, sigma: SigmaType) -> [u32; 6] {
    match sigma {
        SigmaType::SmallSigma0 => [
            a & MASK_SMALL_SIGMA0_L0,
            a & MASK_SMALL_SIGMA0_L1,
            a & MASK_SMALL_SIGMA0_L2,
            (a & MASK_SMALL_SIGMA0_H0) >> 16,
            (a & MASK_SMALL_SIGMA0_H1) >> 16,
            (a & MASK_SMALL_SIGMA0_H2) >> 16,
        ],
        SigmaType::SmallSigma1 => [
            a & MASK_SMALL_SIGMA1_L0,
            a & MASK_SMALL_SIGMA1_L1,
            a & MASK_SMALL_SIGMA1_L2,
            (a & MASK_SMALL_SIGMA1_H0) >> 16,
            (a & MASK_SMALL_SIGMA1_H1) >> 16,
            (a & MASK_SMALL_SIGMA1_H2) >> 16,
        ],
        SigmaType::BigSigma0 => [
            a & MASK_BIG_SIGMA0_L0,
            a & MASK_BIG_SIGMA0_L1,
            a & MASK_BIG_SIGMA0_L2,
            (a & MASK_BIG_SIGMA0_H0) >> 16,
            (a & MASK_BIG_SIGMA0_H1) >> 16,
            (a & MASK_BIG_SIGMA0_H2) >> 16,
        ],
        SigmaType::BigSigma1 => [
            a & MASK_BIG_SIGMA1_L0,
            a & MASK_BIG_SIGMA1_L1,
            a & MASK_BIG_SIGMA1_L2,
            (a & MASK_BIG_SIGMA1_H0) >> 16,
            (a & MASK_BIG_SIGMA1_H1) >> 16,
            (a & MASK_BIG_SIGMA1_H2) >> 16,
        ],
    }
}

fn get_output(sigma: SigmaType, [l0, l1, l2, h0, h1, h2]: [PackedM31; 6]) -> [PackedM31; 8] {
    let input0_u32 = match sigma {
        SigmaType::SmallSigma0 => rebuild_word_from_limbs(&[l1, l2], &[h2]),
        SigmaType::SmallSigma1 => rebuild_word_from_limbs(&[l0], &[h0]),
        SigmaType::BigSigma0 => rebuild_word_from_limbs(&[l1, l2], &[h2]),
        SigmaType::BigSigma1 => rebuild_word_from_limbs(&[l0], &[h0, h1]),
    };
    let input1_u32 = match sigma {
        SigmaType::SmallSigma0 => rebuild_word_from_limbs(&[l0], &[h0, h1]),
        SigmaType::SmallSigma1 => rebuild_word_from_limbs(&[l1, l2], &[h1, h2]),
        SigmaType::BigSigma0 => rebuild_word_from_limbs(&[l0], &[h0, h1]),
        SigmaType::BigSigma1 => rebuild_word_from_limbs(&[l1, l2], &[h2]),
    };
    let sigma_function_u32 = match sigma {
        SigmaType::SmallSigma0 => {
            |x: u32| x.rotate_right(3) ^ x.rotate_right(7) ^ x.rotate_right(18)
        }
        SigmaType::SmallSigma1 => {
            |x: u32| x.rotate_right(10) ^ x.rotate_right(17) ^ x.rotate_right(19)
        }
        SigmaType::BigSigma0 => {
            |x: u32| x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
        }
        SigmaType::BigSigma1 => {
            |x: u32| x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
        }
    };
    let sigma_function = |x: [u32; N_LANES]| x.map(sigma_function_u32);
    let out0_u32 = sigma_function(input0_u32);
    let out1_u32 = sigma_function(input1_u32);

    // Extract output values using a helper function, similar to decompose pattern
    match sigma {
        SigmaType::SmallSigma0 => [
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA0_OUT0_LO))),
            PackedM31::from_array(
                out0_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA0_OUT0_HI) >> 16)),
            ),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA0_OUT1_LO))),
            PackedM31::from_array(
                out1_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA0_OUT1_HI) >> 16)),
            ),
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA0_OUT2_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA0_OUT2_LO))),
            PackedM31::from_array(
                out0_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA0_OUT2_HI) >> 16)),
            ),
            PackedM31::from_array(
                out1_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA0_OUT2_HI) >> 16)),
            ),
        ],
        SigmaType::SmallSigma1 => [
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA1_OUT0_LO))),
            PackedM31::from_array(
                out0_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA1_OUT0_HI) >> 16)),
            ),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA1_OUT1_LO))),
            PackedM31::from_array(
                out1_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA1_OUT1_HI) >> 16)),
            ),
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA1_OUT2_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_SMALL_SIGMA1_OUT2_LO))),
            PackedM31::from_array(
                out0_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA1_OUT2_HI) >> 16)),
            ),
            PackedM31::from_array(
                out1_u32.map(|x| M31::from((x & MASK_SMALL_SIGMA1_OUT2_HI) >> 16)),
            ),
        ],
        SigmaType::BigSigma0 => [
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_BIG_SIGMA0_OUT0_LO))),
            PackedM31::from_array(out0_u32.map(|x| M31::from((x & MASK_BIG_SIGMA0_OUT0_HI) >> 16))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_BIG_SIGMA0_OUT1_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from((x & MASK_BIG_SIGMA0_OUT1_HI) >> 16))),
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_BIG_SIGMA0_OUT2_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_BIG_SIGMA0_OUT2_LO))),
            PackedM31::from_array(out0_u32.map(|x| M31::from((x & MASK_BIG_SIGMA0_OUT2_HI) >> 16))),
            PackedM31::from_array(out1_u32.map(|x| M31::from((x & MASK_BIG_SIGMA0_OUT2_HI) >> 16))),
        ],
        SigmaType::BigSigma1 => [
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_BIG_SIGMA1_OUT0_LO))),
            PackedM31::from_array(out0_u32.map(|x| M31::from((x & MASK_BIG_SIGMA1_OUT0_HI) >> 16))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_BIG_SIGMA1_OUT1_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from((x & MASK_BIG_SIGMA1_OUT1_HI) >> 16))),
            PackedM31::from_array(out0_u32.map(|x| M31::from(x & MASK_BIG_SIGMA1_OUT2_LO))),
            PackedM31::from_array(out1_u32.map(|x| M31::from(x & MASK_BIG_SIGMA1_OUT2_LO))),
            PackedM31::from_array(out0_u32.map(|x| M31::from((x & MASK_BIG_SIGMA1_OUT2_HI) >> 16))),
            PackedM31::from_array(out1_u32.map(|x| M31::from((x & MASK_BIG_SIGMA1_OUT2_HI) >> 16))),
        ],
    }
}

/// Apply a bitwise XOR to two PackedM31 values
fn apply_xor(x: PackedM31, y: PackedM31) -> PackedM31 {
    PackedM31::from_array(
        x.to_array()
            .iter()
            .zip(y.to_array().iter())
            .map(|(x, y)| M31::from(x.0 ^ y.0))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}

/// Apply a bitwise AND and XOR to three PackedM31 values
fn apply_ch(e: PackedM31, f: PackedM31, g: PackedM31) -> PackedM31 {
    PackedM31::from_array(
        e.to_array()
            .iter()
            .zip(f.to_array().iter())
            .zip(g.to_array().iter())
            .map(|((e, f), g)| M31::from((e.0 & f.0) ^ ((!e.0) & g.0)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}

/// Apply a bitwise AND and XOR to three PackedM31 values
fn apply_maj(a: PackedM31, b: PackedM31, c: PackedM31) -> PackedM31 {
    PackedM31::from_array(
        a.to_array()
            .iter()
            .zip(b.to_array().iter())
            .zip(c.to_array().iter())
            .map(|((a, b), c)| M31::from((a.0 & b.0) ^ (a.0 & c.0) ^ (b.0 & c.0)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}

/// Adds two u32s, returning the sum.
fn add2_u32_unchecked(
    a: Fu32<PackedM31>,
    b: Fu32<PackedM31>,
    col_index: &mut usize,
    row: &mut [&mut PackedM31],
) -> Fu32<PackedM31> {
    let two_pow_16 = PackedM31::from(M31::from(1 << 16));

    // Calculate carry using mask instead of comparison
    // For 16-bit addition: carry = (sum >> 16) & 1
    let sum_lo = a.lo + b.lo;
    let carry_lo = PackedM31::from_array(sum_lo.to_array().map(|x| M31::from((x.0 >> 16) & 1)));

    let sum_hi = a.hi + b.hi + carry_lo;
    let carry_hi = PackedM31::from_array(sum_hi.to_array().map(|x| M31::from((x.0 >> 16) & 1)));

    let res = Fu32 {
        lo: sum_lo - carry_lo * two_pow_16,
        hi: sum_hi - carry_hi * two_pow_16,
    };

    *row[*col_index] = res.lo;
    *col_index += 1;
    *row[*col_index] = res.hi;
    *col_index += 1;

    res
}

/// Adds three u32s, returning the sum.
fn add3_u32_unchecked(
    a: Fu32<PackedM31>,
    b: Fu32<PackedM31>,
    c: Fu32<PackedM31>,
    col_index: &mut usize,
    row: &mut [&mut PackedM31],
) -> Fu32<PackedM31> {
    let two_pow_16 = PackedM31::from(M31::from(1 << 16));

    // Calculate carry using mask instead of comparison
    // For 16-bit addition: carry = (sum >> 16) & 0b11
    let sum_lo = a.lo + b.lo + c.lo;
    let carry_lo = PackedM31::from_array(sum_lo.to_array().map(|x| M31::from((x.0 >> 16) & 0b10)));

    let sum_hi = a.hi + b.hi + c.hi + carry_lo;
    let carry_hi = PackedM31::from_array(sum_hi.to_array().map(|x| M31::from((x.0 >> 16) & 0b10)));

    let res = Fu32 {
        lo: sum_lo - carry_lo * two_pow_16,
        hi: sum_hi - carry_hi * two_pow_16,
    };

    *row[*col_index] = res.lo;
    *col_index += 1;
    *row[*col_index] = res.hi;
    *col_index += 1;

    res
}

/// Helper function to rebuild a 32-bit word from low and high limbs
fn rebuild_word_from_limbs(low_limbs: &[PackedM31], high_limbs: &[PackedM31]) -> [u32; N_LANES] {
    let mut result_array = [0u32; N_LANES];

    for i in 0..N_LANES {
        let mut word = 0u32;

        // Combine low limbs with bitwise OR
        for low_limb in low_limbs {
            word |= low_limb.to_array()[i].0;
        }

        // Combine high limbs shifted by 16 bits
        for high_limb in high_limbs {
            word |= high_limb.to_array()[i].0 << 16;
        }

        result_array[i] = word;
    }

    result_array
}

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        Vec<CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
    ) {
        let log_size = std::cmp::max(
            interaction_claim_data.non_padded_length.next_power_of_two(),
            N_LANES,
        )
        .ilog2();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);
        /// Macro to generate interaction trace for lookup data pairs
        macro_rules! generate_interaction_trace {
            ($relation_name_1:ident, $i_1:expr, $relation_name_2:ident, $i_2:expr) => {{
                let mut col = interaction_trace.new_col();
                (
                    col.par_iter_mut(),
                    &interaction_claim_data.lookup_data.$relation_name_1[$i_1],
                    &interaction_claim_data.lookup_data.$relation_name_2[$i_2],
                )
                    .into_par_iter()
                    .enumerate()
                    .for_each(|(i, (writer, value0, value1))| {
                        let num: PackedQM31 = -PackedQM31::from(enabler_col.packed_at(i));
                        let denom0: PackedQM31 = relations.$relation_name_1.combine(value0);
                        let denom1: PackedQM31 = relations.$relation_name_2.combine(value1);

                        let numerator = num * (denom1 + denom0);
                        let denom = denom0 * denom1;

                        writer.write_frac(numerator, denom);
                    });
                col.finalize_col();
            }};
        }

        // Message schedule
        for i in 0..16 {
            generate_interaction_trace!(range_check_16, 2 * i, range_check_16, 2 * i + 1);
        }
        for i in 16..64 {
            generate_interaction_trace!(small_sigma0_0, i - 16, small_sigma0_1, i - 16);
            generate_interaction_trace!(xor_small_sigma0, i - 16, range_check_16, 4 * (i - 8));
            generate_interaction_trace!(range_check_16, 4 * (i - 8) + 1, small_sigma1_0, i - 16);
            generate_interaction_trace!(small_sigma1_1, i - 16, xor_small_sigma1, i - 16);
            generate_interaction_trace!(
                range_check_16,
                4 * (i - 8) + 2,
                range_check_16,
                4 * (i - 8) + 3
            );
        }

        // Rounds
        for i in 0..32 {
            generate_interaction_trace!(big_sigma0_0, 2 * i, big_sigma0_1, 2 * i);
            generate_interaction_trace!(xor_big_sigma0_0, 2 * i, xor_big_sigma0_1, 2 * i);
            generate_interaction_trace!(
                range_check_16,
                224 + 8 * i,
                range_check_16,
                224 + 8 * i + 1
            );
            generate_interaction_trace!(big_sigma1_0, 2 * i, big_sigma1_1, 2 * i);
            generate_interaction_trace!(xor_big_sigma1, 2 * i, range_check_16, 224 + 8 * i + 2);
            generate_interaction_trace!(range_check_16, 224 + 8 * i + 3, ch, 12 * i);
            generate_interaction_trace!(ch, 12 * i + 1, ch, 12 * i + 2);
            generate_interaction_trace!(ch, 12 * i + 3, ch, 12 * i + 4);
            generate_interaction_trace!(ch, 12 * i + 5, maj, 12 * i);
            generate_interaction_trace!(maj, 12 * i + 1, maj, 12 * i + 2);
            generate_interaction_trace!(maj, 12 * i + 3, maj, 12 * i + 4);
            generate_interaction_trace!(maj, 12 * i + 5, big_sigma0_0, 2 * i + 1);
            generate_interaction_trace!(big_sigma0_1, 2 * i + 1, xor_big_sigma0_0, 2 * i + 1);
            generate_interaction_trace!(
                xor_big_sigma0_1,
                2 * i + 1,
                range_check_16,
                224 + 8 * i + 4
            );
            generate_interaction_trace!(range_check_16, 224 + 8 * i + 5, big_sigma1_0, 2 * i + 1);
            generate_interaction_trace!(big_sigma1_1, 2 * i + 1, xor_big_sigma1, 2 * i + 1);
            generate_interaction_trace!(
                range_check_16,
                224 + 8 * i + 6,
                range_check_16,
                224 + 8 * i + 7
            );
            generate_interaction_trace!(ch, 12 * i + 6, ch, 12 * i + 7);
            generate_interaction_trace!(ch, 12 * i + 8, ch, 12 * i + 9);
            generate_interaction_trace!(ch, 12 * i + 10, ch, 12 * i + 11);
            generate_interaction_trace!(maj, 12 * i + 6, maj, 12 * i + 7);
            generate_interaction_trace!(maj, 12 * i + 8, maj, 12 * i + 9);
            generate_interaction_trace!(maj, 12 * i + 10, maj, 12 * i + 11);
        }

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }
}
