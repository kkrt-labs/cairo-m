use num_traits::identities::One;
use num_traits::Zero;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use stwo_air_utils::trace::component_trace::ComponentTrace;
use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::BaseField;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::relations;

#[derive(Copy, Clone)]
pub struct Claim {
    pub log_size: u32,
}

#[derive(Copy, Clone)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

/// A container to hold the looked up data during main trace generation.
/// It is then used to generate the interaction trace once the challenge has been drawn.
#[derive(Uninitialized, IterMut, ParIterMut)]
pub struct LookupData {
    pub memory: Vec<[PackedM31; 2]>,
}

impl Claim {
    pub const fn new(log_size: u32) -> Self {
        Self { log_size }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; 3];
        let interaction_log_sizes = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace_log_sizes, interaction_log_sizes])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    #[allow(non_snake_case)]
    pub fn write_trace<MC: MerkleChannel>(&self) -> (ComponentTrace<3>, LookupData)
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let (mut trace, mut lookup_data) = unsafe {
            (
                ComponentTrace::<3>::uninitialized(self.log_size),
                LookupData::uninitialized(self.log_size - LOG_N_LANES),
            )
        };

        let M31_0 = PackedM31::broadcast(Zero::zero());
        let M31_1 = PackedM31::broadcast(One::one());

        (trace.par_iter_mut(), lookup_data.par_iter_mut())
            .into_par_iter()
            .for_each(|(mut row, lookup_data)| {
                *row[0] = M31_0;
                *row[1] = M31_1;
                *row[2] = M31_1;
                *lookup_data.memory = [M31_0, M31_1];
            });
        (trace, lookup_data)
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
        let log_size = lookup_data.memory.len().ilog2();
        let mut interaction_trace = LogupTraceGenerator::new(log_size + LOG_N_LANES);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &lookup_data.memory)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = memory.combine(value);
                writer.write_frac(PackedQM31::one(), denom);
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
        let col0 = eval.next_trace_mask();
        let col1 = eval.next_trace_mask();
        let col2 = eval.next_trace_mask();

        eval.add_constraint(col0.clone() + col1.clone() - col2);

        eval.add_to_relation(RelationEntry::new(
            &self.memory,
            E::EF::one(),
            &[col0, col1],
        ));

        eval.finalize_logup();

        eval
    }
}

pub type Component = FrameworkComponent<Eval>;
