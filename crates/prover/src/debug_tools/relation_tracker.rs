// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/relation_tracker.rs

use itertools::chain;
use num_traits::{One, Zero};
use stwo_constraint_framework::relation_tracker::{
    add_to_relation_entries, RelationSummary, RelationTrackerEntry,
};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Column};
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::pcs::{CommitmentSchemeProver, TreeVec};
use stwo_prover::core::poly::circle::CanonicCoset;

use crate::adapter::merkle::TREE_HEIGHT;
use crate::components::Components;
use crate::public_data::PublicData;

/// Show emitted but unconsumed OR consumed but non emitted relation entries.
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
    entries.push(RelationTrackerEntry {
        relation: "Merkle".to_string(),
        mult: M31::one(),
        values: [
            M31::zero(),
            M31::zero(),
            public_data.initial_root,
            public_data.initial_root,
        ]
        .to_vec(),
    });
    entries.push(RelationTrackerEntry {
        relation: "Merkle".to_string(),
        mult: M31::one(),
        values: [
            M31::zero(),
            M31::zero(),
            public_data.final_root,
            public_data.final_root,
        ]
        .to_vec(),
    });

    // Add memory relation entries for all public addresses (program, input, output)
    let mut add_memory_entries = |public_entries: &[Option<(M31, QM31, M31)>],
                                  multiplicity: M31| {
        let one = M31::one();
        let m31_2 = M31::from(2);
        let m31_3 = M31::from(3);
        let m31_4 = M31::from(4);
        let root = if multiplicity == M31::one() {
            public_data.initial_root
        } else {
            public_data.final_root
        };

        for (addr, value, clock) in public_entries.iter().flatten() {
            let value_array = value.to_m31_array();

            // Add memory relation entry
            entries.push(RelationTrackerEntry {
                relation: "Memory".to_string(),
                mult: multiplicity,
                values: [
                    *addr,
                    *clock,
                    value_array[0],
                    value_array[1],
                    value_array[2],
                    value_array[3],
                ]
                .to_vec(),
            });

            // Add 4 merkle relation entries for each QM31 component
            entries.push(RelationTrackerEntry {
                relation: "Merkle".to_string(),
                mult: -M31::one(),
                values: [m31_4 * *addr, M31::from(TREE_HEIGHT), value_array[0], root].to_vec(),
            });
            entries.push(RelationTrackerEntry {
                relation: "Merkle".to_string(),
                mult: -M31::one(),
                values: [
                    m31_4 * *addr + one,
                    M31::from(TREE_HEIGHT),
                    value_array[1],
                    root,
                ]
                .to_vec(),
            });
            entries.push(RelationTrackerEntry {
                relation: "Merkle".to_string(),
                mult: -M31::one(),
                values: [
                    m31_4 * *addr + m31_2,
                    M31::from(TREE_HEIGHT),
                    value_array[2],
                    root,
                ]
                .to_vec(),
            });
            entries.push(RelationTrackerEntry {
                relation: "Merkle".to_string(),
                mult: -M31::one(),
                values: [
                    m31_4 * *addr + m31_3,
                    M31::from(TREE_HEIGHT),
                    value_array[3],
                    root,
                ]
                .to_vec(),
            });
        }
    };

    // Emit the initial program and input values
    add_memory_entries(&public_data.public_memory.program, M31::one());
    add_memory_entries(&public_data.public_memory.input, M31::one());
    // Use the final output values
    add_memory_entries(&public_data.public_memory.output, -M31::one());
    entries
}

/// Goes through add_to_relation all and keeps count of each entry used/emitted.
/// Should be updated when components are modified.
fn relation_entries(
    components: &Components,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) -> Vec<RelationTrackerEntry> {
    let Components {
        memory,
        merkle,
        poseidon2,
        range_check_16,
        range_check_20,
        opcodes,
        clock_update,
    } = components;

    let entries: Vec<RelationTrackerEntry> = chain!(
        add_to_relation_entries(&opcodes.call_abs_imm, trace),
        add_to_relation_entries(&opcodes.jmp_imm, trace),
        add_to_relation_entries(&opcodes.jnz_fp_imm, trace),
        add_to_relation_entries(&opcodes.ret, trace),
        add_to_relation_entries(&opcodes.store_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_double_deref_fp, trace),
        add_to_relation_entries(&opcodes.store_imm, trace),
        add_to_relation_entries(memory, trace),
        add_to_relation_entries(merkle, trace),
        add_to_relation_entries(clock_update, trace),
        add_to_relation_entries(poseidon2, trace),
        add_to_relation_entries(range_check_16, trace),
        add_to_relation_entries(range_check_20, trace),
    )
    .collect();

    entries
}
