use rayon::iter::ParallelIterator;
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::TreeVec;

#[derive(Copy, Clone)]
pub struct Claim<const N: usize> {
    pub log_size: u32,
}

impl<const N: usize> Claim<N> {
    pub const fn new(log_size: u32) -> Self {
        Self { log_size }
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace_log_sizes = vec![self.log_size; N];
        TreeVec::new(vec![vec![], trace_log_sizes, vec![]])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    #[allow(non_snake_case)]
    pub fn write_trace<MC: MerkleChannel>(&self) -> ComponentTrace<N>
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let mut trace = unsafe { ComponentTrace::<N>::uninitialized(self.log_size) };
        let M31_0 = PackedM31::broadcast(M31::from(0));
        let M31_1 = PackedM31::broadcast(M31::from(1));
        trace.par_iter_mut().for_each(|mut row| {
            for i in (0..N).step_by(3) {
                *row[i] = M31_0;
                *row[i + 1] = M31_1;
                *row[i + 2] = M31_1;
            }
        });
        trace
    }
}

pub struct Eval<const N: usize> {
    pub claim: Claim<N>,
}

impl<const N: usize> FrameworkEval for Eval<N> {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    #[allow(non_snake_case)]
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        for _ in (0..N).step_by(3) {
            let M31_1 = E::F::from(M31::from(1));
            let M31_2 = E::F::from(M31::from(2));

            let col0 = eval.next_trace_mask();
            let col1 = eval.next_trace_mask();
            let col2 = eval.next_trace_mask();

            // col0, col1, col2 are all 0 or 1.
            eval.add_constraint(col0.clone() * (M31_1.clone() - col0.clone()));
            eval.add_constraint(col1.clone() * (M31_1.clone() - col1.clone()));
            eval.add_constraint(col2.clone() * (M31_1.clone() - col2.clone()));

            // col0 or col1 and col2
            eval.add_constraint(col0.clone() + col1.clone() - col2.clone());
            // col0 or col2 and col1
            eval.add_constraint(col0.clone() + col2.clone() - col1.clone());
            // col1 and col2
            eval.add_constraint(col1.clone() + col2.clone() - M31_2.clone());
            eval.add_constraint(col1.clone() * col2.clone() - M31_1.clone());
        }

        eval
    }
}

pub type Component<const N: usize> = FrameworkComponent<Eval<N>>;
