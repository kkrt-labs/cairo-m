/// Macro to generate bitwise components with minimal boilerplate
///
/// This macro generates all the necessary structures and implementations for a bitwise component.
/// It takes the operation name, module name, relation type, and the operation function.
#[macro_export]
macro_rules! define_bitwise {
    ($op_name:ident, $name:ident, $relation_type:ty, $op_fn:expr) => {
        paste::paste! {
            use std::sync::atomic::{AtomicU32, Ordering};

            use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
            use rayon::slice::ParallelSlice;
            use serde::{Deserialize, Serialize};
            use stwo_constraint_framework::logup::LogupTraceGenerator;
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

            use $crate::preprocessed::bitwise::{BITWISE_LOOKUP_BITS, BITWISE_OPERAND_BITS, BITWISE_OPERAND_MASK};

            const LOG_SIZE_BITWISE: u32 = BITWISE_LOOKUP_BITS;

            pub struct InteractionClaimData {
                pub [<bitwise_ $op_name>]: Vec<[PackedM31; 4]>, // input1, input2, result, multiplicity
            }

            #[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
            pub struct Claim {
                pub log_size: u32,
            }

            impl Claim {
                pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
                    let trace = vec![self.log_size; 4]; // 4 columns: input1, input2, result, multiplicity
                    let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
                    TreeVec::new(vec![vec![], trace, interaction_trace])
                }

                pub fn mix_into(&self, channel: &mut impl Channel) {
                    channel.mix_u64(self.log_size as u64);
                }

                /// Writes the preprocessed bitwise_{op_name} trace
                ///
                /// lookup_data contains all bitwise operations made in other components during main trace generation
                /// Each entry is [input1, input2] where both are BITWISE_OPERAND_BITS-bit values
                ///
                /// write_trace creates columns for:
                /// - All possible BITWISE_OPERAND_BITS × BITWISE_OPERAND_BITS combinations (2^BITWISE_LOOKUP_BITS entries)
                /// - input1: first 8-bit operand
                /// - input2: second 8-bit operand
                /// - result: bitwise operation result
                /// - multiplicity: how many times each combination was used
                pub fn write_trace<'a, MC: MerkleChannel>(
                    lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; 2]]>,
                ) -> (
                    Self,
                    [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 4],
                    InteractionClaimData,
                )
                where
                    SimdBackend: BackendForChannel<MC>,
                {
                    // Initialize multiplicities for all 65536 combinations
                    let mults_atomic: Vec<AtomicU32> =
                        (0..1 << LOG_SIZE_BITWISE).map(|_| AtomicU32::new(0)).collect();

                    // Count occurrences of each (input1, input2) pair
                    lookup_data.for_each(|entries| {
                        for entry in entries.iter() {
                            // entry[0] contains packed input1 values
                            // entry[1] contains packed input2 values
                            for (input1, input2) in entry[0].to_array().iter().zip(entry[1].to_array().iter()) {
                                // Compute index from BITWISE_OPERAND_BITS-bit inputs
                                let index = ((input1.0 as usize) << BITWISE_OPERAND_BITS) | (input2.0 as usize);
                                mults_atomic[index].fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    });

                    // Convert atomic multiplicities to M31
                    let mults: Vec<M31> = mults_atomic
                        .into_par_iter()
                        .map(|atomic| M31(atomic.into_inner()))
                        .collect();

                    // Generate all columns
                    let mut input1_col = Vec::with_capacity(1 << LOG_SIZE_BITWISE);
                    let mut input2_col = Vec::with_capacity(1 << LOG_SIZE_BITWISE);
                    let mut result_col = Vec::with_capacity(1 << LOG_SIZE_BITWISE);

                    let op_fn: fn(u32, u32) -> u32 = $op_fn;

                    for i in 0..(1 << LOG_SIZE_BITWISE) {
                        let input1 = (i >> BITWISE_OPERAND_BITS) as u32;
                        let input2 = (i & BITWISE_OPERAND_MASK) as u32;
                        let result = op_fn(input1, input2);

                        input1_col.push(M31(input1));
                        input2_col.push(M31(input2));
                        result_col.push(M31(result));
                    }

                    // Pack data for interaction
                    let packed_data: Vec<[PackedM31; 4]> = (0..(1 << LOG_SIZE_BITWISE))
                        .collect::<Vec<_>>()
                        .par_chunks(N_LANES)
                        .enumerate()
                        .map(|(chunk_idx, _chunk)| {
                            let base_idx = chunk_idx * N_LANES;
                            [
                                PackedM31::from_array(std::array::from_fn(|i| {
                                    input1_col[base_idx + i]
                                })),
                                PackedM31::from_array(std::array::from_fn(|i| {
                                    input2_col[base_idx + i]
                                })),
                                PackedM31::from_array(std::array::from_fn(|i| {
                                    result_col[base_idx + i]
                                })),
                                PackedM31::from_array(std::array::from_fn(|i| {
                                    mults[base_idx + i]
                                })),
                            ]
                        })
                        .collect();

                    let domain = CanonicCoset::new(LOG_SIZE_BITWISE).circle_domain();

                    (
                        Self {
                            log_size: LOG_SIZE_BITWISE,
                        },
                        [
                            CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                                domain,
                                BaseColumn::from_iter(input1_col),
                            ),
                            CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                                domain,
                                BaseColumn::from_iter(input2_col),
                            ),
                            CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                                domain,
                                BaseColumn::from_iter(result_col),
                            ),
                            CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                                domain,
                                BaseColumn::from_iter(mults),
                            ),
                        ],
                        InteractionClaimData {
                            [<bitwise_ $op_name>]: packed_data,
                        },
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
                    [<bitwise_ $op_name>]: &$relation_type,
                    interaction_claim_data: &InteractionClaimData,
                ) -> (
                    Self,
                    impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
                ) {
                    let log_size = interaction_claim_data.[<bitwise_ $op_name>].len().ilog2() + LOG_N_LANES;
                    let mut interaction_trace = LogupTraceGenerator::new(log_size);

                    let mut col = interaction_trace.new_col();
                    (col.par_iter_mut(), &interaction_claim_data.[<bitwise_ $op_name>])
                        .into_par_iter()
                        .for_each(|(writer, value)| {
                            // value[0] = input1, value[1] = input2, value[2] = result, value[3] = multiplicity
                            let denom: PackedQM31 = [<bitwise_ $op_name>].combine(&[value[0], value[1], value[2]]);
                            writer.write_frac(value[3].into(), denom);
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
                pub relation: $relation_type,
                pub claimed_sum: SecureField,
            }

            impl FrameworkEval for Eval {
                fn log_size(&self) -> u32 {
                    self.claim.log_size
                }

                fn max_constraint_log_degree_bound(&self) -> u32 {
                    self.log_size() + 1
                }

                fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
                    // Read the 4 trace columns
                    let input1 = eval.next_trace_mask();
                    let input2 = eval.next_trace_mask();
                    let result = eval.next_trace_mask();
                    let multiplicity = eval.next_trace_mask();

                    // Add lookups to the relation
                    eval.add_to_relation(RelationEntry::new(
                        &self.relation,
                        E::EF::from(multiplicity),
                        &[input1, input2, result],
                    ));

                    eval.finalize_logup();
                    eval
                }
            }

            pub type Component = FrameworkComponent<Eval>;
        }
    };
}
