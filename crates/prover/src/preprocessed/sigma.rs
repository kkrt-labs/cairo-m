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

// Masks from witness.rs for sigma functions
const MASK_SMALL_SIGMA0_L0: u32 = 0x4aaa;
const MASK_SMALL_SIGMA0_L1: u32 = 0x155;
const MASK_SMALL_SIGMA0_L2: u32 = 0xb400;
const MASK_SMALL_SIGMA0_H0: u32 = 0x550000;
const MASK_SMALL_SIGMA0_H1: u32 = 0xb5000000;
const MASK_SMALL_SIGMA0_H2: u32 = 0x4aaa0000;
const MASK_SMALL_SIGMA0_OUT0_LO: u32 = 0x2aa0;
const MASK_SMALL_SIGMA0_OUT1_LO: u32 = 0x5550;
const MASK_SMALL_SIGMA0_OUT0_HI: u32 = 0x55500000;
const MASK_SMALL_SIGMA0_OUT1_HI: u32 = 0x2aa00000;
const MASK_SMALL_SIGMA0_OUT2_LO: u32 = 0x800f;
const MASK_SMALL_SIGMA0_OUT2_HI: u32 = 0x800f0000;

const MASK_SMALL_SIGMA1_L0: u32 = 0x4285;
const MASK_SMALL_SIGMA1_L1: u32 = 0x17a;
const MASK_SMALL_SIGMA1_L2: u32 = 0xbc00;
const MASK_SMALL_SIGMA1_H0: u32 = 0x4aa40000;
const MASK_SMALL_SIGMA1_H1: u32 = 0x15a0000;
const MASK_SMALL_SIGMA1_H2: u32 = 0xb4000000;
const MASK_SMALL_SIGMA1_OUT0_LO: u32 = 0x150a;
const MASK_SMALL_SIGMA1_OUT1_LO: u32 = 0x2a95;
const MASK_SMALL_SIGMA1_OUT0_HI: u32 = 0x40a0000;
const MASK_SMALL_SIGMA1_OUT1_HI: u32 = 0x6ad50000;
const MASK_SMALL_SIGMA1_OUT2_LO: u32 = 0xc060;
const MASK_SMALL_SIGMA1_OUT2_HI: u32 = 0x91200000;

const MASK_BIG_SIGMA0_L0: u32 = 0x7292;
const MASK_BIG_SIGMA0_L1: u32 = 0x6d;
const MASK_BIG_SIGMA0_L2: u32 = 0x8d00;
const MASK_BIG_SIGMA0_H0: u32 = 0xd60000;
const MASK_BIG_SIGMA0_H1: u32 = 0x9c000000;
const MASK_BIG_SIGMA0_H2: u32 = 0x63290000;
const MASK_BIG_SIGMA0_OUT0_LO: u32 = 0x4318;
const MASK_BIG_SIGMA0_OUT1_LO: u32 = 0x84a4;
const MASK_BIG_SIGMA0_OUT0_HI: u32 = 0x48420000;
const MASK_BIG_SIGMA0_OUT1_HI: u32 = 0x21100000;
const MASK_BIG_SIGMA0_OUT2_LO: u32 = 0x3843;
const MASK_BIG_SIGMA0_OUT2_HI: u32 = 0x96ad0000;

const MASK_BIG_SIGMA1_L0: u32 = 0xf83;
const MASK_BIG_SIGMA1_L1: u32 = 0x7c;
const MASK_BIG_SIGMA1_L2: u32 = 0xf000;
const MASK_BIG_SIGMA1_H0: u32 = 0x7c0000;
const MASK_BIG_SIGMA1_H1: u32 = 0xf0000000;
const MASK_BIG_SIGMA1_H2: u32 = 0xf830000;
const MASK_BIG_SIGMA1_OUT0_LO: u32 = 0x1e03;
const MASK_BIG_SIGMA1_OUT1_LO: u32 = 0x80f0;
const MASK_BIG_SIGMA1_OUT0_HI: u32 = 0x80f00000;
const MASK_BIG_SIGMA1_OUT1_HI: u32 = 0x1e030000;
const MASK_BIG_SIGMA1_OUT2_LO: u32 = 0x610c;
const MASK_BIG_SIGMA1_OUT2_HI: u32 = 0x610c0000;

// Macro to generate sigma variant structures and implementations
macro_rules! generate_sigma_variant {
    (
        $variant_name:ident,
        $relation_type:ident,
        $input_masks:expr, // Array of input masks
        $output_masks:expr, // Array of output masks
        $sigma_func:expr,
        $num_input_columns:expr,
        $num_output_columns:expr
    ) => {
        pub mod $variant_name {
            use super::*;

            pub struct InteractionClaimData {
                pub data: Vec<[PackedM31; $num_input_columns + $num_output_columns + 1]>, // inputs, outputs, multiplicity
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
                    lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; $num_input_columns + $num_output_columns]]>,
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
                    let columns: Vec<Vec<M31>> = (0..$num_input_columns + $num_output_columns)
                        .map(|col_idx| {
                            SigmaCol::new(col_idx).generate_column_values(total_size)
                        })
                        .collect();

                    // Pack data
                    let packed_data: Vec<[PackedM31; $num_input_columns + $num_output_columns + 1]> = (0..total_size)
                        .collect::<Vec<_>>()
                        .par_chunks(N_LANES)
                        .enumerate()
                        .map(|(chunk_idx, _)| {
                            let base_idx = chunk_idx * N_LANES;
                            let mut result = [PackedM31::from(M31::zero()); $num_input_columns + $num_output_columns + 1];

                            for col_idx in 0..($num_input_columns + $num_output_columns) {
                                result[col_idx] = PackedM31::from_array(std::array::from_fn(|i| columns[col_idx][base_idx + i]));
                            }
                            result[$num_input_columns + $num_output_columns] = PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i]));

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
                            let cols: Vec<PackedM31> = (0..($num_input_columns + $num_output_columns))
                                .map(|i| value[i])
                                .collect();
                            let denom: PackedQM31 = relation.combine(&cols);
                            writer.write_frac(value[$num_input_columns + $num_output_columns].into(), denom);
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
                    let cols: Vec<_> = (0..($num_input_columns + $num_output_columns))
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
                pub fn ids(&self) -> [PreProcessedColumnId; $num_input_columns + $num_output_columns] {
                    std::array::from_fn(|i| SigmaCol::new(i).id())
                }
            }

            pub(crate) struct SigmaCol {
                col_index: usize,
            }

            impl SigmaCol {
                pub const fn new(col_index: usize) -> Self {
                    Self { col_index }
                }

                fn generate_column_values(&self, total_size: usize) -> Vec<M31> {
                    let input_masks = $input_masks;
                    let output_masks = $output_masks;
                    let sigma_func = $sigma_func;

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

                        let value = if self.col_index < $num_input_columns {
                            // Input column
                            M31(inputs[self.col_index])
                        } else {
                            // Output column
                            let output_idx = self.col_index - $num_input_columns;

                            // Rebuild the full word from inputs and apply sigma
                            // High masks need to be shifted back up to their correct position
                            let mut word = 0u32;
                            for (input, mask) in inputs.iter().zip(input_masks.iter()) {
                                if *mask > 0xFFFF {
                                    // This is a high mask, shift the input up
                                    word |= input << 16;
                                } else {
                                    // This is a low mask, use as-is
                                    word |= input;
                                }
                            }
                            let sigma_result = sigma_func(word);

                            // Extract the specific output using the mask
                            let output_value = sigma_result & output_masks[output_idx];
                            // Shift if it's a high mask
                            if output_masks[output_idx] > 0xFFFF {
                                M31(output_value >> 16)
                            } else {
                                M31(output_value)
                            }
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

            impl PreProcessedColumn for SigmaCol {
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

// Sigma functions
const fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(3) ^ x.rotate_right(7) ^ x.rotate_right(18)
}

const fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(10) ^ x.rotate_right(17) ^ x.rotate_right(19)
}

const fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

const fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

// Generate sigma variants based on witness.rs patterns
// SmallSigma0_0: Takes l1, l2, h2 -> outputs out0_lo, out0_hi, out2_0_lo, out2_0_hi
generate_sigma_variant!(
    small_sigma0_0,
    SmallSigma0_0,
    [
        MASK_SMALL_SIGMA0_L1,
        MASK_SMALL_SIGMA0_L2,
        MASK_SMALL_SIGMA0_H2 // Keep original unshifted mask
    ],
    [
        MASK_SMALL_SIGMA0_OUT0_LO,
        MASK_SMALL_SIGMA0_OUT0_HI, // Keep original unshifted mask
        MASK_SMALL_SIGMA0_OUT2_LO,
        MASK_SMALL_SIGMA0_OUT2_HI // Keep original unshifted mask
    ],
    small_sigma0,
    3,
    4
);

// SmallSigma0_1: Takes l0, h0, h1 -> outputs out1_lo, out1_hi, out2_1_lo, out2_1_hi
generate_sigma_variant!(
    small_sigma0_1,
    SmallSigma0_1,
    [
        MASK_SMALL_SIGMA0_L0,
        MASK_SMALL_SIGMA0_H0, // Keep original unshifted mask
        MASK_SMALL_SIGMA0_H1  // Keep original unshifted mask
    ],
    [
        MASK_SMALL_SIGMA0_OUT1_LO,
        MASK_SMALL_SIGMA0_OUT1_HI, // Keep original unshifted mask
        MASK_SMALL_SIGMA0_OUT2_LO,
        MASK_SMALL_SIGMA0_OUT2_HI // Keep original unshifted mask
    ],
    small_sigma0,
    3,
    4
);

// SmallSigma1_0: Takes l0, h0 -> outputs out0_lo, out0_hi, out2_0_lo, out2_0_hi
generate_sigma_variant!(
    small_sigma1_0,
    SmallSigma1_0,
    [MASK_SMALL_SIGMA1_L0, MASK_SMALL_SIGMA1_H0], // Keep original unshifted mask
    [
        MASK_SMALL_SIGMA1_OUT0_LO,
        MASK_SMALL_SIGMA1_OUT0_HI, // Keep original unshifted mask
        MASK_SMALL_SIGMA1_OUT2_LO,
        MASK_SMALL_SIGMA1_OUT2_HI // Keep original unshifted mask
    ],
    small_sigma1,
    2,
    4
);

// SmallSigma1_1: Takes l1, l2, h1, h2 -> outputs out1_lo, out1_hi, out2_1_lo, out2_1_hi
generate_sigma_variant!(
    small_sigma1_1,
    SmallSigma1_1,
    [
        MASK_SMALL_SIGMA1_L1,
        MASK_SMALL_SIGMA1_L2,
        MASK_SMALL_SIGMA1_H1, // Keep original unshifted mask
        MASK_SMALL_SIGMA1_H2  // Keep original unshifted mask
    ],
    [
        MASK_SMALL_SIGMA1_OUT1_LO,
        MASK_SMALL_SIGMA1_OUT1_HI, // Keep original unshifted mask
        MASK_SMALL_SIGMA1_OUT2_LO,
        MASK_SMALL_SIGMA1_OUT2_HI // Keep original unshifted mask
    ],
    small_sigma1,
    4,
    4
);

// BigSigma0_0: Takes l1, l2, h2 -> outputs out0_lo, out0_hi, out2_0_lo, out2_0_hi
generate_sigma_variant!(
    big_sigma0_0,
    BigSigma0_0,
    [
        MASK_BIG_SIGMA0_L1,
        MASK_BIG_SIGMA0_L2,
        MASK_BIG_SIGMA0_H2 // Keep original unshifted mask
    ],
    [
        MASK_BIG_SIGMA0_OUT0_LO,
        MASK_BIG_SIGMA0_OUT0_HI, // Keep original unshifted mask
        MASK_BIG_SIGMA0_OUT2_LO,
        MASK_BIG_SIGMA0_OUT2_HI // Keep original unshifted mask
    ],
    big_sigma0,
    3,
    4
);

// BigSigma0_1: Takes l0, h0, h1 -> outputs out1_lo, out1_hi, out2_1_lo, out2_1_hi
generate_sigma_variant!(
    big_sigma0_1,
    BigSigma0_1,
    [
        MASK_BIG_SIGMA0_L0,
        MASK_BIG_SIGMA0_H0, // Keep original unshifted mask
        MASK_BIG_SIGMA0_H1  // Keep original unshifted mask
    ],
    [
        MASK_BIG_SIGMA0_OUT1_LO,
        MASK_BIG_SIGMA0_OUT1_HI, // Keep original unshifted mask
        MASK_BIG_SIGMA0_OUT2_LO,
        MASK_BIG_SIGMA0_OUT2_HI // Keep original unshifted mask
    ],
    big_sigma0,
    3,
    4
);

// BigSigma1_0: Takes l0, h0, h1 -> outputs out0_lo, out0_hi, out2_0_lo, out2_0_hi
generate_sigma_variant!(
    big_sigma1_0,
    BigSigma1_0,
    [
        MASK_BIG_SIGMA1_L0,
        MASK_BIG_SIGMA1_H0, // Keep original unshifted mask
        MASK_BIG_SIGMA1_H1  // Keep original unshifted mask
    ],
    [
        MASK_BIG_SIGMA1_OUT0_LO,
        MASK_BIG_SIGMA1_OUT0_HI, // Keep original unshifted mask
        MASK_BIG_SIGMA1_OUT2_LO,
        MASK_BIG_SIGMA1_OUT2_HI // Keep original unshifted mask
    ],
    big_sigma1,
    3,
    4
);

// BigSigma1_1: Takes l1, l2, h2 -> outputs out1_lo, out1_hi, out2_1_lo, out2_1_hi
generate_sigma_variant!(
    big_sigma1_1,
    BigSigma1_1,
    [
        MASK_BIG_SIGMA1_L1,
        MASK_BIG_SIGMA1_L2,
        MASK_BIG_SIGMA1_H2 // Keep original unshifted mask
    ],
    [
        MASK_BIG_SIGMA1_OUT1_LO,
        MASK_BIG_SIGMA1_OUT1_HI, // Keep original unshifted mask
        MASK_BIG_SIGMA1_OUT2_LO,
        MASK_BIG_SIGMA1_OUT2_HI // Keep original unshifted mask
    ],
    big_sigma1,
    3,
    4
);
