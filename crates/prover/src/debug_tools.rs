// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/relation_tracker.rs

use itertools::chain;
use stwo_prover::constraint_framework::relation_tracker::{
    add_to_relation_entries, RelationSummary, RelationTrackerEntry,
};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Column};
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::{CommitmentSchemeProver, TreeVec};
use stwo_prover::core::poly::circle::CanonicCoset;

use crate::components::Components;

pub fn track_and_summarize_cairo_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
) -> RelationSummary
where
    SimdBackend: BackendForChannel<MC>,
{
    let entries = track_cairo_relations(commitment_scheme, components);
    RelationSummary::summarize_relations(&entries).cleaned()
}

pub fn track_cairo_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
) -> Vec<RelationTrackerEntry>
where
    SimdBackend: BackendForChannel<MC>,
{
    let evals = commitment_scheme.trace().polys.map(|interaction_tree| {
        interaction_tree
            .iter()
            .map(|poly| {
                poly.evaluate(CanonicCoset::new(poly.log_size()).circle_domain())
                    .values
                    .to_cpu()
            })
            .collect()
    });
    let evals = &evals.as_ref();
    let trace = &evals.into();

    cairo_relation_entries(components, trace)
}

fn cairo_relation_entries(
    components: &Components,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) -> Vec<RelationTrackerEntry> {
    let Components {
        memory,
        range_check_20,
        store_deref_fp,
    } = components;

    let entries: Vec<RelationTrackerEntry> = chain!(
        add_to_relation_entries(memory, trace),
        add_to_relation_entries(range_check_20, trace),
        add_to_relation_entries(store_deref_fp, trace),
    )
    .collect();

    entries
}
