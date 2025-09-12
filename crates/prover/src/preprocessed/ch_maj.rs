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

// Constants for ch/maj operations
pub const CH_MAJ_OPERAND_BITS: u32 = 2;
pub const CH_LOG_SIZE: u32 = CH_MAJ_OPERAND_BITS * 3; // 24 bits for 3 operands
pub const MAJ_LOG_SIZE: u32 = CH_MAJ_OPERAND_BITS * 3; // 24 bits for 3 operands

// Ch module for ch function: (e & f) ^ (!e & g)
pub mod ch {
    use super::*;

    pub struct InteractionClaimData {
        pub data: Vec<[PackedM31; 5]>, // input1, input2, input3, result, multiplicity
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
            let total_size = 1 << CH_LOG_SIZE;

            // Initialize multiplicities
            let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

            // Count occurrences
            lookup_data.for_each(|entries| {
                for entry in entries.iter() {
                    for i in 0..N_LANES {
                        let e = entry[0].to_array()[i].0 as usize;
                        let f = entry[1].to_array()[i].0 as usize;
                        let g = entry[2].to_array()[i].0 as usize;
                        let index = (e << 16) | (f << 8) | g;
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
            let input1_col = ChCol::new(0).generate_column_values(total_size);
            let input2_col = ChCol::new(1).generate_column_values(total_size);
            let input3_col = ChCol::new(2).generate_column_values(total_size);
            let result_col = ChCol::new(3).generate_column_values(total_size);

            // Pack data
            let packed_data: Vec<[PackedM31; 5]> = (0..total_size)
                .collect::<Vec<_>>()
                .par_chunks(N_LANES)
                .enumerate()
                .map(|(chunk_idx, _)| {
                    let base_idx = chunk_idx * N_LANES;
                    [
                        PackedM31::from_array(std::array::from_fn(|i| input1_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| input2_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| input3_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| result_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i])),
                    ]
                })
                .collect();

            let domain = CanonicCoset::new(CH_LOG_SIZE).circle_domain();

            (
                Self {
                    log_size: CH_LOG_SIZE,
                },
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
            relation: &crate::relations::Ch,
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
                    let denom: PackedQM31 =
                        relation.combine(&[value[0], value[1], value[2], value[3]]);
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
            let ids = Columns.ids();
            let [input1, input2, input3, result] =
                std::array::from_fn(|i| eval.get_preprocessed_column(ids[i].clone()));
            let multiplicity = eval.next_trace_mask();

            eval.add_to_relation(RelationEntry::new(
                &self.relation,
                E::EF::from(multiplicity),
                &[input1, input2, input3, result],
            ));

            eval.finalize_logup();
            eval
        }
    }

    pub type Component = FrameworkComponent<Eval>;

    pub struct Columns;

    impl Columns {
        pub(crate) const fn columns(&self) -> [ChCol; 4] {
            [ChCol::new(0), ChCol::new(1), ChCol::new(2), ChCol::new(3)]
        }

        pub fn ids(&self) -> [PreProcessedColumnId; 4] {
            [
                ChCol::new(0).id(),
                ChCol::new(1).id(),
                ChCol::new(2).id(),
                ChCol::new(3).id(),
            ]
        }
    }

    pub(crate) struct ChCol {
        col_index: usize, // 0: input1(e), 1: input2(f), 2: input3(g), 3: result
    }

    impl ChCol {
        pub const fn new(col_index: usize) -> Self {
            Self { col_index }
        }

        fn generate_column_values(&self, total_size: usize) -> Vec<M31> {
            let mut values = Vec::with_capacity(total_size);

            for e in 0u32..1 << CH_MAJ_OPERAND_BITS {
                for f in 0u32..1 << CH_MAJ_OPERAND_BITS {
                    for g in 0u32..1 << CH_MAJ_OPERAND_BITS {
                        let value = match self.col_index {
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

            values
        }
    }

    impl PreProcessedColumn for ChCol {
        fn log_size(&self) -> u32 {
            CH_LOG_SIZE
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
                id: format!("ch_col_{}", self.col_index),
            }
        }
    }
}

// Maj module for maj function: (a & b) ^ (a & c) ^ (b & c)
pub mod maj {
    use super::*;

    pub struct InteractionClaimData {
        pub data: Vec<[PackedM31; 5]>, // input1, input2, input3, result, multiplicity
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
            let total_size = 1 << MAJ_LOG_SIZE;

            // Initialize multiplicities
            let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

            // Count occurrences
            lookup_data.for_each(|entries| {
                for entry in entries.iter() {
                    for i in 0..N_LANES {
                        let a = entry[0].to_array()[i].0 as usize;
                        let b = entry[1].to_array()[i].0 as usize;
                        let c = entry[2].to_array()[i].0 as usize;
                        let index = (a << 16) | (b << 8) | c;
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
            let input1_col = MajCol::new(0).generate_column_values(total_size);
            let input2_col = MajCol::new(1).generate_column_values(total_size);
            let input3_col = MajCol::new(2).generate_column_values(total_size);
            let result_col = MajCol::new(3).generate_column_values(total_size);

            // Pack data
            let packed_data: Vec<[PackedM31; 5]> = (0..total_size)
                .collect::<Vec<_>>()
                .par_chunks(N_LANES)
                .enumerate()
                .map(|(chunk_idx, _)| {
                    let base_idx = chunk_idx * N_LANES;
                    [
                        PackedM31::from_array(std::array::from_fn(|i| input1_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| input2_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| input3_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| result_col[base_idx + i])),
                        PackedM31::from_array(std::array::from_fn(|i| mults[base_idx + i])),
                    ]
                })
                .collect();

            let domain = CanonicCoset::new(MAJ_LOG_SIZE).circle_domain();

            (
                Self {
                    log_size: MAJ_LOG_SIZE,
                },
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
            relation: &crate::relations::Maj,
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
                    let denom: PackedQM31 =
                        relation.combine(&[value[0], value[1], value[2], value[3]]);
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
        pub relation: crate::relations::Maj,
    }

    impl FrameworkEval for Eval {
        fn log_size(&self) -> u32 {
            self.claim.log_size
        }

        fn max_constraint_log_degree_bound(&self) -> u32 {
            self.log_size() + 1
        }

        fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
            let ids = Columns.ids();
            let [input1, input2, input3, result] =
                std::array::from_fn(|i| eval.get_preprocessed_column(ids[i].clone()));
            let multiplicity = eval.next_trace_mask();

            eval.add_to_relation(RelationEntry::new(
                &self.relation,
                E::EF::from(multiplicity),
                &[input1, input2, input3, result],
            ));

            eval.finalize_logup();
            eval
        }
    }

    pub type Component = FrameworkComponent<Eval>;

    pub struct Columns;

    impl Columns {
        pub(crate) const fn columns(&self) -> [MajCol; 4] {
            [
                MajCol::new(0),
                MajCol::new(1),
                MajCol::new(2),
                MajCol::new(3),
            ]
        }

        pub fn ids(&self) -> [PreProcessedColumnId; 4] {
            [
                MajCol::new(0).id(),
                MajCol::new(1).id(),
                MajCol::new(2).id(),
                MajCol::new(3).id(),
            ]
        }
    }

    pub(crate) struct MajCol {
        col_index: usize, // 0: input1(a), 1: input2(b), 2: input3(c), 3: result
    }

    impl MajCol {
        pub const fn new(col_index: usize) -> Self {
            Self { col_index }
        }

        fn generate_column_values(&self, total_size: usize) -> Vec<M31> {
            let mut values = Vec::with_capacity(total_size);

            for a in 0u32..1 << CH_MAJ_OPERAND_BITS {
                for b in 0u32..1 << CH_MAJ_OPERAND_BITS {
                    for c in 0u32..1 << CH_MAJ_OPERAND_BITS {
                        let value = match self.col_index {
                            0 => M31(a),
                            1 => M31(b),
                            2 => M31(c),
                            3 => {
                                let result = (a & b) ^ (a & c) ^ (b & c);
                                M31(result)
                            }
                            _ => unreachable!(),
                        };
                        values.push(value);
                    }
                }
            }

            values
        }
    }

    impl PreProcessedColumn for MajCol {
        fn log_size(&self) -> u32 {
            MAJ_LOG_SIZE
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
                id: format!("maj_col_{}", self.col_index),
            }
        }
    }
}
