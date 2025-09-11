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

// Macro to generate ch variant structures and implementations
macro_rules! generate_ch_variant {
    ($variant_name:ident, $variant_index:expr, $mask:expr, $relation_type:ident) => {
        pub mod $variant_name {
            use super::*;

            pub struct InteractionClaimData {
                pub data: Vec<[PackedM31; 5]>, // e, f, g, result, multiplicity
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
                    lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; 4]]>,
                ) -> (
                    Self,
                    [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
                    InteractionClaimData,
                )
                where
                    SimdBackend: BackendForChannel<MC>,
                {
                    let mask = $mask;
                    let num_bits = mask.count_ones();
                    let total_size = 1usize << (3 * num_bits);

                    // Initialize multiplicities
                    let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

                    // Count occurrences
                    lookup_data.for_each(|entries| {
                        for entry in entries.iter() {
                            for i in 0..N_LANES {
                                let e = entry[0].to_array()[i].0;
                                let f = entry[1].to_array()[i].0;
                                let g = entry[2].to_array()[i].0;

                                let e_compressed = compress_value_to_mask(e, mask);
                                let f_compressed = compress_value_to_mask(f, mask);
                                let g_compressed = compress_value_to_mask(g, mask);

                                let num_combinations = 1usize << num_bits;
                                let index = (e_compressed as usize) * num_combinations * num_combinations
                                    + (f_compressed as usize) * num_combinations
                                    + (g_compressed as usize);

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
                    let col = Col::new($variant_index);
                    let e_col = col.generate_column_values(total_size, 0);
                    let f_col = col.generate_column_values(total_size, 1);
                    let g_col = col.generate_column_values(total_size, 2);
                    let result_col = col.generate_column_values(total_size, 3);

                    // Pack data
                    let packed_data: Vec<[PackedM31; 5]> = (0..total_size)
                        .collect::<Vec<_>>()
                        .par_chunks(N_LANES)
                        .enumerate()
                        .map(|(chunk_idx, _)| {
                            let base_idx = chunk_idx * N_LANES;
                            [
                                PackedM31::from_array(std::array::from_fn(|i| e_col[base_idx + i])),
                                PackedM31::from_array(std::array::from_fn(|i| f_col[base_idx + i])),
                                PackedM31::from_array(std::array::from_fn(|i| g_col[base_idx + i])),
                                PackedM31::from_array(std::array::from_fn(|i| result_col[base_idx + i])),
                                PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i])),
                            ]
                        })
                        .collect();

                    let log_size = total_size.ilog2();
                    dbg!(&log_size);
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
                            let denom: PackedQM31 = relation.combine(&[value[0], value[1], value[2], value[3]]);
                            writer.write_frac(value[4].into(), denom);
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
                    let ids = Columns::new().ids();
                    let [e, f, g, result] =
                        std::array::from_fn(|i| eval.get_preprocessed_column(ids[i].clone()));
                    let multiplicity = eval.next_trace_mask();

                    eval.add_to_relation(RelationEntry::new(
                        &self.relation,
                        E::EF::from(multiplicity),
                        &[e, f, g, result],
                    ));

                    eval.finalize_logup();
                    eval
                }
            }

            pub type Component = FrameworkComponent<Eval>;

            pub struct Columns;

            impl Columns {
                pub const fn new() -> Self {
                    Self
                }

                pub(crate) const fn columns(&self) -> [Col; 4] {
                    [Col::new(0), Col::new(1), Col::new(2), Col::new(3)]
                }

                pub fn ids(&self) -> [PreProcessedColumnId; 4] {
                    [
                        Col::new(0).id(),
                        Col::new(1).id(),
                        Col::new(2).id(),
                        Col::new(3).id(),
                    ]
                }
            }

            pub(crate) struct Col {
                col_index: usize, // 0: e, 1: f, 2: g, 3: result
                variant_index: usize,
            }

            impl Col {
                pub const fn new(col_index: usize) -> Self {
                    Self { col_index, variant_index: $variant_index }
                }

                fn generate_column_values(&self, total_size: usize, col_type: usize) -> Vec<M31> {
                    let mut values = Vec::with_capacity(total_size);
                    let mask = $mask;
                    let num_bits = mask.count_ones();
                    let num_combinations = 1u32 << num_bits;

                    for e_bits in 0..num_combinations {
                        for f_bits in 0..num_combinations {
                            for g_bits in 0..num_combinations {
                                let e = expand_bits_to_mask(e_bits, mask);
                                let f = expand_bits_to_mask(f_bits, mask);
                                let g = expand_bits_to_mask(g_bits, mask);

                                let value = match col_type {
                                    0 => M31(e),
                                    1 => M31(f),
                                    2 => M31(g),
                                    3 => {
                                        let result = (e & f) ^ ((!e) & g);
                                        M31(result)
                                    }
                                    _ => unreachable!(),
                                };
                                values.push(value);
                            }
                        }
                    }

                    values.resize(total_size, M31(0));
                    values
                }

                pub fn id(&self) -> PreProcessedColumnId {
                    PreProcessedColumnId {
                        id: format!("ch_{}_col_{}", stringify!($variant_name), self.col_index),
                    }
                }
            }

            impl PreProcessedColumn for Col {
                fn log_size(&self) -> u32 {
                    let mask = $mask;
                    let num_bits = mask.count_ones();
                    3 * num_bits
                }

                fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
                    let total_size = 1 << self.log_size();
                    let values = self.generate_column_values(total_size, self.col_index);

                    CircleEvaluation::new(
                        CanonicCoset::new(self.log_size()).circle_domain(),
                        BaseColumn::from_iter(values),
                    )
                }

                fn id(&self) -> PreProcessedColumnId {
                    PreProcessedColumnId {
                        id: format!("ch_{}_col_{}", stringify!($variant_name), self.col_index),
                    }
                }
            }
        }
    };
}

// Generate all 6 ch variants
generate_ch_variant!(ch_l0, 0, MASK_BIG_SIGMA1_L0, ChL0);
generate_ch_variant!(ch_l1, 1, MASK_BIG_SIGMA1_L1, ChL1);
generate_ch_variant!(ch_l2, 2, MASK_BIG_SIGMA1_L2, ChL2);
generate_ch_variant!(ch_h0, 3, MASK_BIG_SIGMA1_H0 >> 16, ChH0);
generate_ch_variant!(ch_h1, 4, MASK_BIG_SIGMA1_H1 >> 16, ChH1);
generate_ch_variant!(ch_h2, 5, MASK_BIG_SIGMA1_H2 >> 16, ChH2);

/// Expands bits from a compact representation to match a mask
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

/// Compresses a value according to a mask
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
