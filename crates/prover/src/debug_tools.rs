// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/relation_tracker.rs

use std::ops::Deref;

use itertools::{chain, Itertools};
use num_traits::One;
use stwo_prover::constraint_framework::relation_tracker::{
    add_to_relation_entries, RelationSummary, RelationTrackerEntry,
};
use stwo_prover::constraint_framework::{
    assert_constraints_on_trace, FrameworkComponent, FrameworkEval, PREPROCESSED_TRACE_IDX,
};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{Backend, BackendForChannel, Column};
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::{CommitmentSchemeProver, TreeSubspan, TreeVec};
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::ColumnVec;

use crate::components::Components;
use crate::public_data::PublicData;

pub fn track_and_summarize_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
    public_data: &PublicData,
) -> RelationSummary
where
    SimdBackend: BackendForChannel<MC>,
{
    let entries = track_relations(commitment_scheme, components, public_data);
    RelationSummary::summarize_relations(&entries).cleaned()
}

fn track_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
    public_data: &PublicData,
) -> Vec<RelationTrackerEntry>
where
    SimdBackend: BackendForChannel<MC>,
{
    let evals = commitment_scheme.trace().polys.map(|tree| {
        tree.iter()
            .map(|poly| {
                poly.evaluate(CanonicCoset::new(poly.log_size()).circle_domain())
                    .values
                    .to_cpu()
            })
            .collect()
    });
    let evals = &evals.as_ref();
    let trace = &evals.into();

    let mut entries = relation_entries(components, trace);

    let initial_registers = public_data.initial_registers;
    let final_registers = public_data.final_registers;

    entries.push(RelationTrackerEntry {
        relation: "Registers".to_string(),
        mult: M31::one(),
        values: [initial_registers.pc, initial_registers.fp].to_vec(),
    });
    entries.push(RelationTrackerEntry {
        relation: "Registers".to_string(),
        mult: -M31::one(),
        values: [final_registers.pc, final_registers.fp].to_vec(),
    });

    entries
}

fn relation_entries(
    components: &Components,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) -> Vec<RelationTrackerEntry> {
    let Components {
        memory,
        range_check_20,
        opcodes,
    } = components;

    let entries: Vec<RelationTrackerEntry> = chain!(
        add_to_relation_entries(&opcodes.call_abs_fp, trace),
        add_to_relation_entries(&opcodes.call_abs_imm, trace),
        add_to_relation_entries(&opcodes.call_rel_fp, trace),
        add_to_relation_entries(&opcodes.call_rel_imm, trace),
        add_to_relation_entries(&opcodes.jmp_abs_add_fp_fp, trace),
        add_to_relation_entries(&opcodes.jmp_abs_add_fp_imm, trace),
        add_to_relation_entries(&opcodes.jmp_abs_deref_fp, trace),
        add_to_relation_entries(&opcodes.jmp_abs_double_deref_fp, trace),
        add_to_relation_entries(&opcodes.jmp_abs_imm, trace),
        add_to_relation_entries(&opcodes.jmp_abs_mul_fp_fp, trace),
        add_to_relation_entries(&opcodes.jmp_abs_mul_fp_imm, trace),
        add_to_relation_entries(&opcodes.jmp_rel_add_fp_fp, trace),
        add_to_relation_entries(&opcodes.jmp_rel_add_fp_imm, trace),
        add_to_relation_entries(&opcodes.jmp_rel_deref_fp, trace),
        add_to_relation_entries(&opcodes.jmp_rel_double_deref_fp, trace),
        add_to_relation_entries(&opcodes.jmp_rel_imm, trace),
        add_to_relation_entries(&opcodes.jmp_rel_mul_fp_fp, trace),
        add_to_relation_entries(&opcodes.jmp_rel_mul_fp_imm, trace),
        add_to_relation_entries(&opcodes.jnz_fp_fp, trace),
        add_to_relation_entries(&opcodes.jnz_fp_fp_taken, trace),
        add_to_relation_entries(&opcodes.jnz_fp_imm, trace),
        add_to_relation_entries(&opcodes.jnz_fp_imm_taken, trace),
        add_to_relation_entries(&opcodes.ret, trace),
        add_to_relation_entries(&opcodes.store_add_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_add_fp_fp_inplace, trace),
        add_to_relation_entries(&opcodes.store_add_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_add_fp_imm_inplace, trace),
        add_to_relation_entries(&opcodes.store_deref_fp, trace),
        add_to_relation_entries(&opcodes.store_div_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_div_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_double_deref_fp, trace),
        add_to_relation_entries(&opcodes.store_imm, trace),
        add_to_relation_entries(&opcodes.store_mul_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_mul_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_sub_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_sub_fp_imm, trace),
        add_to_relation_entries(memory, trace),
        add_to_relation_entries(range_check_20, trace),
    )
    .collect();

    entries
}

#[derive(Default)]
pub struct MockCommitmentScheme {
    pub trees: TreeVec<ColumnVec<Vec<M31>>>,
}

impl MockCommitmentScheme {
    pub fn tree_builder(&mut self) -> MockTreeBuilder<'_> {
        MockTreeBuilder {
            tree_index: self.trees.len(),
            commitment_scheme: self,
            evals: Vec::default(),
        }
    }

    fn next_interaction(&mut self, evals: ColumnVec<Vec<M31>>) {
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
pub trait TreeBuilder<B: Backend> {
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

pub fn assert_components(trace: TreeVec<Vec<&Vec<M31>>>, components: &Components) {
    let Components {
        opcodes,
        memory,
        range_check_20,
    } = components;
    assert_component(&opcodes.call_abs_fp, &trace);
    assert_component(&opcodes.call_abs_imm, &trace);
    assert_component(&opcodes.call_rel_fp, &trace);
    assert_component(&opcodes.call_rel_imm, &trace);
    assert_component(&opcodes.jmp_abs_add_fp_fp, &trace);
    assert_component(&opcodes.jmp_abs_add_fp_imm, &trace);
    assert_component(&opcodes.jmp_abs_deref_fp, &trace);
    assert_component(&opcodes.jmp_abs_double_deref_fp, &trace);
    assert_component(&opcodes.jmp_abs_imm, &trace);
    assert_component(&opcodes.jmp_abs_mul_fp_fp, &trace);
    assert_component(&opcodes.jmp_abs_mul_fp_imm, &trace);
    assert_component(&opcodes.jmp_rel_add_fp_fp, &trace);
    assert_component(&opcodes.jmp_rel_add_fp_imm, &trace);
    assert_component(&opcodes.jmp_rel_deref_fp, &trace);
    assert_component(&opcodes.jmp_rel_double_deref_fp, &trace);
    assert_component(&opcodes.jmp_rel_imm, &trace);
    assert_component(&opcodes.jmp_rel_mul_fp_fp, &trace);
    assert_component(&opcodes.jmp_rel_mul_fp_imm, &trace);
    assert_component(&opcodes.jnz_fp_fp, &trace);
    assert_component(&opcodes.jnz_fp_fp_taken, &trace);
    assert_component(&opcodes.jnz_fp_imm, &trace);
    assert_component(&opcodes.jnz_fp_imm_taken, &trace);
    assert_component(&opcodes.ret, &trace);
    assert_component(&opcodes.store_add_fp_fp, &trace);
    assert_component(&opcodes.store_add_fp_fp_inplace, &trace);
    assert_component(&opcodes.store_add_fp_imm, &trace);
    assert_component(&opcodes.store_add_fp_imm_inplace, &trace);
    assert_component(&opcodes.store_deref_fp, &trace);
    assert_component(&opcodes.store_div_fp_fp, &trace);
    assert_component(&opcodes.store_div_fp_imm, &trace);
    assert_component(&opcodes.store_double_deref_fp, &trace);
    assert_component(&opcodes.store_imm, &trace);
    assert_component(&opcodes.store_mul_fp_fp, &trace);
    assert_component(&opcodes.store_mul_fp_imm, &trace);
    assert_component(&opcodes.store_sub_fp_fp, &trace);
    assert_component(&opcodes.store_sub_fp_imm, &trace);
    assert_component(memory, &trace);
    assert_component(range_check_20, &trace);
}

pub fn assert_component<E: FrameworkEval + Sync>(
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
