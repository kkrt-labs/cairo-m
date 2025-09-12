use std::sync::atomic::{AtomicU32, Ordering};

use num_traits::Zero;
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

// Masks for XOR operations - these come from sigma outputs
const MASK_SMALL_SIGMA0_OUT2_LO: u32 = 0x800f;
const MASK_SMALL_SIGMA0_OUT2_HI: u32 = 0x800f0000;
const MASK_SMALL_SIGMA1_OUT2_LO: u32 = 0xc060;
const MASK_SMALL_SIGMA1_OUT2_HI: u32 = 0x91200000;
const MASK_BIG_SIGMA0_OUT2_LO: u32 = 0x3843;
const MASK_BIG_SIGMA0_OUT2_HI: u32 = 0x96ad0000;
const MASK_BIG_SIGMA1_OUT2_LO: u32 = 0x610c;
const MASK_BIG_SIGMA1_OUT2_HI: u32 = 0x610c0000;

// Macro to generate XOR variant structures and implementations
macro_rules! generate_xor_variant {
    (
        $variant_name:ident,
        $relation_type:ident,
        $input_masks:expr, // Array of input masks (high masks NOT pre-shifted)
        $num_columns:expr
    ) => {
        pub mod $variant_name {
            use super::*;

            pub struct InteractionClaimData {
                pub data: Vec<[PackedM31; $num_columns + 1]>, // columns + multiplicity
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

                pub fn write_trace<'a, MC: MerkleChannel>(
                    lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; $num_columns]]>,
                ) -> (
                    Self,
                    [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
                    InteractionClaimData,
                )
                where
                    SimdBackend: BackendForChannel<MC>,
                {
                    let input_masks = $input_masks;
                    // For high masks (> 0xFFFF), we need to shift them for bit counting
                    let total_bits = input_masks.iter().map(|m| {
                        if *m > 0xFFFF {
                            (*m >> 16).count_ones()
                        } else {
                            m.count_ones()
                        }
                    }).sum::<u32>();
                    let total_size = 1usize << total_bits;

                    // Initialize multiplicities
                    let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

                    // Count occurrences
                    lookup_data.for_each(|entries| {
                        for entry in entries.iter() {
                            for i in 0..N_LANES {
                                let mut index = 0usize;
                                let mut shift = 0u32;

                                // Compress each input to form the index
                                for (col_idx, mask) in input_masks.iter().enumerate() {
                                    let value = entry[col_idx].to_array()[i].0;
                                    // For high masks, we need to use the shifted version for compression
                                    let (compressed, bits) = if *mask > 0xFFFF {
                                        (compress_value_to_mask(value, *mask >> 16), (*mask >> 16).count_ones())
                                    } else {
                                        (compress_value_to_mask(value, *mask), mask.count_ones())
                                    };
                                    index |= (compressed as usize) << shift;
                                    shift += bits;
                                }

                                if index < total_size {
                                    mults_atomic[index].fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    });

                    // Convert atomic multiplicities to M31
                    let mults: Vec<M31> = mults_atomic
                        .into_par_iter()
                        .map(|atomic| M31(atomic.into_inner()))
                        .collect();

                    // Generate columns
                    let columns: Vec<Vec<M31>> = (0..$num_columns)
                        .map(|col_idx| {
                            XorCol::new(col_idx).generate_column_values(total_size)
                        })
                        .collect();

                    // Pack data
                    let packed_data: Vec<[PackedM31; $num_columns + 1]> = (0..total_size)
                        .collect::<Vec<_>>()
                        .par_chunks(N_LANES)
                        .enumerate()
                        .map(|(chunk_idx, _)| {
                            let base_idx = chunk_idx * N_LANES;
                            let mut result = [PackedM31::from(M31::zero()); $num_columns + 1];

                            for col_idx in 0..$num_columns {
                                result[col_idx] = PackedM31::from_array(std::array::from_fn(|i| columns[col_idx][base_idx + i]));
                            }
                            result[$num_columns] = PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i]));

                            result
                        })
                        .collect();

                    let log_size = total_size.ilog2();
                    let domain = CanonicCoset::new(log_size).circle_domain();

                    (
                        Self { log_size },
                        [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                            domain,
                            BaseColumn::from_iter(mults),
                        )],
                        InteractionClaimData { data: packed_data },
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
                    relation: &crate::relations::$relation_type,
                    interaction_claim_data: &InteractionClaimData,
                ) -> (
                    Self,
                    impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                ) {
                    let log_size = interaction_claim_data.data.len().ilog2() + LOG_N_LANES;
                    let mut interaction_trace = LogupTraceGenerator::new(log_size);

                    let mut col = interaction_trace.new_col();
                    (col.par_iter_mut(), &interaction_claim_data.data)
                        .into_par_iter()
                        .for_each(|(writer, value)| {
                            let cols: Vec<PackedM31> = (0..$num_columns)
                                .map(|i| value[i])
                                .collect();
                            let denom: PackedQM31 = relation.combine(&cols);
                            writer.write_frac(value[$num_columns].into(), denom);
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
                pub relation: crate::relations::$relation_type,
            }

            impl FrameworkEval for Eval {
                fn log_size(&self) -> u32 {
                    self.claim.log_size
                }

                fn max_constraint_log_degree_bound(&self) -> u32 {
                    self.log_size() + 1
                }

                fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
                    let ids = Columns::default().ids();
                    let cols: Vec<_> = (0..$num_columns)
                        .map(|i| eval.get_preprocessed_column(ids[i].clone()))
                        .collect();
                    let multiplicity = eval.next_trace_mask();

                    eval.add_to_relation(RelationEntry::new(
                        &self.relation,
                        E::EF::from(multiplicity),
                        &cols,
                    ));

                    eval.finalize_logup();
                    eval
                }
            }

            pub type Component = FrameworkComponent<Eval>;

            #[derive(Default)]
            pub struct Columns;

            impl Columns {
                pub fn ids(&self) -> [PreProcessedColumnId; $num_columns] {
                    std::array::from_fn(|i| XorCol::new(i).id())
                }
            }

            pub(crate) struct XorCol {
                col_index: usize,
            }

            impl XorCol {
                pub const fn new(col_index: usize) -> Self {
                    Self { col_index }
                }

                fn generate_column_values(&self, total_size: usize) -> Vec<M31> {
                    let input_masks = $input_masks;
                    let mut values = Vec::with_capacity(total_size);

                    // Calculate total combinations (handle high masks by shifting)
                    let input_bit_counts: Vec<u32> = input_masks.iter().map(|m| {
                        if *m > 0xFFFF {
                            (*m >> 16).count_ones()
                        } else {
                            m.count_ones()
                        }
                    }).collect();
                    let total_combinations: Vec<u32> = input_bit_counts.iter().map(|&bits| 1u32 << bits).collect();

                    // Generate all possible input combinations
                    let mut indices = vec![0u32; input_masks.len()];

                    for _ in 0..total_size {
                        // Expand indices to get input values
                        // For high masks, we need to expand using the shifted mask
                        let inputs: Vec<u32> = indices.iter().zip(input_masks.iter())
                            .map(|(&idx, &mask)| {
                                if mask > 0xFFFF {
                                    expand_bits_to_mask(idx, mask >> 16)
                                } else {
                                    expand_bits_to_mask(idx, mask)
                                }
                            })
                            .collect();

                        // Compute the value for this column
                        let value = if $num_columns == 6 {
                            // For 6-column XOR (small sigma and big sigma1)
                            // Columns are: out2_0_lo, out2_0_hi, out2_1_lo, out2_1_hi, xor_out2_lo, xor_out2_hi
                            match self.col_index {
                                0..=3 => M31(inputs[self.col_index]), // Direct input columns
                                4 => M31(inputs[0] ^ inputs[2]), // xor_out2_lo = out2_0_lo ^ out2_1_lo
                                5 => M31(inputs[1] ^ inputs[3]), // xor_out2_hi = out2_0_hi ^ out2_1_hi
                                _ => unreachable!(),
                            }
                        } else if $num_columns == 3 {
                            // For 3-column XOR (big sigma0 separate xor)
                            // Columns are either: out2_0_lo, out2_1_lo, xor_out2_lo
                            // or: out2_0_hi, out2_1_hi, xor_out2_hi
                            match self.col_index {
                                0..=1 => M31(inputs[self.col_index]), // Direct input columns
                                2 => M31(inputs[0] ^ inputs[1]), // xor result
                                _ => unreachable!(),
                            }
                        } else {
                            unreachable!()
                        };

                        values.push(value);

                        // Increment indices
                        let mut carry = true;
                        for i in 0..indices.len() {
                            if carry {
                                indices[i] += 1;
                                if indices[i] >= total_combinations[i] {
                                    indices[i] = 0;
                                } else {
                                    carry = false;
                                }
                            }
                        }
                    }

                    values
                }
            }

            impl PreProcessedColumn for XorCol {
                fn log_size(&self) -> u32 {
                    let input_masks = $input_masks;
                    input_masks.iter().map(|m| {
                        if *m > 0xFFFF {
                            (*m >> 16).count_ones()
                        } else {
                            m.count_ones()
                        }
                    }).sum::<u32>()
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
                        id: format!("{}_col_{}", stringify!($variant_name), self.col_index),
                    }
                }
            }
        }
    };
}

/// Expands bits from a compact representation to match a mask
fn expand_bits_to_mask(bits: u32, mask: u32) -> u32 {
    let mut result = 0u32;
    let mut bit_index = 0;

    for i in 0..32 {
        if (mask >> i) & 1 == 1 {
            if (bits >> bit_index) & 1 == 1 {
                result |= 1 << i;
            }
            bit_index += 1;
        }
    }

    result
}

/// Compresses a value according to a mask
fn compress_value_to_mask(value: u32, mask: u32) -> u32 {
    let mut result = 0u32;
    let mut bit_index = 0;

    for i in 0..32 {
        if (mask >> i) & 1 == 1 {
            if (value >> i) & 1 == 1 {
                result |= 1 << bit_index;
            }
            bit_index += 1;
        }
    }

    result
}

// XOR for SmallSigma0: 6 columns (out2_0_lo, out2_0_hi, out2_1_lo, out2_1_hi, xor_out2_lo, xor_out2_hi)
generate_xor_variant!(
    xor_small_sigma0,
    XorSmallSigma0,
    [
        MASK_SMALL_SIGMA0_OUT2_LO,
        MASK_SMALL_SIGMA0_OUT2_HI, // Keep original un-shifted mask
        MASK_SMALL_SIGMA0_OUT2_LO,
        MASK_SMALL_SIGMA0_OUT2_HI // Keep original un-shifted mask
    ],
    6
);

// XOR for SmallSigma1: 6 columns (out2_0_lo, out2_0_hi, out2_1_lo, out2_1_hi, xor_out2_lo, xor_out2_hi)
generate_xor_variant!(
    xor_small_sigma1,
    XorSmallSigma1,
    [
        MASK_SMALL_SIGMA1_OUT2_LO,
        MASK_SMALL_SIGMA1_OUT2_HI, // Keep original un-shifted mask
        MASK_SMALL_SIGMA1_OUT2_LO,
        MASK_SMALL_SIGMA1_OUT2_HI // Keep original un-shifted mask
    ],
    6
);

// XOR for BigSigma0 low part: 3 columns (out2_0_lo, out2_1_lo, xor_out2_lo)
generate_xor_variant!(
    xor_big_sigma0_0,
    XorBigSigma0_0,
    [MASK_BIG_SIGMA0_OUT2_LO, MASK_BIG_SIGMA0_OUT2_LO],
    3
);

// XOR for BigSigma0 high part: 3 columns (out2_0_hi, out2_1_hi, xor_out2_hi)
generate_xor_variant!(
    xor_big_sigma0_1,
    XorBigSigma0_1,
    [
        MASK_BIG_SIGMA0_OUT2_HI, // Keep original un-shifted mask
        MASK_BIG_SIGMA0_OUT2_HI  // Keep original un-shifted mask
    ],
    3
);

// XOR for BigSigma1: 6 columns (out2_0_lo, out2_0_hi, out2_1_lo, out2_1_hi, xor_out2_lo, xor_out2_hi)
generate_xor_variant!(
    xor_big_sigma1,
    XorBigSigma1,
    [
        MASK_BIG_SIGMA1_OUT2_LO,
        MASK_BIG_SIGMA1_OUT2_HI, // Keep original un-shifted mask
        MASK_BIG_SIGMA1_OUT2_LO,
        MASK_BIG_SIGMA1_OUT2_HI // Keep original un-shifted mask
    ],
    6
);
