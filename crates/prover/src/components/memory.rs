use std::collections::HashMap;

use num_traits::One;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::relations;

#[derive(Copy, Clone, Default)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

pub struct LookupData {
    pub memory: Vec<[PackedM31; 3]>,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; 3];
        let interaction_log_sizes = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    #[allow(non_snake_case)]
    pub fn write_trace<MC: MerkleChannel>(
        &mut self,
        lookup_data: &Vec<[PackedM31; 2]>,
    ) -> (
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 3],
        LookupData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let mut counts: HashMap<(u32, u32), u32> = HashMap::new();
        for entry in lookup_data {
            let arr0 = entry[0].to_array();
            let arr1 = entry[1].to_array();
            for (val0, val1) in arr0.iter().zip(arr1.iter()) {
                let key = (val0.0, val1.0);
                *counts.entry(key).or_insert(0) += 1;
            }
        }

        let unique_pairs: Vec<_> = counts.into_iter().collect();
        let mut addresses = Vec::new();
        let mut values = Vec::new();
        let mut mults = Vec::new();
        let mut memory = Vec::new();

        for chunk in unique_pairs.chunks(N_LANES) {
            // If chunk is smaller than N_LANES, the remaining slots stay as M31(0)
            let mut address = [M31(0); N_LANES];
            let mut value = [M31(0); N_LANES];
            let mut mult = [M31(0); N_LANES];

            for (i, &((_address, _value), _mult)) in chunk.iter().enumerate() {
                address[i] = M31(_address);
                value[i] = M31(_value);
                mult[i] = M31(_mult);
            }

            addresses.push(PackedM31::from_array(address));
            values.push(PackedM31::from_array(value));
            mults.push(PackedM31::from_array(mult));
            memory.push([
                addresses.last().unwrap().to_owned(),
                values.last().unwrap().to_owned(),
                mults.last().unwrap().to_owned(),
            ]);
        }

        self.log_size = addresses.len().ilog2() + LOG_N_LANES;
        let domain = CanonicCoset::new(self.log_size).circle_domain();

        (
            [
                CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                    domain,
                    BaseColumn::from_simd(addresses),
                ),
                CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                    domain,
                    BaseColumn::from_simd(values),
                ),
                CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                    domain,
                    BaseColumn::from_simd(mults),
                ),
            ],
            LookupData { memory },
        )
    }
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        memory: &relations::Memory,
        lookup_data: &LookupData,
    ) -> (
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        Self,
    ) {
        let log_size = lookup_data.memory.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.memory)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = memory.combine(&value[0..2]);
                writer.write_frac(-PackedQM31::one() * value[2], denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, Self { claimed_sum })
    }
}

pub struct Eval {
    pub claim: Claim,
    pub memory: relations::Memory,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let address = eval.next_trace_mask();
        let value = eval.next_trace_mask();
        let mult = eval.next_trace_mask();

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            -E::EF::from(mult),
            &[address, value],
        ));

        eval.finalize_logup();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
