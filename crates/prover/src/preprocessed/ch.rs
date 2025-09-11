use std::sync::atomic::{AtomicU32, Ordering};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

// Masks from witness.rs for BigSigma1 (used for ch decomposition)
const MASK_BIG_SIGMA1_L0: u32 = 0xf83;
const MASK_BIG_SIGMA1_L1: u32 = 0x7c;
const MASK_BIG_SIGMA1_L2: u32 = 0xf000;
const MASK_BIG_SIGMA1_H0: u32 = 0x7c0000;
const MASK_BIG_SIGMA1_H1: u32 = 0xf0000000;
const MASK_BIG_SIGMA1_H2: u32 = 0xf830000;

// Constants for ch operations
/// Number of ch variants (l0, l1, l2, h0, h1, h2)
pub const CH_NUM_VARIANTS: u32 = 6;

/// Total log size for stacked ch operations
pub const CH_STACKED_LOG_SIZE: u32 = 23; // This should be dynamically calculated but we use a conservative estimate

pub struct InteractionClaimData {
    pub ch: Vec<[PackedM31; 6]>, // variant_id, e, f, g, result, multiplicity
}

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; 1];
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    /// Writes the preprocessed ch trace for all 6 variants
    ///
    /// lookup_data contains all ch operations made in other components during main trace generation
    /// Each entry is [variant_id, e, f, g, result] where variant_id is 0-5 for l0, l1, l2, h0, h1, h2
    ///
    /// write_trace creates columns for:
    /// - All variants stacked with their respective combinations
    pub fn write_trace<'a, MC: MerkleChannel>(
        lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; 5]]>,
    ) -> (
        Self,
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        // Build index mapping for each variant
        // The high masks are incorrect to compute the ch output of high limbs
        // but here `masks` is only used for indexing
        let masks = [
            MASK_BIG_SIGMA1_L0,
            MASK_BIG_SIGMA1_L1,
            MASK_BIG_SIGMA1_L2,
            MASK_BIG_SIGMA1_H0 >> 16,
            MASK_BIG_SIGMA1_H1 >> 16,
            MASK_BIG_SIGMA1_H2 >> 16,
        ];

        // Calculate offsets for each variant
        let mut variant_offsets = [0usize; 7];
        for i in 0..6 {
            let num_bits = masks[i].count_ones();
            let variant_size = 1usize << (3 * num_bits);
            variant_offsets[i + 1] = variant_offsets[i] + variant_size;
        }

        let total_size = variant_offsets[6].next_power_of_two();

        // Initialize multiplicities for all combinations
        let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

        // Count occurrences of each (variant_id, e, f, g) quadruple
        lookup_data.for_each(|entries| {
            for entry in entries.iter() {
                // entry[0] contains variant_id values (0-5)
                // entry[1] contains packed e values
                // entry[2] contains packed f values
                // entry[3] contains packed g values
                for i in 0..N_LANES {
                    let variant_id = entry[0].to_array()[i].0 as usize;
                    let e = entry[1].to_array()[i].0;
                    let f = entry[2].to_array()[i].0;
                    let g = entry[3].to_array()[i].0;

                    if variant_id < 6 {
                        let mask = masks[variant_id];

                        // Compress the values according to the mask
                        let e_compressed = compress_value_to_mask(e, mask);
                        let f_compressed = compress_value_to_mask(f, mask);
                        let g_compressed = compress_value_to_mask(g, mask);

                        let num_bits = mask.count_ones();
                        let num_combinations = 1usize << num_bits;

                        // Calculate index within variant
                        let variant_index =
                            (e_compressed as usize) * num_combinations * num_combinations
                                + (f_compressed as usize) * num_combinations
                                + (g_compressed as usize);

                        let global_index = variant_offsets[variant_id] + variant_index;

                        if global_index < total_size {
                            mults_atomic[global_index].fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        });

        // Convert atomic multiplicities to M31
        let mults: Vec<M31> = mults_atomic
            .into_par_iter()
            .map(|atomic| M31(atomic.into_inner()))
            .collect();

        // Generate all columns using the helper method
        let variant_id_col = ChCol::new(0).generate_column_values(total_size);
        let e_col = ChCol::new(1).generate_column_values(total_size);
        let f_col = ChCol::new(2).generate_column_values(total_size);
        let g_col = ChCol::new(3).generate_column_values(total_size);
        let result_col = ChCol::new(4).generate_column_values(total_size);

        // Pack data for interaction
        let packed_data: Vec<[PackedM31; 6]> = (0..total_size)
            .collect::<Vec<_>>()
            .par_chunks(N_LANES)
            .enumerate()
            .map(|(chunk_idx, _chunk)| {
                let base_idx = chunk_idx * N_LANES;
                [
                    PackedM31::from_array(std::array::from_fn(|i| variant_id_col[base_idx + i])),
                    PackedM31::from_array(std::array::from_fn(|i| e_col[base_idx + i])),
                    PackedM31::from_array(std::array::from_fn(|i| f_col[base_idx + i])),
                    PackedM31::from_array(std::array::from_fn(|i| g_col[base_idx + i])),
                    PackedM31::from_array(std::array::from_fn(|i| result_col[base_idx + i])),
                    PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i])),
                ]
            })
            .collect();

        let domain = CanonicCoset::new(CH_STACKED_LOG_SIZE).circle_domain();

        (
            Self {
                log_size: CH_STACKED_LOG_SIZE,
            },
            [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                domain,
                BaseColumn::from_iter(mults),
            )],
            InteractionClaimData { ch: packed_data },
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        ch: &crate::relations::Ch,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.ch.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &interaction_claim_data.ch)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 =
                    ch.combine(&[value[0], value[1], value[2], value[3], value[4]]);
                writer.write_frac(value[5].into(), denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }
}

#[derive(Clone)]
pub struct Eval {
    pub claim: Claim,
    pub relation: crate::relations::Ch,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        // Read the 5 preprocessed columns
        let ids = Ch::new().ids();
        let [variant_id, e, f, g, result] =
            std::array::from_fn(|i| eval.get_preprocessed_column(ids[i].clone()));
        let multiplicity = eval.next_trace_mask();

        // Add lookups to the relation
        eval.add_to_relation(RelationEntry::new(
            &self.relation,
            E::EF::from(multiplicity),
            &[variant_id, e, f, g, result],
        ));

        eval.finalize_logup();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;

/// Ch preprocessed columns container
/// Represents all columns needed for ch operations (variant_id, e, f, g, result)
pub struct Ch;

impl Ch {
    pub const fn new() -> Self {
        Self
    }

    /// Returns the 5 columns needed for ch operations
    pub(crate) const fn columns(&self) -> [ChCol; 5] {
        [
            ChCol::new(0),
            ChCol::new(1),
            ChCol::new(2),
            ChCol::new(3),
            ChCol::new(4),
        ]
    }

    /// Returns the column IDs for the 5 columns
    pub fn ids(&self) -> [PreProcessedColumnId; 5] {
        [
            ChCol::new(0).id(),
            ChCol::new(1).id(),
            ChCol::new(2).id(),
            ChCol::new(3).id(),
            ChCol::new(4).id(),
        ]
    }
}

/// Stacked ch preprocessed column
/// Stacks ch_l0, ch_l1, ch_l2, ch_h0, ch_h1, ch_h2 operations into a single column
pub(crate) struct ChCol {
    col_index: usize, // 0: variant_id, 1: e, 2: f, 3: g, 4: result
}

impl ChCol {
    pub const fn new(col_index: usize) -> Self {
        assert!(col_index < 5, "col_index must be in range 0..=4");
        Self { col_index }
    }

    /// Generate column values for a specific column index
    /// Returns values for all stacked ch variants
    fn generate_column_values(&self, total_size: usize) -> Vec<M31> {
        let mut values = Vec::with_capacity(total_size);

        // Get the masks for each variant
        // High masks are correct because of bit expansion
        let masks = [
            MASK_BIG_SIGMA1_L0,       // l0
            MASK_BIG_SIGMA1_L1,       // l1
            MASK_BIG_SIGMA1_L2,       // l2
            MASK_BIG_SIGMA1_H0 >> 16, // h0
            MASK_BIG_SIGMA1_H1 >> 16, // h1
            MASK_BIG_SIGMA1_H2 >> 16, // h2
        ];

        // Stack six ch variants
        for variant_id in 0..6 {
            let mask = masks[variant_id];

            // Count the number of bits set in the mask
            let num_bits = mask.count_ones();
            let num_combinations = 1u32 << num_bits;

            // Generate all possible combinations for e, f, g based on the mask
            for e_bits in 0..num_combinations {
                for f_bits in 0..num_combinations {
                    for g_bits in 0..num_combinations {
                        // Convert bit indices to actual 16-bit values based on mask
                        let e = expand_bits_to_mask(e_bits, mask);
                        let f = expand_bits_to_mask(f_bits, mask);
                        let g = expand_bits_to_mask(g_bits, mask);

                        let value = match self.col_index {
                            0 => M31(variant_id as u32), // variant_id
                            1 => M31(e),                 // e
                            2 => M31(f),                 // f
                            3 => M31(g),                 // g
                            4 => {
                                // result: apply ch function
                                let result = (e & f) ^ ((!e) & g);
                                M31(result)
                            }
                            _ => unreachable!(),
                        };
                        values.push(value);
                    }
                }
            }
        }

        // Pad with zeros to reach power of 2
        values.resize(total_size, M31(0));
        values
    }
}

impl PreProcessedColumn for ChCol {
    fn log_size(&self) -> u32 {
        CH_STACKED_LOG_SIZE
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        let total_size = 1 << self.log_size();
        let values = self.generate_column_values(total_size);

        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_iter(values),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: format!("ch_stacked_col_{}", self.col_index),
        }
    }
}

/// Expands bits from a compact representation to match a mask
/// For example, if mask = 0b1010 and bits = 0b11, returns 0b1010
fn expand_bits_to_mask(bits: u32, mask: u32) -> u32 {
    let mut result = 0u32;
    let mut bit_index = 0;

    for i in 0..16 {
        if (mask >> i) & 1 == 1 {
            if (bits >> bit_index) & 1 == 1 {
                result |= 1 << i;
            }
            bit_index += 1;
        }
    }

    result
}

/// Compresses a value according to a mask, extracting only the bits set in the mask
/// For example, if mask = 0b1010 and value = 0b1010, returns 0b11
fn compress_value_to_mask(value: u32, mask: u32) -> u32 {
    let mut result = 0u32;
    let mut bit_index = 0;

    for i in 0..16 {
        if (mask >> i) & 1 == 1 {
            if (value >> i) & 1 == 1 {
                result |= 1 << bit_index;
            }
            bit_index += 1;
        }
    }

    result
}
