// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/assert_constraints.rs
#![allow(unused)]

use std::ops::Deref;

use itertools::Itertools;
use stwo_constraint_framework::{
    assert_constraints_on_trace, FrameworkComponent, FrameworkEval, TraceLocationAllocator,
    PREPROCESSED_TRACE_IDX,
};
use stwo_prover::core::backend::{Backend, BackendForChannel, Column};
use stwo_prover::core::channel::{Blake2sChannel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::{TreeSubspan, TreeVec};
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;
use stwo_prover::core::ColumnVec;

use crate::adapter::ProverInput;
use crate::components::{Claim, Components, InteractionClaim, Relations};
use crate::preprocessed::PreProcessedTraceBuilder;

pub fn assert_constraints(input: &mut ProverInput) {
    let mut commitment_scheme = MockCommitmentScheme::default();

    // Preprocessed trace.
    let preprocessed_trace = PreProcessedTraceBuilder::default().build();
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(preprocessed_trace.gen_trace());
    tree_builder.finalize_interaction();

    // Base trace.
    let (claim, trace, lookup_data) = Claim::write_trace::<Blake2sMerkleChannel>(input);
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(trace);
    tree_builder.finalize_interaction();

    // Interaction trace.

    let mut dummy_channel = Blake2sChannel::default();
    let relations = Relations::draw(&mut dummy_channel);
    let mut tree_builder = commitment_scheme.tree_builder();
    let (interaction_trace, interaction_claim) =
        InteractionClaim::write_interaction_trace(&relations, &lookup_data);
    tree_builder.extend_evals(interaction_trace);
    tree_builder.finalize_interaction();

    let mut tree_span_provider =
        TraceLocationAllocator::new_with_preproccessed_columns(&preprocessed_trace.ids());

    let components = Components::new(
        &mut tree_span_provider,
        &claim,
        &interaction_claim,
        &relations,
    );

    assert_components(commitment_scheme.trace_domain_evaluations(), &components);
}

#[derive(Default)]
pub struct MockCommitmentScheme {
    trees: TreeVec<ColumnVec<Vec<M31>>>,
}

impl MockCommitmentScheme {
    pub fn tree_builder(&mut self) -> MockTreeBuilder<'_> {
        MockTreeBuilder {
            tree_index: self.trees.len(),
            commitment_scheme: self,
            evals: Vec::default(),
        }
    }

    pub fn next_interaction(&mut self, evals: ColumnVec<Vec<M31>>) {
        self.trees.push(evals);
    }

    /// Returns the trace domain evaluations.
    /// Used for testing purposes.
    pub fn trace_domain_evaluations(&self) -> TreeVec<ColumnVec<&Vec<M31>>> {
        self.trees.as_cols_ref()
    }
}

/// A [`TreeBuilder`] used by [`MockCommitmentScheme`] to aggregate trace values.
/// This implementation avoids low degree extension and Merkle commitments.
pub struct MockTreeBuilder<'a> {
    tree_index: usize,
    evals: ColumnVec<Vec<M31>>,
    commitment_scheme: &'a mut MockCommitmentScheme,
}
impl MockTreeBuilder<'_> {
    pub fn extend_evals<B: Backend>(
        &mut self,
        columns: impl IntoIterator<Item = CircleEvaluation<B, M31, BitReversedOrder>>,
    ) {
        self.evals
            .extend(columns.into_iter().map(|s| s.to_cpu()).collect_vec());
    }

    pub fn finalize_interaction(self) {
        self.commitment_scheme.next_interaction(self.evals);
    }
}

impl<B: Backend> TreeBuilder<B> for MockTreeBuilder<'_> {
    fn extend_evals(
        &mut self,
        columns: impl IntoIterator<Item = CircleEvaluation<B, M31, BitReversedOrder>>,
    ) -> TreeSubspan {
        let col_start = self.evals.len();
        let tree_index = self.tree_index;
        self.extend_evals(columns);
        let col_end = self.evals.len();
        TreeSubspan {
            tree_index,
            col_start,
            col_end,
        }
    }
}

// Extenders of a commitment-tree with evaluations.
trait TreeBuilder<B: Backend> {
    fn extend_evals(
        &mut self,
        columns: impl IntoIterator<Item = CircleEvaluation<B, M31, BitReversedOrder>>,
    ) -> TreeSubspan;
}

impl<B: BackendForChannel<MC>, MC: MerkleChannel> TreeBuilder<B>
    for stwo_prover::core::pcs::TreeBuilder<'_, '_, B, MC>
{
    fn extend_evals(
        &mut self,
        columns: impl IntoIterator<Item = CircleEvaluation<B, M31, BitReversedOrder>>,
    ) -> TreeSubspan {
        self.extend_evals(columns)
    }
}

/// Asserts that constraints are correctly enforced.
/// When adding a component, this function should be updated.
fn assert_components(trace: TreeVec<Vec<&Vec<M31>>>, components: &Components) {
    let Components {
        opcodes,
        memory,
        merkle,
        poseidon2,
        range_check_8,
        range_check_16,
        range_check_20,
        clock_update,
    } = components;
    assert_component(&opcodes.call_abs_imm, &trace);
    assert_component(&opcodes.jmp_imm, &trace);
    assert_component(&opcodes.jnz_fp_imm, &trace);
    assert_component(&opcodes.ret, &trace);
    assert_component(&opcodes.store_fp_fp, &trace);
    assert_component(&opcodes.store_fp_imm, &trace);
    assert_component(&opcodes.store_double_deref_fp, &trace);
    assert_component(&opcodes.store_imm, &trace);
    assert_component(&opcodes.u32_store_imm, &trace);
    assert_component(memory, &trace);
    assert_component(merkle, &trace);
    assert_component(clock_update, &trace);
    assert_component(poseidon2, &trace);
    assert_component(range_check_8, &trace);
    assert_component(range_check_16, &trace);
    assert_component(range_check_20, &trace);
}

fn assert_component<E: FrameworkEval + Sync>(
    component: &FrameworkComponent<E>,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) {
    let mut component_trace = trace
        .sub_tree(component.trace_locations())
        .map(|tree| tree.into_iter().cloned().collect_vec());
    component_trace[PREPROCESSED_TRACE_IDX] = component
        .preproccessed_column_indices()
        .iter()
        .map(|idx| trace[PREPROCESSED_TRACE_IDX][*idx])
        .collect();

    let log_size = component.log_size();

    let component_eval = component.deref();
    assert_constraints_on_trace(
        &component_trace,
        log_size,
        |eval| {
            component_eval.evaluate(eval);
        },
        component.claimed_sum(),
    );
}
