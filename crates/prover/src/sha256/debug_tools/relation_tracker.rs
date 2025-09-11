// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/relation_tracker.rs

use itertools::chain;
use stwo_constraint_framework::relation_tracker::{
    add_to_relation_entries, RelationSummary, RelationTrackerEntry,
};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Column};
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::{CommitmentSchemeProver, TreeVec};
use stwo_prover::core::poly::circle::CanonicCoset;

use crate::sha256::Components;

/// Show emitted but unconsumed OR consumed but non emitted relation entries.
pub fn track_and_summarize_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
) -> RelationSummary
where
    SimdBackend: BackendForChannel<MC>,
{
    let entries = track_relations(commitment_scheme, components);
    RelationSummary::summarize_relations(&entries).cleaned()
}

/// Tracks lookup emissions/consumptions
///
/// Goes through each add_to_relation in each component and for each entry it counts how much time it is emitted/used:
/// - adds `numerator` times for emissions
/// - subtracts `numerator` times for uses
///
/// Most of the logic in the track_relations function reproduces the PublicData::initial_logup_sum logic.
/// Must be updated when components or public data are modified.
fn track_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
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

    relation_entries(components, trace)
}

/// Goes through add_to_relation all and keeps count of each entry used/emitted.
/// Should be updated when components are modified.
fn relation_entries(
    components: &Components,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) -> Vec<RelationTrackerEntry> {
    let Components {
        sha256,
        ch_l0,
        ch_l1,
        ch_l2,
        ch_h0,
        ch_h1,
        ch_h2,
        maj_l0,
        maj_l1,
        maj_l2,
        maj_h0,
        maj_h1,
        maj_h2,
        small_sigma0_0,
        small_sigma0_1,
        small_sigma1_0,
        small_sigma1_1,
        big_sigma0_0,
        big_sigma0_1,
        big_sigma1_0,
        big_sigma1_1,
        range_check_16,
    } = components;

    let entries: Vec<RelationTrackerEntry> = chain!(
        add_to_relation_entries(sha256, trace),
        add_to_relation_entries(ch_l0, trace),
        add_to_relation_entries(ch_l1, trace),
        add_to_relation_entries(ch_l2, trace),
        add_to_relation_entries(ch_h0, trace),
        add_to_relation_entries(ch_h1, trace),
        add_to_relation_entries(ch_h2, trace),
        add_to_relation_entries(maj_l0, trace),
        add_to_relation_entries(maj_l1, trace),
        add_to_relation_entries(maj_l2, trace),
        add_to_relation_entries(maj_h0, trace),
        add_to_relation_entries(maj_h1, trace),
        add_to_relation_entries(maj_h2, trace),
        add_to_relation_entries(small_sigma0_0, trace),
        add_to_relation_entries(small_sigma0_1, trace),
        add_to_relation_entries(small_sigma1_0, trace),
        add_to_relation_entries(small_sigma1_1, trace),
        add_to_relation_entries(big_sigma0_0, trace),
        add_to_relation_entries(big_sigma0_1, trace),
        add_to_relation_entries(big_sigma1_0, trace),
        add_to_relation_entries(big_sigma1_1, trace),
        add_to_relation_entries(range_check_16, trace),
    )
    .collect();

    entries
}
