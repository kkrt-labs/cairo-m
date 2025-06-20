use num_traits::{One, Zero};
use rayon::iter::ParallelIterator;
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
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
        let M31_0 = PackedM31::broadcast(Zero::zero());
        let M31_1 = PackedM31::broadcast(One::one());
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

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        for _ in (0..N).step_by(3) {
            let col0 = eval.next_trace_mask();
            let col1 = eval.next_trace_mask();
            let col2 = eval.next_trace_mask();

            eval.add_constraint(col0.clone() + col1.clone() - col2.clone());
        }

        eval
    }
}

pub type Component<const N: usize> = FrameworkComponent<Eval<N>>;
